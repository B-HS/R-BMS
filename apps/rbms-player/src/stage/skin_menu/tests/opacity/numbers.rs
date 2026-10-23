//! The rows of the skin menu that hold a number -- an opacity, and an axis of an offset -- set by
//! their slider and by a figure typed into them, over the fading play document.

use std::time::Instant;

use super::*;
use crate::number_row::{EDIT_HINT, REFUSAL_SHOWN_FOR, knob_rect, steps_at};
use crate::pointer::{PointerInput, route_pointer};
use crate::skin_select::fixtures::FADING_MARK_PLACE;

/// How far past an end of a slider, and how far off its row, the pointer is taken while it drags.
const PAST_THE_SLIDER: f32 = 300.0;

/// The pointer's place over a slider at `fraction` of the way along it: the middle of the knob.
fn along(slider: Rect, fraction: f32) -> (f32, f32) {
    middle(knob_rect(slider, fraction))
}

impl Menu {
    /// The step one row that holds a number stands on.
    fn steps_of(&self, row: SettingRow) -> i64 {
        self.app.shared.row_number(row).unwrap_or_else(|| panic!("{row:?} holds no number")).steps
    }

    /// Open the row to be typed into with Enter, type `typed` into it key by key, and press Enter.
    fn type_in(&mut self, row: SettingRow, typed: &[&'static str]) {
        self.focus(row);
        self.key(KeyCode::Enter);
        for text in typed {
            self.typed(text);
        }
        self.key(KeyCode::Enter);
    }

    /// The offset stored for one id of the document being configured.
    fn stored_offset(&self, id: i32) -> Option<rbms_skin::dst::SkinOffset> {
        let config = &self.app.shared.config;
        let path = self.app.shared.skins.document_path(config, config.skin.screen)?;
        config.skin.customisation(path)?.offsets.get(&id).copied()
    }
}

/// A press on the slider of an opacity puts the row where the press landed, and the row then goes
/// with the pointer for as long as the button is held, onto its ends when the pointer passes them
/// and wherever on the screen the pointer goes. The preview is drawn with the opacity on the frame
/// after each move, with its document never read again.
#[test]
fn the_slider_of_an_opacity_carries_it_with_the_pointer_and_the_preview_follows() {
    let mut menu = Menu::over(app_with_fading_play("number-drag"));
    let screen = menu.app.shared.config.skin.screen;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let build = menu.app.shared.skins.build_of(screen);
    let row = menu.row_called(FADING_PANEL_ROW);
    menu.focus(row);
    menu.frame();
    let slider = menu.slider_of(row);
    let spec = menu.app.shared.row_number(row).expect("an opacity holds a number").track();
    let whole = Color::rgb(FADING_PANEL.1.0, FADING_PANEL.1.1, FADING_PANEL.1.2);

    menu.state.focus(0);
    assert!(matches!(menu.click(along(slider, 0.5)), Transition::Stay));
    assert_eq!((menu.steps_of(row), menu.stored_alpha(FADING_PANEL_OPACITY)), (50, Some(opacity_alpha(50))), "a press in the middle of the slider is not half");
    assert_eq!(menu.state.focused(), Some(row), "the press did not move the cursor to the row");
    assert!(menu.state.numbers.is_dragging(row));
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a press on a slider cost a frame of the preview");
    assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA), "the panel is not at half on the frame after the press");

    let mut x = slider.x - PAST_THE_SLIDER;
    let mut last = 0;
    while x < slider.x + slider.w + PAST_THE_SLIDER {
        menu.drag((x, slider.y));
        let now = menu.steps_of(row);
        assert!(now >= last, "the opacity went back while the pointer went on");
        assert_eq!(now, steps_at(&spec, slider, x), "the opacity is not under the pointer at {x}");
        last = now;
        x += 1.0;
    }
    assert_eq!((last, menu.value_of(row).as_str()), (100, "100%"), "the opacity went past all of it");
    menu.drag((slider.x - PAST_THE_SLIDER, slider.y + PAST_THE_SLIDER));
    assert_eq!((menu.steps_of(row), menu.stored_alpha(FADING_PANEL_OPACITY)), (0, Some(-255.0)), "the opacity did not stop at none of it");
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), Color::rgb(0, 0, 0), "a panel dragged to none of its opacity is drawn");

    menu.drag(along(slider, 0.5));
    let over_the_preview = middle(PREVIEW_RECT);
    let routed =
        route_pointer(&mut menu.state, &mut FrameCtx { shared: &mut menu.app.shared, now: Instant::now(), dt: 0.0 }, over_the_preview, PointerInput::Drag);
    assert!(matches!(routed, Transition::Stay));
    assert_eq!(menu.steps_of(row), 100, "the drag did not go on with the pointer over the preview, which is past the slider's end");
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), whole, "the panel did not come back with its opacity");

    menu.drag(along(slider, 0.5));
    menu.let_go(along(slider, 0.5));
    assert_eq!(menu.state.numbers.dragged(), None);
    menu.drag(along(slider, 0.1));
    assert_eq!(menu.steps_of(row), 50, "the pointer moved the opacity with no button held on its slider");
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA));
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an opacity being dragged had the document read again");
    assert_eq!(menu.value_of(menu.row_called(FADING_NOTES_ROW)), "100%", "one opacity moved another");
}

/// A drag writes the settings file once, when the button comes back up, and not while the slider
/// moves; a press that begins no drag writes nothing.
#[test]
fn a_drag_writes_the_settings_file_once_when_it_ends() {
    let mut menu = Menu::over(app_with_fading_play("number-drag-save"));
    let settings = menu.app.shared.settings_path.clone();
    let row = menu.row_called(FADING_GROUND_ROW);
    menu.focus(row);
    menu.frame();
    let slider = menu.slider_of(row);
    let _ = std::fs::remove_file(&settings);

    menu.click(along(slider, 0.9));
    for fraction in [0.8, 0.6, 0.4, 0.3] {
        menu.drag(along(slider, fraction));
        assert!(!settings.exists(), "the settings file was written while the slider was being dragged");
    }
    menu.let_go(along(slider, 0.3));
    let written = rbms_config::load(&settings).expect("the file the drag wrote reads back").config;
    assert_eq!(written.skin, menu.app.shared.config.skin, "what was written is not where the drag ended");
    assert_eq!(menu.stored_alpha(FADING_GROUND_OPACITY), Some(opacity_alpha(30)));

    std::fs::remove_file(&settings).expect("the settings file can be removed");
    menu.let_go(along(slider, 0.3));
    menu.drag(along(slider, 0.7));
    menu.click(menu.on_the_label_of(row));
    menu.right_click(menu.on_the_label_of(row));
    assert_eq!(menu.value_of(row), "30%");
    assert!(!settings.exists(), "the settings file was written with no drag to end");
}

/// An axis of an offset runs both ways from nothing, so the middle of its slider is nothing and
/// its ends are the limits of the axis. The slider of a place has fewer places than the axis has
/// pixels, which is why its help names the keys and Enter for setting one exactly.
#[test]
fn the_slider_of_an_offset_runs_both_ways_from_nothing() {
    let mut menu = Menu::over(app_with_fading_play("number-offset"));
    let row = menu.row_called("MARK PLACE X");
    menu.focus(row);
    menu.frame();
    let slider = menu.slider_of(row);
    menu.click(along(slider, 0.5));
    assert_eq!(menu.value_of(row), "+0");
    menu.drag(along(slider, 1.0));
    assert_eq!(menu.value_of(row), "+2000");
    menu.drag(along(slider, 0.0));
    assert_eq!(menu.value_of(row), "-2000");
    menu.drag(along(slider, 0.75));
    assert_eq!((menu.value_of(row).as_str(), menu.stored_offset(FADING_MARK_PLACE).map(|offset| offset.x)), ("+1000", Some(1000.0)));
    menu.let_go(along(slider, 0.75));
    assert!(slider.w < 4000.0, "the slider has a place for every pixel of the axis, and its help need not say otherwise");
    assert!(row_help(&menu.app.shared, row).contains("arrows or ENTER set it exactly"), "{}", row_help(&menu.app.shared, row));
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(row), "+1001", "an arrow does not set what the slider is too coarse for");
}

/// Enter, or a press on its number, opens an opacity to be typed into, on the percent it reads and
/// with all of it selected. Enter stores what was typed as a percent, held to the row's range; what
/// is not a number is refused and said on the row.
#[test]
fn an_opacity_is_typed_as_a_percent_and_held_to_its_range() {
    let mut menu = Menu::over(app_with_fading_play("number-type"));
    let row = menu.row_called(FADING_GROUND_ROW);
    menu.focus(row);
    assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay));
    let edit = menu.state.numbers.edit_of(row).expect("Enter did not open the row");
    assert_eq!((edit.text(), edit.is_all_selected()), ("100", true));
    menu.typed("4");
    menu.typed("0");
    assert_eq!(menu.stored_alpha(FADING_GROUND_OPACITY), None, "a figure was stored before it was finished");
    menu.key(KeyCode::Enter);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)), ("40%", Some(opacity_alpha(40))));
    assert_eq!(menu.state.numbers.editing(), None);

    menu.type_in(row, &["6", "5", "%"]);
    assert_eq!(menu.value_of(row), "65%", "a percent sign typed after the number was not read past");
    menu.type_in(row, &["1", "5", "0"]);
    assert_eq!(
        (menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)),
        ("150% (+128)", Some(opacity_alpha(150))),
        "a typed percent does not reach past all of the opacity, where a slider and an arrow end"
    );
    menu.type_in(row, &["2", "5", "0"]);
    assert_eq!(
        (menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)),
        ("200% (+255)", Some(255.0)),
        "a percent past the row's end was not held to it"
    );
    menu.type_in(row, &["-", "5"]);
    assert_eq!(menu.value_of(row), "0%", "a percent under none of the opacity was not held to it");
    menu.type_in(row, &["3", "3", ".", "6"]);
    assert_eq!(menu.value_of(row), "34%", "a percent between two steps was not rounded to the nearer");

    let before = menu.app.shared.config.clone();
    menu.focus(row);
    menu.key(KeyCode::Enter);
    menu.key(KeyCode::Backspace);
    menu.typed("-");
    let refused_at = Instant::now();
    assert!(matches!(menu.key_at(KeyCode::Enter, refused_at), Transition::Stay));
    assert_eq!(menu.app.shared.config, before, "what is not a number was stored");
    assert!(menu.state.numbers.is_refused(row, refused_at), "the row does not say that what was typed is not a number");
    assert!(!menu.state.numbers.is_refused(row, refused_at + REFUSAL_SHOWN_FOR), "the row goes on saying so");

    menu.frame();
    menu.click(menu.on_the_number_of(row));
    assert_eq!(menu.state.numbers.editing(), Some(row), "a press on the number did not open the row");
    menu.typed("7");
    menu.click(middle(PREVIEW_RECT));
    assert_eq!((menu.value_of(row).as_str(), menu.state.numbers.editing()), ("7%", None), "a press elsewhere did not store what was typed");
}

/// An axis of an offset is typed with its sign, and held to the limits of the axis.
#[test]
fn an_offset_is_typed_with_its_sign_and_held_to_the_limits_of_its_axis() {
    let mut menu = Menu::over(app_with_fading_play("number-type-offset"));
    let row = menu.row_called("MARK PLACE X");
    menu.type_in(row, &["-", "1", "2"]);
    assert_eq!(menu.value_of(row), "-12");
    menu.focus(row);
    menu.key(KeyCode::Enter);
    assert_eq!(menu.state.numbers.edit_of(row).map(|edit| edit.text().to_string()), Some("-12".to_string()), "the row does not open on the number it holds");
    menu.key(KeyCode::Escape);
    menu.type_in(row, &["+", "7", "p", "x"]);
    assert_eq!(menu.value_of(row), "+7", "a unit typed after the number was not read past");
    menu.type_in(row, &["5", "0", "0", "0"]);
    assert_eq!(menu.value_of(row), "+2000");
    menu.type_in(row, &["-", "9", "9", "9", "9"]);
    assert_eq!(menu.value_of(row), "-2000");
    menu.type_in(row, &["1", "2", ".", "6"]);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_offset(FADING_MARK_PLACE).map(|offset| offset.x)), ("+13", Some(13.0)));
}

/// Escape while a row is being typed into drops what was typed and stays on the menu, and the
/// Escape after it writes the settings out and leaves. A key that moves on stores what was typed
/// first. While a row is open, the foot of the screen names the keys that are then in force.
#[test]
fn escape_while_typing_drops_what_was_typed_and_a_key_that_moves_on_stores_it() {
    let mut menu = Menu::over(app_with_fading_play("number-escape"));
    let settings = menu.app.shared.settings_path.clone();
    let _ = std::fs::remove_file(&settings);
    let row = menu.row_called(FADING_GROUND_ROW);
    let screen = menu.app.shared.config.skin.screen;
    menu.focus(row);
    menu.key(KeyCode::Enter);
    menu.typed("2");
    menu.typed("0");
    for code in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Home, KeyCode::End, KeyCode::F12, KeyCode::Digit0] {
        assert!(matches!(menu.key(code), Transition::Stay));
    }
    assert_eq!(menu.value_of(row), "100%", "a key reached the row while it was being typed into");
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Stay), "Escape left the menu with a row open");
    assert_eq!((menu.state.numbers.editing(), menu.value_of(row).as_str(), menu.state.focused()), (None, "100%", Some(row)));
    assert!(!settings.exists(), "a figure that was dropped was written out");

    menu.key(KeyCode::Enter);
    menu.typed("2");
    menu.typed("0");
    menu.key(KeyCode::ArrowDown);
    assert_eq!(menu.value_of(row), "20%", "the arrow dropped what was typed");
    assert_eq!(menu.state.focused(), Some(menu.row_called(FADING_LINE_ROW)), "the arrow did not move the cursor after storing");
    menu.key(KeyCode::Enter);
    menu.typed("6");
    menu.key(KeyCode::Tab);
    assert_ne!(menu.app.shared.config.skin.screen, screen, "Tab did not step the screen after storing");
    menu.choose(screen);
    assert_eq!(menu.value_of(menu.row_called(FADING_LINE_ROW)), "6%", "Tab dropped what was typed");
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back));
    assert!(settings.exists(), "the Escape that leaves did not write the settings out");
}

/// The wheel scrolls the list over a slider as it does anywhere on the list, and never moves the
/// row under it.
#[test]
fn the_wheel_over_a_slider_scrolls_the_list_and_leaves_the_row_alone() {
    let mut menu = Menu::over(app_with_fading_play("number-wheel"));
    let row = menu.row_called("MARK PLACE X");
    menu.focus(row);
    menu.frame();
    let before = (menu.app.shared.config.clone(), menu.state.top);
    let on_slider = middle(menu.slider_of(row));
    menu.wheel(on_slider, 2.0);
    assert_eq!(menu.state.top, before.1 + 2, "the list did not scroll under the pointer");
    assert_eq!(menu.app.shared.config, before.0, "a turn of the wheel moved a row");
}

/// An opacity a settings file holds as an offset that adds alpha reads as more than all of it, with
/// its slider at the end, and is left exactly as it is stored by everything that does not ask for
/// another percent: a press that takes hold of its knob, Enter on the figure it opens on, and the
/// frames that draw it. An arrow steps it from where it stands, and a drag brings it onto the
/// slider.
#[test]
fn an_opacity_stored_past_all_of_it_reads_as_it_is_and_is_kept_until_it_is_moved() {
    let mut menu = Menu::over(app_with_fading_play("number-past"));
    let config = &mut menu.app.shared.config;
    let path = menu.app.shared.skins.document_path(config, config.skin.screen).expect("the document is chosen").to_owned();
    config.skin.customise(&path).offsets.insert(FADING_PANEL_OPACITY, rbms_skin::dst::SkinOffset { a: 128.0, ..Default::default() });
    menu.state.dirty = true;
    let row = menu.row_called(FADING_PANEL_ROW);
    menu.focus(row);
    menu.frame();
    let number = menu.app.shared.row_number(row).expect("an opacity holds a number");
    assert_eq!((menu.value_of(row).as_str(), number.steps, number.fraction()), ("150% (+128)", 150, 1.0));
    assert_eq!(menu.stored_alpha(FADING_PANEL_OPACITY), Some(128.0), "reading the opacity rewrote it");

    let slider = menu.slider_of(row);
    let on_the_knob = along(slider, 1.0);
    menu.click(on_the_knob);
    assert!(menu.state.numbers.is_dragging(row), "the knob of a row past its slider is not at the slider's end");
    menu.let_go(on_the_knob);
    menu.key(KeyCode::Enter);
    assert_eq!(menu.state.numbers.edit_of(row).map(|edit| edit.text().to_string()), Some("150".to_string()), "the row does not open on the percent it reads");
    menu.key(KeyCode::Enter);
    menu.frame();
    assert_eq!(menu.stored_alpha(FADING_PANEL_OPACITY), Some(128.0), "taking hold of the knob, or Enter on the figure the row opened on, rewrote the opacity");

    menu.key(KeyCode::ArrowLeft);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_PANEL_OPACITY)), ("149% (+125)", Some(opacity_alpha(149))));
    menu.key(KeyCode::ArrowRight);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(row), "151% (+130)", "an arrow does not step a row past its slider from where it stands");

    menu.frame();
    menu.click(on_the_knob);
    menu.drag(along(slider, 0.5));
    assert_eq!(
        (menu.value_of(row).as_str(), menu.stored_alpha(FADING_PANEL_OPACITY)),
        ("50%", Some(opacity_alpha(50))),
        "a drag did not bring the row onto its slider"
    );
    menu.drag((slider.x + slider.w + PAST_THE_SLIDER, slider.y));
    menu.let_go(along(slider, 1.0));
    assert_eq!(menu.value_of(row), "100%", "a slider carried an opacity past all of it");
    menu.key(KeyCode::ArrowRight);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_PANEL_OPACITY)), ("100%", Some(0.0)), "an arrow carried an opacity past all of it");
}

/// What a row that holds a number writes fits the room a number has beside its slider at every
/// limit an axis and an opacity have, and the lines of keys fit the foot of the screen.
#[test]
fn the_number_of_a_number_row_fits_beside_its_slider() {
    rbms_render::font::use_embedded_fonts_only();
    for figure in ["100%", "200%", "0%", "+2000", "-2000", "+360", "-360", "+255", "-255"] {
        let width = rbms_render::text_width(figure, VALUE_SCALE);
        assert!(width <= number_value_room(), "{figure} is {width} wide in a room of {}", number_value_room());
    }
    assert!(number_label_room() > number_value_room(), "the name of a row has less room than its number");
    for hint in [SkinMenuText::Hint.get(language()), EDIT_HINT] {
        assert_eq!(fit_text(hint, HINT_SCALE, CW as f32 - MARGIN * 2.0), hint, "a line of keys is cut short");
    }
    for help in [
        SkinMenuText::OffsetPlaceHelp.get(language()),
        SkinMenuText::OffsetAngleHelp.get(language()),
        SkinMenuText::OffsetAlphaHelp.get(language()),
        SkinMenuText::OpacityHelp.get(language()),
        SkinMenuText::AutomaticPlaceHelp.get(language()),
        SkinMenuText::AutomaticAlphaHelp.get(language()),
    ] {
        assert_eq!(fit_text(help, INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0), help, "what is said of a row is cut short");
    }
}

/// The rows that hold a number drawn for a person to look at, at the size the menu is laid out for
/// and in a larger window: the opacities with their sliders, one being dragged, one being typed
/// into with its number selected and with a figure typed, one that refused what was typed, and one
/// a typed figure put past all of its opacity.
#[test]
fn the_number_rows_of_the_skin_menu_can_be_captured() {
    for size in [UI, LARGE] {
        let mut menu = Menu::sized(app_with_fading_play(&format!("number-capture-{}", size.0)), size);
        assert_eq!(menu.until_said(), PreviewStatus::Drawn);
        for (label, percent) in [(FADING_GROUND_ROW, 0), (FADING_LINE_ROW, 35), (FADING_NOTES_ROW, 80)] {
            let row = menu.row_called(label);
            menu.app.shared.set_row_number(row, percent);
        }
        menu.state.dirty = true;
        let panel = menu.row_called(FADING_PANEL_ROW);
        menu.focus(panel);
        menu.capture_at("skin-menu-number-opacity-sliders", CAPTURE_SECONDS);

        let slider = menu.slider_of(panel);
        menu.click(along(slider, 0.9));
        menu.drag(along(slider, 0.5));
        menu.frame();
        assert!(menu.state.numbers.is_dragging(panel));
        if size == UI {
            assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA));
        }
        menu.save("skin-menu-number-opacity-dragging");
        menu.let_go(along(slider, 0.5));

        menu.key(KeyCode::Enter);
        menu.frame();
        menu.save("skin-menu-number-opacity-selected");
        menu.typed("7");
        menu.typed("5");
        menu.frame();
        menu.save("skin-menu-number-opacity-typing");
        menu.key(KeyCode::Enter);
        menu.key(KeyCode::Enter);
        menu.key(KeyCode::Backspace);
        let refused_at = Instant::now();
        menu.key_at(KeyCode::Enter, refused_at);
        menu.frame_at(refused_at);
        assert!(menu.state.numbers.is_refused(panel, refused_at));
        menu.save("skin-menu-number-opacity-refused");

        let notes = menu.row_called(FADING_NOTES_ROW);
        menu.type_in(notes, &["1", "5", "0"]);
        menu.frame();
        assert_eq!(menu.value_of(notes), "150% (+128)");
        menu.save("skin-menu-number-opacity-typed-past-all");

        let place = menu.row_called("MARK PLACE X");
        menu.focus(place);
        menu.frame();
        menu.save("skin-menu-number-offset-rows");
    }
}

/// How far inside an edge of a slider's knob a press that takes hold of the knob by that edge
/// lands.
const INSIDE_THE_KNOB: f32 = 0.5;

/// A press on the knob of a slider takes hold of the knob where the press landed: an opacity and an
/// axis of an offset are where they were until the pointer moves, wherever across the knob it was
/// taken hold of.
#[test]
fn a_press_on_the_knob_takes_hold_of_it_and_leaves_the_row_where_it_was() {
    let mut menu = Menu::over(app_with_fading_play("number-grip"));
    for (label, steps) in [(FADING_GROUND_ROW, 62), (FADING_PANEL_ROW, 100), (FADING_LINE_ROW, 0), ("MARK PLACE X", 1237)] {
        let row = menu.row_called(label);
        menu.app.shared.set_row_number(row, steps);
        menu.state.dirty = true;
        menu.focus(row);
        menu.frame();
        let slider = menu.slider_of(row);
        let knob = knob_rect(slider, menu.app.shared.row_number(row).expect("the row holds a number").fraction());
        let down = knob.y + knob.h * 0.5;
        for across in [knob.x + INSIDE_THE_KNOB, knob.x + knob.w * 0.5, knob.x + knob.w - INSIDE_THE_KNOB] {
            menu.click((across, down));
            assert!(menu.state.numbers.is_dragging(row), "{label}: a press on the knob did not take hold of it");
            assert_eq!(menu.steps_of(row), steps, "{label}: a press on the knob at {across} moved the row, the knob being at {}", knob.x);
            menu.let_go((across, down));
            assert_eq!(menu.steps_of(row), steps, "{label}: letting go of the knob moved the row");
        }
    }
}
