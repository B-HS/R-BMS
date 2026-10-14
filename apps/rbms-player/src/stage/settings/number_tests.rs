//! The rows of the settings screen that hold a number: a key moves one by the smallest change it
//! can hold, its slider is dragged, and a figure is typed into it.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, the AUDIO group is also saved there as PNGs a person
//! can open: its volumes with their sliders, one being dragged, and one being typed into.

use std::time::Instant;

use rbms_config::{AUDIO_POLYPHONY_MAX_VOICES, AUDIO_POLYPHONY_MIN_VOICES, number_spec, number_steps};
use rbms_render::Rect;

use super::*;
use crate::number_row::{REFUSAL_SHOWN_FOR, knob_rect};
use crate::settings_view::{row_number, row_slider};
use crate::skin_select::SkinLibrary;
use crate::stage::render_tests::app;
use crate::stage::{Canvas, HeadlessCanvas};

/// Environment variable naming the folder captures are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The two sizes the screen is drawn at: the one it is laid out for, and a full HD window.
pub(super) const DRAWN_SIZES: [(u32, u32); 2] = [(CW, CH), (1920, 1080)];

/// Every row of the screen that holds a number, by the tab it is on.
const NUMBER_ROWS: [(SettingTab, &[SettingId]); 4] = [
    (
        SettingTab::Judge,
        &[
            SettingId::JudgeWidthKeyPGreat,
            SettingId::JudgeWidthKeyGreat,
            SettingId::JudgeWidthKeyGood,
            SettingId::JudgeWidthScratchPGreat,
            SettingId::JudgeWidthScratchGreat,
            SettingId::JudgeWidthScratchGood,
            SettingId::LongNoteMargin,
            SettingId::Total,
        ],
    ),
    (SettingTab::Display, &[SettingId::JudgeTextY]),
    (SettingTab::Audio, &[SettingId::MasterVolume, SettingId::KeyVolume, SettingId::BgmVolume, SettingId::SystemVolume, SettingId::AudioPolyphony]),
    (SettingTab::Select, &[SettingId::PreviewVolume, SettingId::PreviewFade]),
];

/// How far from an end of its range a row is put before it is stepped onto the end: nearer than a
/// wide step.
const NEAR_THE_END: i64 = 3;

/// How far past an end of a slider, and how far off its row, the pointer is taken while it drags.
const PAST_THE_SLIDER: f32 = 300.0;

/// How far inside a row's left edge a press is made on its label.
const INSIDE_THE_ROW: f32 = 8.0;

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }
}

/// An app whose settings live in a folder of this test's own, with no skin chosen.
pub(super) fn bare_app(tag: &str) -> App {
    let mut app = app();
    let root = std::env::temp_dir().join(format!("rbms-settings-numbers-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the fixture folder is writable");
    app.shared.settings_path = root.join("settings.ron");
    app.shared.config.skin = rbms_config::SkinOptions::default();
    app.shared.skins = SkinLibrary::new(&app.shared.settings_path, &app.shared.config);
    app
}

/// The screen on one tab over an app, with the canvas its frames are drawn on.
pub(super) struct Screen {
    pub(super) app: App,
    pub(super) state: SettingsState,
    pub(super) pixels: HeadlessCanvas,
    pub(super) size: (u32, u32),
}

impl Screen {
    pub(super) fn on(tag: &str, tab: SettingTab) -> Screen {
        Screen::sized(tag, tab, DRAWN_SIZES[0])
    }

    pub(super) fn sized(tag: &str, tab: SettingTab, size: (u32, u32)) -> Screen {
        let mut app = bare_app(tag);
        let mut state = SettingsState::on_tab(tab);
        state.on_enter(&mut ctx(&mut app));
        let mut screen = Screen { app, state, pixels: HeadlessCanvas::new(size.0, size.1), size };
        screen.frame();
        screen
    }

    /// One frame: the regions of the last are forgotten and the screen is drawn.
    pub(super) fn frame(&mut self) {
        self.app.shared.hot.clear();
        self.state.draw(&mut ctx(&mut self.app), &mut Canvas::Headless(&mut self.pixels));
    }

    pub(super) fn at(&self, id: SettingId) -> usize {
        self.state.rows.iter().position(|row| *row == SettingRow::Fixed(id)).unwrap_or_else(|| panic!("{id:?} is not on the open tab"))
    }

    /// Put the cursor on one row and draw it there, so the row is on show.
    pub(super) fn focus(&mut self, id: SettingId) {
        self.state.sel = self.at(id);
        self.frame();
    }

    pub(super) fn key(&mut self, code: KeyCode) -> Transition {
        self.state.handle_key(&mut ctx(&mut self.app), KeyInput { code, pressed: true, released: false, text: None })
    }

    /// A key pressed with Shift held, and Shift let go of after it.
    pub(super) fn shifted(&mut self, code: KeyCode) {
        self.key(KeyCode::ShiftLeft);
        self.key(code);
        self.state.handle_key(&mut ctx(&mut self.app), KeyInput { code: KeyCode::ShiftLeft, pressed: false, released: true, text: None });
    }

    /// One key pressed that types `text`.
    pub(super) fn typed(&mut self, text: &'static str) -> Transition {
        self.state.handle_key(&mut ctx(&mut self.app), KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) })
    }

    /// Open the row to be typed into with Enter, type `typed` into it key by key, and press Enter.
    pub(super) fn type_in(&mut self, id: SettingId, typed: &[&'static str]) {
        self.focus(id);
        self.key(KeyCode::Enter);
        for text in typed {
            self.typed(text);
        }
        self.key(KeyCode::Enter);
    }

    pub(super) fn press(&mut self, at: (f32, f32), button: MouseButton) -> Transition {
        self.state.handle_mouse(&mut ctx(&mut self.app), at, button, true)
    }

    pub(super) fn drag(&mut self, at: (f32, f32)) {
        self.state.handle_mouse_drag(&mut ctx(&mut self.app), at);
    }

    pub(super) fn let_go(&mut self, at: (f32, f32)) {
        self.state.handle_mouse(&mut ctx(&mut self.app), at, MouseButton::Left, false);
    }

    /// Where the frame drawn last drew one row.
    pub(super) fn row_of(&self, id: SettingId) -> Rect {
        crate::number_row::hot_rect(&self.app.shared, Hot::SettingRow(self.at(id))).unwrap_or_else(|| panic!("{id:?} is not on show"))
    }

    pub(super) fn slider_of(&self, id: SettingId) -> Rect {
        row_slider(self.row_of(id))
    }

    /// A point on the label of one row: on the row, and on neither its slider nor its number.
    pub(super) fn on_the_label_of(&self, id: SettingId) -> (f32, f32) {
        let row = self.row_of(id);
        (row.x + INSIDE_THE_ROW, row.y + row.h * 0.5)
    }

    pub(super) fn on_the_number_of(&self, id: SettingId) -> (f32, f32) {
        middle(row_number(self.row_of(id)))
    }

    /// The step one row that holds a number stands on in the configuration.
    pub(super) fn steps_of(&self, id: SettingId) -> i64 {
        number_steps(&self.app.shared.config, id).unwrap_or_else(|| panic!("{id:?} holds no number"))
    }

    /// What the row reads as the screen draws it.
    pub(super) fn reads(&mut self, id: SettingId) -> String {
        self.state.ensure_rows(&self.app.shared);
        self.state.lines[self.at(id)].value.clone()
    }

    pub(super) fn scene(&mut self) -> SettingsScene {
        self.state.ensure_rows(&self.app.shared);
        self.state.scene(&self.app.shared, Instant::now())
    }

    /// Save the frame drawn last as `<capture folder>/<name>-<size>.png`, when a folder was named.
    pub(super) fn save(&self, name: &str) {
        let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
            return;
        };
        let folder = std::path::Path::new(&folder);
        std::fs::create_dir_all(folder).expect("the capture folder can be created");
        let file = folder.join(format!("{name}-{}x{}.png", self.size.0, self.size.1));
        image::save_buffer(file, self.pixels.rgba(), self.size.0, self.size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}

pub(super) fn middle(rect: Rect) -> (f32, f32) {
    (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

/// The pointer's place over a slider at `fraction` of the way along it: the middle of the knob.
pub(super) fn along(slider: Rect, fraction: f32) -> (f32, f32) {
    middle(knob_rect(slider, fraction))
}

/// The rows of the screen that hold a number are the rows of a range, tab by tab, and every one of
/// them is drawn with a slider and a number of its own.
#[test]
fn every_row_of_a_range_is_a_number_row_with_a_slider() {
    for tab in SettingTab::SHOWN {
        let screen = Screen::on("number-rows", tab);
        let numbers: Vec<SettingId> = screen
            .state
            .rows
            .iter()
            .filter_map(|row| match row {
                SettingRow::Fixed(id) if number_spec(*id).is_some() => Some(*id),
                _ => None,
            })
            .collect();
        let expected = NUMBER_ROWS.iter().find(|(listed, _)| *listed == tab).map_or(&[][..], |(_, ids)| *ids);
        assert_eq!(numbers, expected, "{tab:?}");
        for (line, row) in screen.state.lines.iter().zip(&screen.state.rows) {
            let holds_number = matches!(row, SettingRow::Fixed(id) if number_spec(*id).is_some());
            assert_eq!(line.number.is_some(), holds_number, "{}", line.label);
        }
    }
}

/// An arrow moves every row that holds a number by the smallest change it can hold, and ten of
/// them with Shift held; the row stops at both ends of its range, and the zero key puts it back on
/// what it ships as.
#[test]
fn a_key_moves_every_number_row_by_its_smallest_change_and_ten_with_shift() {
    for (tab, ids) in NUMBER_ROWS {
        let mut screen = Screen::on("number-keys", tab);
        for id in ids.iter().copied() {
            let spec = number_spec(id).expect("the row holds a number");
            let shipped = screen.steps_of(id);
            let middle = (spec.min + spec.max) / 2;
            rbms_config::set_number_steps(&mut screen.app.shared.config, id, middle);
            screen.focus(id);
            screen.key(KeyCode::ArrowRight);
            assert_eq!(screen.steps_of(id), middle + 1, "{id:?}: one press is not the smallest change");
            assert_eq!(screen.reads(id), spec.text(middle + 1), "{id:?}");
            screen.shifted(KeyCode::ArrowRight);
            assert_eq!(screen.steps_of(id), middle + 11, "{id:?}: Shift is not ten of them");
            screen.key(KeyCode::ArrowLeft);
            screen.shifted(KeyCode::ArrowLeft);
            assert_eq!(screen.steps_of(id), middle, "{id:?}: the way back is not the way there");

            rbms_config::set_number_steps(&mut screen.app.shared.config, id, spec.max - NEAR_THE_END);
            screen.shifted(KeyCode::ArrowRight);
            screen.key(KeyCode::ArrowRight);
            assert_eq!(screen.steps_of(id), spec.max, "{id:?} did not stop at its upper end");
            rbms_config::set_number_steps(&mut screen.app.shared.config, id, spec.min + NEAR_THE_END);
            screen.shifted(KeyCode::ArrowLeft);
            screen.key(KeyCode::ArrowLeft);
            assert_eq!(screen.steps_of(id), spec.min, "{id:?} did not stop at its lower end");

            assert!(matches!(screen.key(if id == SettingId::KeyVolume { KeyCode::Numpad0 } else { KeyCode::Digit0 }), Transition::Stay));
            assert_eq!(screen.steps_of(id), shipped, "{id:?}: the zero key did not put the row back");
        }
    }
}

/// A press on a slider puts its row where the press landed, and the row then goes with the pointer
/// for as long as the button is held, onto its ends when the pointer passes them and wherever on
/// the screen the pointer goes. The settings file is written once, when the button comes back up.
#[test]
fn a_slider_carries_its_row_with_the_pointer_and_is_written_out_when_it_is_let_go_of() {
    let mut screen = Screen::on("number-drag", SettingTab::Audio);
    let settings = screen.app.shared.settings_path.clone();
    let id = SettingId::KeyVolume;
    let slider = screen.slider_of(id);
    assert!(!settings.exists(), "the settings file was written before anything moved");

    assert!(matches!(screen.press(along(slider, 0.5), MouseButton::Left), Transition::Stay));
    assert_eq!((screen.steps_of(id), screen.state.sel), (50, screen.at(id)), "a press in the middle of the slider is not half, with the cursor on the row");
    let mut last = 0;
    let mut x = slider.x - PAST_THE_SLIDER;
    while x < slider.x + slider.w + PAST_THE_SLIDER {
        screen.drag((x, slider.y));
        let now = screen.steps_of(id);
        assert!(now >= last, "the volume went back while the pointer went on");
        last = now;
        x += 1.0;
    }
    assert_eq!((last, screen.reads(id).as_str()), (100, "100%"), "the volume did not stop at its upper end");
    screen.drag((slider.x - PAST_THE_SLIDER, slider.y + PAST_THE_SLIDER));
    assert_eq!(screen.steps_of(id), 0, "the volume did not stop at its lower end, or the drag ended off the row");
    for fraction in [0.25, 0.75, 0.37] {
        screen.drag(along(slider, fraction));
        assert_eq!(screen.steps_of(id), (fraction * 100.0_f32).round() as i64);
        assert!(!settings.exists(), "the settings file was written while the slider was being dragged");
    }
    assert_eq!(screen.scene().dragged, Some(screen.at(id)), "the row is not drawn as the one being dragged");

    screen.let_go(along(slider, 0.37));
    let written = rbms_config::load(&settings).expect("the file the drag wrote reads back").config;
    assert_eq!(number_steps(&written, id), Some(37), "what was written is not where the drag ended");
    assert_eq!(screen.scene().dragged, None);
    std::fs::remove_file(&settings).expect("the settings file can be removed");
    screen.drag(along(slider, 0.9));
    screen.let_go(along(slider, 0.9));
    assert_eq!(screen.steps_of(id), 37, "the pointer moved the volume with no button held on its slider");
    assert!(!settings.exists(), "the settings file was written with no drag to end");
}

/// The count of voices reaches the output stream only by opening it again, so a slider dragged
/// along it is shown where the pointer is and stored once, when the button comes back up.
#[test]
fn a_row_that_reopens_the_output_is_stored_when_its_drag_ends() {
    let mut screen = Screen::on("number-carried", SettingTab::Audio);
    let id = SettingId::AudioPolyphony;
    screen.focus(id);
    let slider = screen.slider_of(id);
    let shipped = screen.steps_of(id);
    screen.press(along(slider, 0.0), MouseButton::Left);
    screen.drag(along(slider, 1.0));
    assert_eq!(screen.reads(id), AUDIO_POLYPHONY_MAX_VOICES.to_string(), "the row does not read where the pointer has it");
    assert_eq!(screen.scene().rows[screen.at(id)].number.map(|number| number.fraction()), Some(1.0), "the slider is not where the pointer has it");
    assert_eq!(screen.steps_of(id), shipped, "the count of voices was stored while its slider was being dragged");
    assert!(!screen.app.shared.config.audio.reopen_pending(), "the output was asked to open again while a slider was being dragged");
    screen.drag(along(slider, 0.0));
    assert_eq!(screen.reads(id), AUDIO_POLYPHONY_MIN_VOICES.to_string());
    screen.let_go(along(slider, 0.0));
    assert_eq!(screen.steps_of(id), AUDIO_POLYPHONY_MIN_VOICES as i64, "the count of voices was not stored when its drag ended");
    assert!(screen.app.shared.config.audio.reopen_pending(), "the output was not asked to open again once the drag ended");
    assert_eq!(screen.reads(id), AUDIO_POLYPHONY_MIN_VOICES.to_string());
}

/// Enter opens a row that holds a number to be typed into, without leaving the screen, and Enter
/// stores what was typed: held to the row's range, rounded to its smallest change and read past a
/// unit. What is not a number is refused and said on the row.
#[test]
fn enter_opens_a_number_row_to_be_typed_into_and_stores_what_was_typed() {
    let mut screen = Screen::on("number-type", SettingTab::Audio);
    let id = SettingId::MasterVolume;
    screen.focus(id);
    assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay), "Enter on a row that holds a number left the screen");
    let (at, edit) = screen.scene().typed.expect("Enter did not open the row");
    assert_eq!((at, edit.text(), edit.is_all_selected()), (screen.at(id), "100", true));
    assert_eq!(screen.scene().hint, crate::number_row::EDIT_HINT, "the hint does not name the keys of a row being typed into");
    screen.typed("4");
    screen.typed("2");
    assert_eq!(screen.steps_of(id), 100, "a figure was stored before it was finished");
    assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay));
    assert_eq!((screen.steps_of(id), screen.reads(id).as_str()), (42, "42%"));
    assert!(screen.scene().typed.is_none());
    assert!(screen.scene().hint.starts_with("LEFT RIGHT 1 STEP   SHIFT x10   ENTER TYPE"), "the hint does not name what sets the row under the cursor");

    screen.type_in(id, &["6", "5", "%"]);
    assert_eq!(screen.steps_of(id), 65, "a percent sign typed after the number was not read past");
    screen.type_in(id, &["2", "5", "0"]);
    assert_eq!(screen.steps_of(id), 100, "a figure past the upper end was not held to it");
    screen.type_in(id, &["-", "3"]);
    assert_eq!(screen.steps_of(id), 0, "a figure past the lower end was not held to it");
    screen.type_in(id, &["3", "3", ".", "6"]);
    assert_eq!(screen.steps_of(id), 34, "a figure between two steps was not rounded to the nearer");

    let before = screen.app.shared.config.clone();
    screen.focus(id);
    screen.key(KeyCode::Enter);
    screen.key(KeyCode::Backspace);
    screen.typed(".");
    assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay));
    assert_eq!(screen.app.shared.config, before, "what is not a number was stored");
    assert_eq!(screen.scene().refused, Some(screen.at(id)), "the row does not say that what was typed is not a number");
    assert!(!screen.state.numbers.input.is_refused(id, Instant::now() + REFUSAL_SHOWN_FOR), "the row goes on saying so");

    let mut judge = Screen::on("number-type-judge", SettingTab::Judge);
    judge.type_in(SettingId::JudgeWidthKeyGreat, &["1", "2", "3"]);
    assert_eq!(judge.app.shared.config.judge.judge_rate_key[1], 123);
    judge.type_in(SettingId::Total, &["3", "2", "1"]);
    assert_eq!((judge.app.shared.config.play.total_override, judge.reads(SettingId::Total).as_str()), (321.0, "321"));
    let mut select = Screen::on("number-type-select", SettingTab::Select);
    select.type_in(SettingId::PreviewFade, &["1", "3", "7", " ", "m", "s"]);
    assert_eq!(select.app.shared.config.library.preview_fade_ms, 137);
}

/// Escape while a row is being typed into drops what was typed and stays on the screen; the Escape
/// after it leaves. A key that moves the cursor or the tab on stores what was typed first, and so
/// does a press of the mouse.
#[test]
fn escape_while_typing_drops_what_was_typed_and_a_key_that_moves_on_stores_it() {
    let mut screen = Screen::on("number-escape", SettingTab::Audio);
    let settings = screen.app.shared.settings_path.clone();
    let id = SettingId::BgmVolume;
    let shipped = screen.steps_of(id);
    screen.focus(id);
    screen.key(KeyCode::Enter);
    screen.typed("7");
    for code in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::F8, KeyCode::F12, KeyCode::F5] {
        assert!(matches!(screen.key(code), Transition::Stay), "{code:?} left the screen, or opened another, with a row open");
    }
    assert!(matches!(screen.key(KeyCode::Escape), Transition::Stay), "Escape left the screen with a row open");
    assert_eq!((screen.steps_of(id), screen.scene().typed.is_none(), settings.exists()), (shipped, true, false), "Escape stored what was typed");

    screen.key(KeyCode::Enter);
    screen.typed("7");
    screen.key(KeyCode::ArrowDown);
    assert_eq!((screen.steps_of(id), screen.state.sel), (7, screen.at(id) + 1), "the arrow did not store what was typed and then move the cursor");
    screen.focus(id);
    screen.key(KeyCode::Enter);
    screen.typed("9");
    screen.key(KeyCode::Tab);
    assert_eq!(screen.steps_of(id), 9, "Tab dropped what was typed");
    assert_ne!(screen.state.current_tab(), SettingTab::Audio, "Tab did not move on after storing");

    let mut screen = Screen::on("number-press", SettingTab::Audio);
    screen.press(screen.on_the_number_of(id), MouseButton::Left);
    assert_eq!(screen.scene().typed.map(|(at, _)| at), Some(screen.at(id)), "a press on the number did not open the row");
    screen.typed("3");
    screen.typed("1");
    assert!(matches!(screen.press(screen.on_the_label_of(SettingId::GuideSe), MouseButton::Left), Transition::Stay));
    assert_eq!(screen.steps_of(id), 31, "a press on another row dropped what was typed");
    assert!(screen.scene().typed.is_none());
    assert!(matches!(screen.key(KeyCode::Escape), Transition::Back));
}

/// A press of the left button on the label of a row moves the cursor there and changes nothing,
/// whatever the row holds, and one of the right button, anywhere on a row that holds a value, takes
/// it one step back. A switch is turned over by a press on the switch itself, and Enter on a switch
/// goes back as it always did.
#[test]
fn a_press_on_a_label_only_moves_the_cursor_and_the_right_button_steps_back() {
    let mut screen = Screen::on("number-clicks", SettingTab::Audio);
    let id = SettingId::SystemVolume;
    let shipped = screen.steps_of(id);
    rbms_config::set_number_steps(&mut screen.app.shared.config, id, 40);
    screen.focus(SettingId::MasterVolume);
    screen.press(screen.on_the_label_of(id), MouseButton::Left);
    assert_eq!((screen.steps_of(id), screen.state.sel), (40, screen.at(id)), "a press on the label moved the row, or left the cursor behind");
    let places = [screen.on_the_label_of(id), middle(screen.slider_of(id)), screen.on_the_number_of(id)];
    for at in places {
        screen.press(at, MouseButton::Right);
    }
    assert_eq!(screen.steps_of(id), 37, "the right button does not step a row back wherever on it the press lands");
    assert_eq!((screen.scene().dragged, screen.scene().typed.is_none()), (None, true), "the right button took hold of the slider or opened the number");
    assert_ne!(shipped, 37);

    let guide = screen.app.shared.config.audio.guide_se;
    screen.press(screen.on_the_label_of(SettingId::GuideSe), MouseButton::Left);
    assert_eq!(
        (screen.app.shared.config.audio.guide_se, screen.state.sel),
        (guide, screen.at(SettingId::GuideSe)),
        "a press on the label turned the switch over"
    );
    let on_the_switch = middle(crate::settings_view::row_switch(screen.row_of(SettingId::GuideSe)));
    for presses in 1..=3 {
        screen.press(on_the_switch, MouseButton::Left);
        assert_eq!(screen.app.shared.config.audio.guide_se, guide ^ (presses % 2 == 1), "a press on the switch did not turn it over");
        assert_eq!(screen.app.shared.syssound.guide_enabled(), screen.app.shared.config.audio.guide_se, "the cues do not follow the switch at once");
        assert_eq!((screen.scene().dragged, screen.scene().typed.is_none()), (None, true), "a switch has a slider or a number");
    }
    screen.press(screen.on_the_label_of(SettingId::GuideSe), MouseButton::Right);
    assert_eq!(screen.app.shared.config.audio.guide_se, guide, "the right button did not turn the switch back");
    screen.focus(SettingId::GuideSe);
    assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay), "Enter on a switch left the screen");
    assert_eq!(screen.app.shared.config.audio.guide_se, !guide, "Enter on a switch did not turn it over, as the skin and play menus do");
    assert!(matches!(screen.key(KeyCode::Escape), Transition::Back), "Escape is the key that leaves");
}

/// The volumes of the AUDIO tab drawn for a person to look at, at the size the screen is laid out
/// for and at full HD: every slider and number in its own room, one being dragged, one being typed
/// into with its number selected and with a figure typed, and one that refused what was typed.
#[test]
fn the_number_rows_of_the_settings_screen_can_be_captured() {
    for size in DRAWN_SIZES {
        let mut screen = Screen::sized(&format!("number-capture-{}", size.0), SettingTab::Audio, size);
        for (id, percent) in [(SettingId::MasterVolume, 100), (SettingId::KeyVolume, 0), (SettingId::BgmVolume, 37), (SettingId::SystemVolume, 80)] {
            rbms_config::set_number_steps(&mut screen.app.shared.config, id, percent);
        }
        screen.state.dirty = true;
        screen.focus(SettingId::BgmVolume);
        screen.save("settings-number-volumes");
        let before = screen.pixels.pixel_checksum();

        let slider = screen.slider_of(SettingId::BgmVolume);
        screen.press(along(slider, 0.37), MouseButton::Left);
        screen.drag(along(slider, 0.62));
        screen.frame();
        assert_ne!(screen.pixels.pixel_checksum(), before, "a slider being dragged is drawn as it was before");
        screen.save("settings-number-volume-dragging");
        screen.let_go(along(slider, 0.62));

        screen.key(KeyCode::Enter);
        screen.frame();
        screen.save("settings-number-volume-selected");
        screen.typed("4");
        screen.typed("5");
        screen.frame();
        screen.save("settings-number-volume-typing");
        screen.key(KeyCode::Enter);
        screen.key(KeyCode::Enter);
        screen.key(KeyCode::Backspace);
        screen.key(KeyCode::Enter);
        screen.frame();
        assert!(screen.scene().refused.is_some());
        screen.save("settings-number-volume-refused");

        let mut judge = Screen::sized(&format!("number-capture-judge-{}", size.0), SettingTab::Judge, size);
        judge.focus(SettingId::JudgeWidthKeyGreat);
        judge.save("settings-number-judge-tab");
        let mut display = Screen::sized(&format!("number-capture-display-{}", size.0), SettingTab::Display, size);
        display.focus(SettingId::JudgeTextY);
        display.save("settings-number-display-tab");
    }
}

/// How far inside an edge of a slider's knob a press that takes hold of the knob by that edge
/// lands.
const INSIDE_THE_KNOB: f32 = 0.5;

/// A press on the knob of a slider takes hold of the knob where the press landed: every row that
/// holds a number is where it was until the pointer moves, wherever across the knob it was taken
/// hold of, and letting go of it writes nothing new into the row.
#[test]
fn a_press_on_the_knob_takes_hold_of_it_and_leaves_the_row_where_it_was() {
    for (tab, ids) in NUMBER_ROWS {
        let mut screen = Screen::on("number-grip", tab);
        for id in ids.iter().copied() {
            let spec = number_spec(id).expect("the row holds a number");
            for steps in [spec.min, (spec.min * 2 + spec.max) / 3, spec.max] {
                rbms_config::set_number_steps(&mut screen.app.shared.config, id, steps);
                screen.state.dirty = true;
                screen.focus(id);
                let slider = screen.slider_of(id);
                let knob = knob_rect(slider, spec.fraction(steps) as f32);
                let down = knob.y + knob.h * 0.5;
                let read = screen.reads(id);
                for across in [knob.x + INSIDE_THE_KNOB, knob.x + knob.w * 0.5, knob.x + knob.w - INSIDE_THE_KNOB] {
                    screen.press((across, down), MouseButton::Left);
                    assert_eq!(screen.scene().dragged, Some(screen.at(id)), "{id:?}: a press on the knob did not take hold of it");
                    assert_eq!(screen.steps_of(id), steps, "{id:?}: a press on the knob at {across} moved the row, the knob being at {}", knob.x);
                    assert_eq!(screen.reads(id), read, "{id:?}: a press on the knob changed what the row reads");
                    screen.let_go((across, down));
                    assert_eq!(screen.steps_of(id), steps, "{id:?}: letting go of the knob moved the row");
                }
            }
        }
    }
}

/// A drag that is still going when a key is pressed is ended by the key, and what it had carried is
/// stored: Escape leaves the screen with the count of voices the slider showed, and so does a drag
/// whose button was never seen coming back up.
#[test]
fn a_key_pressed_during_a_drag_stores_where_the_drag_had_the_row() {
    let mut screen = Screen::on("number-drag-escape", SettingTab::Audio);
    let settings = screen.app.shared.settings_path.clone();
    let id = SettingId::AudioPolyphony;
    screen.focus(id);
    let slider = screen.slider_of(id);
    screen.press(along(slider, 0.0), MouseButton::Left);
    screen.drag(along(slider, 1.0));
    assert_eq!(screen.reads(id), AUDIO_POLYPHONY_MAX_VOICES.to_string());
    assert!(matches!(screen.key(KeyCode::Escape), Transition::Back), "Escape did not leave the screen");
    assert_eq!(screen.steps_of(id), AUDIO_POLYPHONY_MAX_VOICES as i64, "the count of voices the slider showed was dropped by the key that left the screen");
    assert_eq!(screen.scene().dragged, None, "the drag outlived the key");
    let written = rbms_config::load(&settings).expect("the file Escape wrote reads back").config;
    assert_eq!(number_steps(&written, id), Some(AUDIO_POLYPHONY_MAX_VOICES as i64), "what was written is not what the slider showed");

    let mut screen = Screen::on("number-drag-key", SettingTab::Audio);
    screen.focus(id);
    let slider = screen.slider_of(id);
    screen.press(along(slider, 1.0), MouseButton::Left);
    screen.drag(along(slider, 0.0));
    screen.key(KeyCode::ArrowDown);
    assert_eq!(screen.steps_of(id), AUDIO_POLYPHONY_MIN_VOICES as i64, "a key that moves the cursor dropped what the drag had carried");
    screen.drag(along(slider, 1.0));
    assert_eq!(screen.steps_of(id), AUDIO_POLYPHONY_MIN_VOICES as i64, "the pointer went on moving a row whose drag a key had ended");
}

/// With the cursor on a row that holds a number the hint names what sets the row, and still names
/// the keys that open the play menu, the skin menu and the key editor: the AUDIO tab opens on such
/// a row.
#[test]
fn the_hint_of_a_number_row_still_names_the_keys_to_the_other_screens() {
    rbms_render::font::use_embedded_fonts_only();
    let mut screen = Screen::on("number-hint", SettingTab::Audio);
    screen.focus(SettingId::MasterVolume);
    let hint = screen.scene().hint;
    for part in ["LEFT RIGHT 1 STEP", "SHIFT x10", "ENTER TYPE", "0 DEFAULT", "DRAG", "F8 PLAY", "F12 SKINS", "F5 KEYS"] {
        assert!(hint.contains(part), "the hint of a number row does not say {part}: {hint}");
    }
    screen.focus(SettingId::GuideSe);
    let hint = screen.scene().hint;
    assert!(hint.starts_with("LEFT RIGHT SWITCH   ENTER SWITCH   0 DEFAULT"), "the hint of a switch names other keys than it hears: {hint}");
    assert!(hint.ends_with("ESC SAVE") && !hint.contains("SAVE AND CLOSE"), "the hint says Enter leaves: {hint}");
    assert!(!hint.contains("TYPE") && !hint.contains("DRAG"), "the hint of a switch names what sets a number: {hint}");
}
