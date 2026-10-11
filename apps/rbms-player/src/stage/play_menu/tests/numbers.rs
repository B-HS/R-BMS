//! The rows of the play menu that hold a number: a key moves one by the smallest change it can
//! hold, its slider is dragged, and a figure is typed into it.
//!
//! The rows of the settings table are stepped in a menu with no skin, which is all they need; the
//! preview that follows a cover being dragged is the shipped pack's.

use std::time::Instant;

use rbms_config::{HISPEED_MAX, JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS};

use super::*;
use crate::number_row::{EDIT_HINT, NumberValue, REFUSAL_SHOWN_FOR, knob_rect, steps_at};
use crate::pointer::{PointerInput, route_pointer};

/// Every row of the settings table the menu lists that holds a number, with the smallest change it
/// can hold and its two ends, as the list writes them.
const NUMBER_ROWS: [(SettingId, &str, &str, &str); 8] = [
    (SettingId::GreenNumber, "1", "1", "6000"),
    (SettingId::HiSpeed, "0.01x", "0.01x", "20.00x"),
    (SettingId::HiSpeedStep, "0.01x", "0.01x", "1.00x"),
    (SettingId::LaneCover, "0.1%", "0.0%", "100.0%"),
    (SettingId::Lift, "0.1%", "0.0%", "100.0%"),
    (SettingId::Hidden, "0.1%", "0.0%", "100.0%"),
    (SettingId::LaneCoverFineStep, "0.0001", "0.0001", "0.0100"),
    (SettingId::JudgeOffset, "+1 MS", "-200 MS", "+200 MS"),
];

/// How far from an end of its range a row is put before it is stepped onto the end: nearer than a
/// wide step.
const NEAR_THE_END: i64 = 3;

/// How far past an end of a slider, and how far off its row, the pointer is taken while it drags.
const PAST_THE_SLIDER: f32 = 300.0;

/// Where across the preview, as shares of its width, and how far down it, as a share of its height,
/// the lanes of the shipped pack's seven-key play skin are sampled: between its lane cover's upper
/// edge and its judge line.
const LANES_ACROSS: [f32; 6] = [0.06, 0.10, 0.14, 0.18, 0.22, 0.26];
const LANES_DOWN: f32 = 0.45;

impl Menu {
    /// One key pressed that types `text`, on the key it is typed with.
    fn key_typing(&mut self, code: KeyCode, text: &'static str) -> Transition {
        let input = KeyInput { code, pressed: true, released: false, text: Some(text) };
        self.state.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now: Instant::now(), dt: 0.0 }, input)
    }

    /// Put one row that holds a number on a step, as the menu itself does.
    fn put(&mut self, row: Row, steps: i64) {
        self.state.set_number(&mut FrameCtx { shared: &mut self.app.shared, now: Instant::now(), dt: 0.0 }, row, steps);
    }

    /// The number one row holds.
    fn number(&self, row: Row) -> NumberValue {
        self.state.number_of(&self.app.shared, row).unwrap_or_else(|| panic!("{row:?} holds no number"))
    }

    /// What the list writes for one row that holds a number.
    fn figure(&self, row: Row) -> String {
        number_figure(row, &self.number(row))
    }

    /// Whether the row is still saying that what was typed into it was not a number.
    fn is_refused(&self, row: Row) -> bool {
        self.state.numbers.is_refused(row, Instant::now())
    }

    /// Move the previewed scene on to where its field stands drawn in full, without drawing the
    /// frames on the way, and draw it.
    fn run_in(&mut self) {
        for _ in 0..(RUN_SECONDS / CAPTURE_STEP_SECONDS) as usize {
            self.state.update(&mut FrameCtx { shared: &mut self.app.shared, now: Instant::now(), dt: CAPTURE_STEP_SECONDS });
        }
        self.until_said();
    }

    /// The colours of the previewed lanes, sampled across them on the frame drawn last.
    fn lanes(&self) -> Vec<Color> {
        let down = PREVIEW_RECT.y + PREVIEW_RECT.h * LANES_DOWN;
        LANES_ACROSS.iter().map(|across| self.pixels.pixel_at((PREVIEW_RECT.x + PREVIEW_RECT.w * across) as u32, down as u32)).collect()
    }
}

/// The pointer's place over a slider at `fraction` of the way along it: the middle of the knob.
fn along(slider: Rect, fraction: f32) -> (f32, f32) {
    middle(knob_rect(slider, fraction))
}

/// An arrow moves every row that holds a number by the smallest change it can hold, and ten of
/// them with Shift held; the row stops at both ends of its range, and the zero key puts it back on
/// what it ships as.
#[test]
fn a_key_moves_every_number_row_by_its_smallest_change_and_ten_with_shift() {
    let mut menu = Menu::over(bare_app("number-keys"));
    menu.app.shared.config.play.keep_green_number = true;
    for (id, smallest, low, high) in NUMBER_ROWS {
        if id == SettingId::HiSpeed {
            menu.app.shared.config.play.keep_green_number = false;
        }
        let row = Row::Setting(id);
        let value = menu.number(row);
        assert!(!value.locked, "{id:?} cannot be set");
        assert_eq!(number_figure(row, &NumberValue { steps: 1, ..value }), smallest, "{id:?}");
        assert_eq!(number_figure(row, &NumberValue { steps: value.spec.min, ..value }), low, "{id:?}");
        assert_eq!(number_figure(row, &NumberValue { steps: value.spec.max, ..value }), high, "{id:?}");

        let middle = (value.spec.min + value.spec.max) / 2;
        menu.put(row, middle);
        menu.focus(row);
        menu.key(KeyCode::ArrowRight);
        assert_eq!(menu.steps_of(row), middle + 1, "{id:?}: one press is not the smallest change");
        menu.shifted(KeyCode::ArrowRight);
        assert_eq!(menu.steps_of(row), middle + 11, "{id:?}: Shift is not ten of them");
        menu.key(KeyCode::ArrowLeft);
        menu.shifted(KeyCode::ArrowLeft);
        assert_eq!(menu.steps_of(row), middle, "{id:?}: the way back is not the way there");

        menu.put(row, value.spec.max - NEAR_THE_END);
        menu.shifted(KeyCode::ArrowRight);
        assert_eq!((menu.steps_of(row), menu.figure(row).as_str()), (value.spec.max, high), "{id:?}: a wide step past the end did not stop at it");
        menu.key(KeyCode::ArrowRight);
        assert_eq!(menu.steps_of(row), value.spec.max, "{id:?} went past its upper end");
        menu.put(row, value.spec.min + NEAR_THE_END);
        menu.shifted(KeyCode::ArrowLeft);
        assert_eq!((menu.steps_of(row), menu.figure(row).as_str()), (value.spec.min, low), "{id:?}: a wide step past the end did not stop at it");
        menu.key(KeyCode::ArrowLeft);
        assert_eq!(menu.steps_of(row), value.spec.min, "{id:?} went past its lower end");

        menu.key(if id == SettingId::Lift { KeyCode::Numpad0 } else { KeyCode::Digit0 });
        assert_eq!(menu.steps_of(row), value.shipped, "{id:?}: the zero key did not put the row back");
    }
    let listed: Vec<SettingId> = SPEED_ROWS.iter().chain(&LANE_ROWS).chain(&JUDGE_ROWS).chain(&CHART_ROWS).copied().filter(|id| holds_a_number(*id)).collect();
    assert_eq!(listed, NUMBER_ROWS.map(|(id, ..)| id), "a row that holds a number is not stepped here");
}

/// A press on a slider puts its row where the press landed, and the row then goes with the pointer
/// for as long as the button is held: along the slider, onto its ends when the pointer passes
/// them, and wherever on the screen the pointer goes. Once the button is let go of, the pointer
/// moves nothing.
#[test]
fn a_slider_carries_its_row_with_the_pointer_and_stops_at_the_ends() {
    let mut menu = Menu::over(bare_app("number-drag"));
    menu.frame();
    let row = Row::Setting(SettingId::LaneCover);
    let slider = menu.slider_of(row);
    let spec = menu.number(row).spec;

    menu.state.focus(0, ROW_STEP_FORWARD);
    assert!(matches!(menu.click(along(slider, 0.5), MouseButton::Left), Transition::Stay));
    assert_eq!(menu.steps_of(row), 500, "a press in the middle of the slider did not put the row in the middle of its range");
    assert_eq!(menu.state.focused(), Some(row), "the press did not move the cursor to the row");
    assert!(menu.state.numbers.is_dragging(row));

    let mut last = menu.steps_of(row);
    let mut x = along(slider, 0.5).0;
    while x < slider.x + slider.w + PAST_THE_SLIDER {
        menu.drag((x, along(slider, 0.5).1));
        let now = menu.steps_of(row);
        assert!(now >= last, "the row went back while the pointer went on");
        assert_eq!(now, steps_at(&spec, slider, x), "the row is not under the pointer at {x}");
        last = now;
        x += 1.0;
    }
    assert_eq!((last, menu.value(SettingId::LaneCover).as_str()), (1000, "100.0% (WHITE 1000)"), "the row did not stop at its upper end");
    menu.drag((slider.x - PAST_THE_SLIDER, slider.y));
    assert_eq!(menu.steps_of(row), 0, "the row did not stop at its lower end");

    menu.drag((along(slider, 0.25).0, slider.y - PAST_THE_SLIDER));
    assert_eq!(menu.steps_of(row), 250, "the drag ended when the pointer left the row");
    menu.drag(middle(PREVIEW_RECT));
    assert_eq!(menu.steps_of(row), 1000, "the drag ended when the pointer left the list");
    menu.drag(along(slider, 0.75));
    assert_eq!(menu.value(SettingId::LaneCover), "75.0% (WHITE 750)");

    menu.let_go(along(slider, 0.75));
    assert_eq!(menu.state.numbers.dragged(), None);
    menu.drag(along(slider, 0.1));
    assert_eq!(menu.steps_of(row), 750, "the pointer moved the row with no button held on its slider");
}

/// A drag writes the settings file once, when the button comes back up, and not while the slider
/// moves. A press that begins no drag writes nothing, and a drag whose button was never seen coming
/// back up is ended, and written out, by the next press.
#[test]
fn a_drag_writes_the_settings_file_once_when_it_ends() {
    let mut menu = Menu::over(bare_app("number-drag-save"));
    menu.frame();
    let settings = menu.app.shared.settings_path.clone();
    let row = Row::Setting(SettingId::LaneCover);
    let slider = menu.slider_of(row);
    assert!(!settings.exists(), "the settings file was written before anything moved");

    menu.click(along(slider, 0.2), MouseButton::Left);
    for fraction in [0.3, 0.4, 0.5, 0.6] {
        menu.drag(along(slider, fraction));
        assert!(!settings.exists(), "the settings file was written while the slider was being dragged");
    }
    menu.let_go(along(slider, 0.6));
    let written = rbms_config::load(&settings).expect("the file the drag wrote reads back").config;
    assert_eq!(rbms_config::number_steps(&written, SettingId::LaneCover), Some(600), "what was written is not where the drag ended");

    std::fs::remove_file(&settings).expect("the settings file can be removed");
    menu.let_go(along(slider, 0.6));
    menu.drag(along(slider, 0.9));
    menu.click(menu.on_the_label_of(row), MouseButton::Left);
    menu.let_go(menu.on_the_label_of(row));
    menu.click(menu.on_the_label_of(row), MouseButton::Right);
    assert!(!settings.exists(), "the settings file was written with no drag to end");

    menu.click(along(slider, 0.4), MouseButton::Left);
    assert!(!settings.exists());
    menu.click(menu.on_the_label_of(Row::Setting(SettingId::EnableLift)), MouseButton::Left);
    assert_eq!(menu.state.numbers.dragged(), None, "a drag outlived the press after it");
    assert!(settings.exists(), "a drag whose button never came back up was not written out");
}

/// The wheel scrolls the list over a slider as it does anywhere on the list, and never moves the
/// row under it.
#[test]
fn the_wheel_over_a_slider_scrolls_the_list_and_leaves_the_row_alone() {
    let mut menu = Menu::over(bare_app("number-wheel"));
    menu.frame();
    let before = menu.app.shared.config.clone();
    let on_slider = middle(menu.slider_of(Row::Setting(SettingId::HiSpeed)));
    menu.wheel(on_slider, 3.0);
    menu.wheel(on_slider, -1.0);
    assert_eq!(menu.state.top, 2, "the list did not scroll under the pointer");
    assert_eq!(menu.app.shared.config, before, "a turn of the wheel moved a row");
}

/// Enter opens a row that holds a number to be typed into, on the number it holds and with all of
/// it selected, and Enter stores what was typed: held to the row's range, rounded to the smallest
/// change the row can hold, read past a unit, and in the unit the row shows whatever it is stored
/// in. Nothing is stored before Enter.
#[test]
fn enter_opens_a_number_row_to_be_typed_into_and_stores_what_was_typed() {
    let mut menu = Menu::over(bare_app("number-type"));
    let offset = Row::Setting(SettingId::JudgeOffset);
    menu.focus(offset);
    assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay));
    let edit = menu.state.numbers.edit_of(offset).expect("Enter did not open the row");
    assert_eq!((edit.text(), edit.is_all_selected()), ("0", true));
    for text in ["-", "1", "2"] {
        menu.typed(text);
    }
    assert_eq!(menu.app.shared.config.judge.offset_ms, 0, "a figure was stored before it was finished");
    assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay));
    assert_eq!((menu.app.shared.config.judge.offset_ms, menu.state.numbers.editing()), (-12, None));

    let type_in = |menu: &mut Menu, row: Row, typed: &[&'static str]| {
        menu.focus(row);
        menu.key(KeyCode::Enter);
        for text in typed {
            menu.typed(text);
        }
        menu.key(KeyCode::NumpadEnter);
    };
    type_in(&mut menu, offset, &["7", " ", "m", "s"]);
    assert_eq!(menu.app.shared.config.judge.offset_ms, 7, "a unit typed after the number was not read past");
    type_in(&mut menu, offset, &["9", "9", "9"]);
    assert_eq!(menu.app.shared.config.judge.offset_ms, JUDGE_OFFSET_MAX_MS, "a figure past the upper end was not held to it");
    type_in(&mut menu, offset, &["-", "9", "9", "9"]);
    assert_eq!(menu.app.shared.config.judge.offset_ms, JUDGE_OFFSET_MIN_MS, "a figure past the lower end was not held to it");
    type_in(&mut menu, offset, &["3", ".", "6"]);
    assert_eq!(menu.app.shared.config.judge.offset_ms, 4, "a figure between two steps was not rounded to the nearer");

    let cover = Row::Setting(SettingId::LaneCover);
    type_in(&mut menu, cover, &["4", "0"]);
    assert!((menu.app.shared.config.play.cover - 0.4).abs() < f32::EPSILON, "a percent typed into a row stored as a share was not read as a percent");
    type_in(&mut menu, cover, &["2", "5", ".", "3", "4", "%"]);
    assert_eq!(menu.value(SettingId::LaneCover), "25.3% (WHITE 253)");
    type_in(&mut menu, cover, &["2", "5", "0"]);
    assert_eq!(menu.value(SettingId::LaneCover), "100.0% (WHITE 1000)");

    let speed = Row::Setting(SettingId::HiSpeed);
    type_in(&mut menu, speed, &["1", ".", "5", "x"]);
    assert_eq!(menu.value(SettingId::HiSpeed), "1.50x");
    type_in(&mut menu, speed, &[".", "0", "0", "4"]);
    assert_eq!(menu.value(SettingId::HiSpeed), "0.01x", "a figure under the slowest speed was not held to it");
    type_in(&mut menu, speed, &["9", "9"]);
    assert_eq!(menu.value(SettingId::HiSpeed), format!("{HISPEED_MAX:.2}x"));
}

/// What is typed and is not a number is not stored: the row keeps what it held, says so for a
/// while, and can be typed into again at once.
#[test]
fn what_is_typed_and_is_not_a_number_is_refused_and_said_on_the_row() {
    let mut menu = Menu::over(bare_app("number-refuse"));
    let offset = Row::Setting(SettingId::JudgeOffset);
    menu.app.shared.config.judge.offset_ms = 5;
    let before = menu.app.shared.config.clone();
    for typed in [&["-"][..], &["."], &["1", ".", "2", ".", "3"], &["5", "-"], &[]] {
        menu.focus(offset);
        menu.key(KeyCode::Enter);
        menu.key(KeyCode::Backspace);
        for text in typed {
            menu.typed(text);
        }
        assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay));
        assert_eq!(menu.app.shared.config, before, "{typed:?} was stored");
        assert_eq!(menu.state.numbers.editing(), None, "{typed:?} left the row open");
        assert!(menu.is_refused(offset), "the row does not say that {typed:?} is not a number");
        assert!(!menu.is_refused(Row::Setting(SettingId::HiSpeed)), "another row says so");
        assert!(!menu.state.numbers.is_refused(offset, Instant::now() + REFUSAL_SHOWN_FOR), "the row goes on saying so");
    }
    menu.key(KeyCode::Enter);
    assert!(!menu.is_refused(offset), "a row opened again still says what was typed last was refused");
    menu.typed("8");
    menu.key(KeyCode::Enter);
    assert_eq!(menu.app.shared.config.judge.offset_ms, 8);
}

/// Escape while a row is being typed into drops what was typed and stays on the menu; the Escape
/// after it leaves. While a row is open no key steps it, resets it or moves another row.
#[test]
fn escape_while_typing_drops_what_was_typed_and_does_not_leave_the_menu() {
    let mut menu = Menu::over(bare_app("number-escape"));
    let settings = menu.app.shared.settings_path.clone();
    let offset = Row::Setting(SettingId::JudgeOffset);
    menu.app.shared.config.judge.offset_ms = 5;
    let before = menu.app.shared.config.clone();
    menu.focus(offset);
    menu.key(KeyCode::Enter);
    menu.typed("4");
    menu.typed("2");
    for code in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Home, KeyCode::End, KeyCode::F8, KeyCode::KeyQ] {
        assert!(matches!(menu.key(code), Transition::Stay));
    }
    menu.key_typing(KeyCode::Digit0, "0");
    assert_eq!(menu.state.numbers.edit_of(offset).map(|edit| edit.text().to_string()), Some("420".to_string()), "a key did something other than type");
    assert_eq!(menu.app.shared.config, before, "a key reached the row while it was being typed into");

    assert!(matches!(menu.key(KeyCode::Escape), Transition::Stay), "Escape left the menu with a row open");
    assert_eq!((menu.state.numbers.editing(), menu.state.focused()), (None, Some(offset)));
    assert_eq!(menu.app.shared.config, before, "Escape stored what was typed");
    assert!(!menu.is_refused(offset) && !settings.exists(), "a figure that was dropped was treated as one that was stored");
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back), "the Escape after it did not leave");
}

/// A key that moves the cursor or the mode on stores what was typed and then does what it always
/// does, and so does a press of the mouse, wherever it lands.
#[test]
fn a_key_or_a_press_that_moves_on_stores_what_was_typed_first() {
    let mut menu = Menu::over(bare_app("number-move-on"));
    let offset = Row::Setting(SettingId::JudgeOffset);
    let after_offset = Row::Setting(SettingId::AutoCal);
    let moves = [
        (KeyCode::ArrowDown, Some(after_offset), "1", 1),
        (KeyCode::ArrowUp, None, "2", 2),
        (KeyCode::PageUp, None, "3", 3),
        (KeyCode::PageDown, None, "4", 4),
    ];
    for (code, lands_on, typed, expected) in moves {
        menu.focus(offset);
        menu.key(KeyCode::Enter);
        menu.key_typing(KeyCode::Digit1, typed);
        assert!(matches!(menu.key(code), Transition::Stay));
        assert_eq!(menu.app.shared.config.judge.offset_ms, expected, "{code:?} dropped what was typed");
        assert_eq!(menu.state.numbers.editing(), None);
        assert_ne!(menu.state.focused(), Some(offset), "{code:?} did not move the cursor after storing");
        if let Some(row) = lands_on {
            assert_eq!(menu.state.focused(), Some(row));
        }
    }
    menu.focus(offset);
    menu.key(KeyCode::Enter);
    menu.typed("9");
    menu.key(KeyCode::Tab);
    assert_eq!((menu.app.shared.config.judge.offset_ms, menu.state.mode), (9, 1), "Tab did not store what was typed and then step the mode");

    menu.focus(offset);
    menu.frame();
    assert!(matches!(menu.click(menu.on_the_number_of(offset), MouseButton::Left), Transition::Stay));
    assert_eq!(menu.state.numbers.editing(), Some(offset), "a press on the number did not open the row");
    menu.typed("3");
    menu.typed("0");
    let keep = Row::Setting(SettingId::AutoCal);
    menu.click(menu.on_the_label_of(keep), MouseButton::Left);
    assert_eq!(menu.app.shared.config.judge.offset_ms, 30, "a press on another row dropped what was typed");
    assert!(menu.app.shared.config.judge.auto_offset, "the press did not then step the row it landed on");
    assert_eq!(menu.state.numbers.editing(), None);

    menu.click(menu.on_the_number_of(offset), MouseButton::Left);
    menu.typed("6");
    menu.click(middle(PREVIEW_RECT), MouseButton::Left);
    assert_eq!((menu.app.shared.config.judge.offset_ms, menu.state.numbers.editing()), (6, None), "a press on nothing did not store what was typed");
}

/// A row that holds no number is what it was: Enter and a press step it, and nothing opens it to be
/// typed into or takes hold of it.
#[test]
fn a_row_that_holds_no_number_is_stepped_by_enter_and_a_press_as_it_always_was() {
    let mut menu = Menu::over(bare_app("number-others"));
    let others: Vec<Row> = menu.state.rows.iter().copied().filter(|row| !row.is_heading() && menu.state.number_of(&menu.app.shared, *row).is_none()).collect();
    assert!(
        others.contains(&Row::PreviewBpm) && others.contains(&Row::Setting(SettingId::KeepGreenNumber)) && others.contains(&Row::Setting(SettingId::Gauge))
    );
    for row in others {
        menu.focus(row);
        let line = menu.line(row);
        menu.key(KeyCode::Enter);
        assert_eq!(menu.state.numbers.editing(), None, "{} was opened to be typed into", line.label);
        assert_ne!(menu.line(row).value, line.value, "Enter did not step {}", line.label);
        menu.key(KeyCode::ArrowLeft);
        assert_eq!(menu.line(row).value, line.value, "{}", line.label);

        menu.frame();
        let drawn = menu.rect_of_hot(Hot::PlayMenuRow(menu.at(row)));
        for at in [middle(slider_rect(drawn)), middle(number_value_rect(drawn))] {
            menu.click(at, MouseButton::Left);
            assert_eq!((menu.state.numbers.dragged(), menu.state.numbers.editing()), (None, None), "{}", line.label);
            assert_ne!(menu.line(row).value, line.value, "a press did not step {}", line.label);
            menu.click(at, MouseButton::Right);
            assert_eq!(menu.line(row).value, line.value, "{}", line.label);
        }
    }
}

/// What the list writes for a row that holds a number fits the room a number has beside its
/// slider, at both ends of every row, and the name of every row of the settings table fits what the
/// slider leaves. The line of keys written while a row is being typed into fits the screen.
#[test]
fn the_number_and_the_name_of_a_number_row_fit_beside_its_slider() {
    rbms_render::font::use_embedded_fonts_only();
    let menu = Menu::over(bare_app("number-fits"));
    for (id, _, low, high) in NUMBER_ROWS {
        for figure in [low, high] {
            let width = text_width(figure, VALUE_SCALE);
            assert!(width <= number_value_room(), "{id:?} reads {figure}, which is {width} wide in a room of {}", number_value_room());
        }
        let label = menu.line(Row::Setting(id)).label;
        let width = text_width(&label, LABEL_SCALE);
        assert!(width <= number_label_room(), "{label} is {width} wide and its slider leaves {}", number_label_room());
    }
    assert_eq!(fit_text(EDIT_HINT, HINT_SCALE, CW as f32 - MARGIN * 2.0), EDIT_HINT, "the line of keys of a row being typed into is cut short");
    let drawn = crate::stage::preview_menu::row_rect(0);
    let (slider, number) = (slider_rect(drawn), number_value_rect(drawn));
    assert!(slider.x + slider.w <= number.x, "the slider runs under the number");
    assert!(number.x + number.w <= drawn.x + drawn.w && slider.x >= drawn.x, "the slider or the number is off the row");
}

/// The preview follows a lane cover being dragged: the frame after the pointer moved is drawn with
/// the cover where the pointer put it, with the preview never left waiting for its document, and
/// the drag goes on while the pointer is over the preview itself.
#[test]
fn the_preview_follows_a_lane_cover_being_dragged() {
    let mut menu = Menu::over(app_with_shipped_pack("number-preview"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let build = menu.app.shared.skins.build_of(screen);
    let row = Row::Setting(SettingId::LaneCover);
    let slider = menu.slider_of(row);
    let one_colour = |lanes: &[Color]| lanes.iter().all(|colour| *colour == lanes[0]);
    menu.run_in();
    let open = menu.lanes();
    assert!(!one_colour(&open), "the lanes of the preview are sampled where nothing tells them apart: {open:?}");

    menu.click(along(slider, 0.0), MouseButton::Left);
    menu.drag(along(slider, 1.0));
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a cover being dragged cost a frame of the preview");
    let covered = menu.lanes();
    assert!(one_colour(&covered), "the lanes are not under the cover on the frame after it was dragged over them: {covered:?}");

    menu.drag(along(slider, 0.3));
    assert_eq!(menu.steps_of(row), 300);
    let over_the_preview = middle(PREVIEW_RECT);
    let routed =
        route_pointer(&mut menu.state, &mut FrameCtx { shared: &mut menu.app.shared, now: Instant::now(), dt: 0.0 }, over_the_preview, PointerInput::Drag);
    assert!(matches!(routed, Transition::Stay));
    assert_eq!(menu.steps_of(row), 1000, "the drag did not go on with the pointer over the preview, which is past the slider's end");
    menu.drag(along(slider, 0.0));
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn);
    assert!(!one_colour(&menu.lanes()), "the lanes did not come back from under the cover on the frame after it was dragged away");
    menu.let_go(along(slider, 0.0));
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "a cover being dragged had the document read again");
}

/// The rows that hold a number drawn for a person to look at, at the size the menu is laid out for
/// and in a larger window: a cover before and while its slider is dragged, the judge offset being
/// typed into, a figure that was refused, and the scroll speed while a held green number works it
/// out.
#[test]
fn the_number_rows_of_the_play_menu_can_be_captured() {
    for size in [UI, LARGE] {
        let mut menu = Menu::sized(app_with_shipped_pack(&format!("number-capture-{}", size.0)), size);
        assert_eq!(menu.until_said(), PreviewStatus::Drawn);
        let cover = Row::Setting(SettingId::LaneCover);
        menu.focus(cover);
        menu.capture_at("play-menu-number-cover-before", RUN_SECONDS);
        let slider = menu.slider_of(cover);
        menu.click(along(slider, 0.1), MouseButton::Left);
        menu.drag(along(slider, 0.45));
        menu.frame();
        assert!(menu.state.numbers.is_dragging(cover));
        menu.save("play-menu-number-cover-dragging");
        menu.drag(along(slider, 1.0));
        menu.frame();
        menu.save("play-menu-number-cover-dragged-to-the-end");
        menu.drag(along(slider, 0.45));
        menu.let_go(along(slider, 0.45));

        let offset = Row::Setting(SettingId::JudgeOffset);
        menu.focus(offset);
        menu.key(KeyCode::Enter);
        menu.frame();
        menu.save("play-menu-number-offset-selected");
        for text in ["-", "1", "2"] {
            menu.typed(text);
        }
        menu.frame();
        menu.save("play-menu-number-offset-typing");
        menu.key(KeyCode::Enter);
        menu.key(KeyCode::Enter);
        menu.key(KeyCode::Backspace);
        menu.typed("-");
        menu.key(KeyCode::Enter);
        menu.frame();
        assert!(menu.is_refused(offset));
        menu.save("play-menu-number-offset-refused");

        menu.focus_setting(SettingId::KeepGreenNumber);
        menu.key(KeyCode::ArrowRight);
        menu.focus_setting(SettingId::HiSpeed);
        menu.frame();
        assert!(menu.number(Row::Setting(SettingId::HiSpeed)).locked);
        menu.save("play-menu-number-read-only-hi-speed");
    }
}

/// How far inside an edge of a slider's knob a press that takes hold of the knob by that edge
/// lands, and how far the pointer is then taken along the slider.
const INSIDE_THE_KNOB: f32 = 0.5;
const PULLED: f32 = 20.0;

/// The three places across a knob it is taken hold of: just inside its left edge, its middle, and
/// just inside its right edge.
fn grips(knob: Rect) -> [f32; 3] {
    [knob.x + INSIDE_THE_KNOB, knob.x + knob.w * 0.5, knob.x + knob.w - INSIDE_THE_KNOB]
}

/// A press on the knob of a slider takes hold of the knob where the press landed: the row is where
/// it was until the pointer moves, wherever across the knob it was taken hold of, and then moves by
/// as far as the pointer does.
#[test]
fn a_press_on_the_knob_takes_hold_of_it_and_leaves_the_row_where_it_was() {
    let mut menu = Menu::over(bare_app("number-grip"));
    for (id, steps) in [(SettingId::LaneCover, 334), (SettingId::Lift, 1000), (SettingId::Hidden, 0), (SettingId::JudgeOffset, -37)] {
        let row = Row::Setting(id);
        menu.put(row, steps);
        menu.focus(row);
        menu.frame();
        let slider = menu.slider_of(row);
        let value = menu.number(row);
        let knob = knob_rect(slider, value.fraction());
        let down = knob.y + knob.h * 0.5;
        for across in grips(knob) {
            menu.click((across, down), MouseButton::Left);
            assert!(menu.state.numbers.is_dragging(row), "{id:?}: a press on the knob did not take hold of it");
            assert_eq!(menu.steps_of(row), steps, "{id:?}: a press on the knob at {across} moved the row, the knob being at {}", knob.x);
            menu.drag((across, down - PAST_THE_SLIDER));
            assert_eq!(menu.steps_of(row), steps, "{id:?}: the pointer going up the screen moved the row");
            menu.let_go((across, down));
            assert_eq!(menu.steps_of(row), steps, "{id:?}: letting go of the knob moved the row");
        }
    }

    let row = Row::Setting(SettingId::LaneCover);
    menu.put(row, 334);
    menu.focus(row);
    menu.frame();
    let slider = menu.slider_of(row);
    let value = menu.number(row);
    let knob = knob_rect(slider, value.fraction());
    let (middle_of_knob, down) = (knob.x + knob.w * 0.5, knob.y + knob.h * 0.5);
    for across in grips(knob) {
        menu.put(row, 334);
        menu.click((across, down), MouseButton::Left);
        menu.drag((across + PULLED, down));
        assert_eq!(menu.steps_of(row), steps_at(&value.spec, slider, middle_of_knob + PULLED), "the row did not move by as far as the pointer did");
        menu.drag((across, down));
        assert_eq!(menu.steps_of(row), 334, "the pointer back where it took hold did not bring the row back");
        menu.let_go((across, down));
    }
}
