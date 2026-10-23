use super::*;
use crate::stage::render_tests::app;

pub(super) fn press(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: None }
}

fn typed(text: &str) -> KeyInput<'_> {
    KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) }
}

/// The in-place editor is the same editor the table manager's URL entry is, so a value typed
/// into a settings row can be corrected in the middle rather than only from the end.
#[test]
fn a_settings_row_can_be_edited_anywhere_in_the_line() {
    let mut app = app();
    let mut state = SettingsState::on_tab(SettingTab::Network);
    state.text_input = Some(TextEdit::from_text("dj@exampletest"));

    state.text_input_key(&mut app.shared, &press(KeyCode::Home));
    for _ in 0..10 {
        state.text_input_key(&mut app.shared, &press(KeyCode::ArrowRight));
    }
    state.text_input_key(&mut app.shared, &typed("."));
    assert_eq!(state.text_input.as_ref().expect("the editor is still open").text(), "dj@example.test");

    state.text_input_key(&mut app.shared, &press(KeyCode::End));
    state.text_input_key(&mut app.shared, &press(KeyCode::Backspace));
    assert_eq!(state.text_input.as_ref().expect("the editor is still open").text(), "dj@example.tes");
}

/// A password on screen carries neither the password nor, once it is long, its length.
#[test]
fn a_secret_row_shows_a_capped_mask_and_never_the_value() {
    let secret = TextEdit::from_text("a-very-long-passphrase-indeed");
    let shown = editor_display(&secret, true);
    assert!(!shown.contains("passphrase"), "the value reached the screen");
    assert!(shown.chars().filter(|c| *c == PASSWORD_MASK_CHAR).count() <= PASSWORD_MASK_MAX, "the mask reports the length");
}

/// A value too long for the column scrolls under the caret rather than showing only its start.
#[test]
fn a_long_value_shows_the_stretch_the_caret_is_in() {
    let long = TextEdit::from_text("https://ir.example.test/a/very/long/endpoint/path/that/does/not/fit");
    let at_end = editor_display(&long, false);
    let mut at_start = long.clone();
    at_start.home();
    assert_ne!(at_end, editor_display(&at_start, false), "the window does not follow the caret");
    assert!(at_end.ends_with('_'), "the caret is at the end of the line it is at the end of");
}

/// A native picker can sit in front of the user for as long as they like, so the font row opens
/// it beside the frame loop and collects the answer in `update` — the window keeps drawing while
/// it is up, and the font lands when they choose one.
#[test]
fn the_font_row_waits_for_its_picker_off_the_frame_loop() {
    let mut app = app();
    let mut state = SettingsState::on_tab(SettingTab::Display);
    let (tx, handle) = crate::dialog::handle_for_tests();
    state.font_picker = Some(handle);
    state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
    assert!(state.font_picker.is_some(), "the screen gave up on a picker nobody has answered");

    tx.send(None).expect("the handle is still held");
    state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
    assert!(state.font_picker.is_none(), "the screen kept waiting after the picker was dismissed");
    assert_eq!(app.shared.config.display.font_path, None, "a dismissed picker changed the font");
}

/// A font the user picked is loaded and written out, so it is the face the next launch starts
/// with. A file that is not a font is reported rather than stored.
#[test]
fn a_picked_font_that_cannot_be_read_is_not_stored() {
    let mut app = app();
    let mut state = SettingsState::on_tab(SettingTab::Display);
    let (tx, handle) = crate::dialog::handle_for_tests();
    state.font_picker = Some(handle);
    tx.send(Some(std::path::PathBuf::from("/fonts/no-such-face.ttf"))).expect("the handle is still held");
    state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
    assert!(state.font_picker.is_none());
    assert_eq!(app.shared.config.display.font_path, None, "a font that could not be read was stored anyway");
}

/// The download folder row picks its folder the way the font row picks its file: beside the frame
/// loop, keeping what the picker came back with and leaving the row alone when it is dismissed. A
/// step to the left, which is also what the zero key does, puts it back to none chosen without
/// opening a picker.
#[test]
fn the_download_folder_row_opens_a_picker_and_zero_clears_it() {
    let mut app = app();
    let mut state = SettingsState::on_tab(SettingTab::Select);
    let (tx, handle) = crate::dialog::handle_for_tests();
    state.folder_pickers.download = Some(handle);
    state.update(&mut ctx(&mut app));
    assert!(state.folder_pickers.download.is_some(), "the screen gave up on a picker nobody has answered");

    tx.send(Some(std::path::PathBuf::from("/downloads/charts"))).expect("the handle is still held");
    state.update(&mut ctx(&mut app));
    assert!(state.folder_pickers.download.is_none(), "the screen kept waiting after the picker answered");
    assert_eq!(app.shared.config.library.download_folder.as_deref(), Some("/downloads/charts"));
    assert_eq!(app.shared.setting_line(SettingId::DownloadFolder).1, "/downloads/charts");

    let (tx, handle) = crate::dialog::handle_for_tests();
    state.folder_pickers.download = Some(handle);
    tx.send(None).expect("the handle is still held");
    state.update(&mut ctx(&mut app));
    assert!(state.folder_pickers.download.is_none(), "the screen kept waiting after the picker was dismissed");
    assert_eq!(app.shared.config.library.download_folder.as_deref(), Some("/downloads/charts"), "a dismissed picker changed the folder");

    state.run_action(&mut ctx(&mut app), SettingId::DownloadFolder, -1);
    assert_eq!(app.shared.config.library.download_folder, None, "a step to the left left the folder chosen");
    assert!(state.folder_pickers.download.is_none(), "a step to the left opened a picker");

    app.shared.config.library.download_folder = Some("/downloads/charts".to_string());
    assert!(state.reset_row(&mut ctx(&mut app), SettingId::DownloadFolder), "the zero key did not put the row back");
    assert_eq!(app.shared.config.library.download_folder, None);
    assert!(!state.reset_row(&mut ctx(&mut app), SettingId::DownloadFolder), "a row that ships as it is was said to have moved");
}

/// The row that lists the charts a table names and the library lacks is a switch: it ships on, a
/// step turns it off, and the zero key turns it back on.
#[test]
fn the_missing_charts_row_toggles() {
    let mut app = app();
    let mut state = SettingsState::on_tab(SettingTab::Select);
    let row = SettingRow::Fixed(SettingId::ShowMissingCharts);
    assert!(app.shared.config.library.show_missing_charts);
    assert_eq!(app.shared.setting_line(SettingId::ShowMissingCharts).1, "ON");

    state.step(&mut ctx(&mut app), row, 1, false);
    assert!(!app.shared.config.library.show_missing_charts);
    assert_eq!(app.shared.setting_line(SettingId::ShowMissingCharts).1, "OFF");

    assert!(state.reset_row(&mut ctx(&mut app), SettingId::ShowMissingCharts));
    assert!(app.shared.config.library.show_missing_charts, "the zero key left the charts hidden");
}

/// Names the folder the frames of the settings screen are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The two sizes the screen is drawn at: the one it is laid out for, and a full HD window.
const DRAWN_SIZES: [(u32, u32); 2] = [(CW, CH), (1920, 1080)];

/// How far above the button a press is clear of it and of the two buttons over it, on the part of
/// the rail nothing is drawn on, and how far inside the button's corner a pixel is read, clear of
/// its edge.
const ABOVE_THE_BUTTON: f32 = 200.0;
const INSIDE_THE_BUTTON: f32 = 2.0;

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
}

/// The middle of the button to the key editor.
fn on_the_key_config_button() -> (f32, f32) {
    let link = key_config_link();
    (link.x + link.w * 0.5, link.y + link.h * 0.5)
}

/// The key editor opens from the button under the one to the skin menu and from the key that
/// opens it from the song browser, and from neither while a list or an editor has the screen.
#[test]
fn the_key_editor_opens_from_its_button_and_from_its_key() {
    let mut app = app();
    let mut state = SettingsState::new();
    let at = on_the_key_config_button();
    assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Left, true), Transition::Open(Stage::KeyConfig(_))), "the button");
    assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Right, true), Transition::Stay), "another button of the mouse opened it");
    assert!(
        matches!(state.handle_mouse(&mut ctx(&mut app), (at.0, at.1 - ABOVE_THE_BUTTON), MouseButton::Left, true), Transition::Stay),
        "a press off the button"
    );
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(Shortcut::KeyConfig.default_key())), Transition::Open(Stage::KeyConfig(_))), "the key");

    state.rivals_open = true;
    assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Left, true), Transition::Stay), "the button under the rival list");
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(Shortcut::KeyConfig.default_key())), Transition::Stay), "the key under the rival list");
    state.rivals_open = false;
    state.text_input = Some(TextEdit::new());
    assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Left, true), Transition::Stay), "the button under an editor");
}

/// The keys that open the play menu, the skin menu and the key editor from this screen are the
/// ones the key configuration holds, and the line of keys at the foot of the screen names them as
/// they are now.
#[test]
fn the_keys_to_the_other_screens_are_read_from_the_key_configuration() {
    use crate::keyconfig::Bindable;
    let mut app = app();
    let mut state = SettingsState::new();
    let mode = app.shared.mode;
    assert!(state.scene(&app.shared, Instant::now()).hint.contains("F8 PLAY   F12 SKINS   F5 KEYS"), "{}", state.scene(&app.shared, Instant::now()).hint);
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F8)), Transition::Open(Stage::PlayMenu(_))), "the key the play menu ships on");
    app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::KeyConfig), Some(KeyCode::F6));
    app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::SkinMenu), Some(KeyCode::F7));
    app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::PlayMenu), Some(KeyCode::F10));
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F5)), Transition::Stay), "the key the editor shipped on still opens it");
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F12)), Transition::Stay), "the key the menu shipped on still opens it");
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F8)), Transition::Stay), "the key the play menu shipped on still opens it");
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F6)), Transition::Open(Stage::KeyConfig(_))));
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F7)), Transition::Open(Stage::SkinMenu(_))));
    assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F10)), Transition::Open(Stage::PlayMenu(_))));
    assert!(state.scene(&app.shared, Instant::now()).hint.contains("F10 PLAY   F7 SKINS   F6 KEYS"), "{}", state.scene(&app.shared, Instant::now()).hint);
    app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::PlayMenu), None);
    assert!(
        state.scene(&app.shared, Instant::now()).hint.contains("- PLAY"),
        "a menu left without a key is not named as one: {}",
        state.scene(&app.shared, Instant::now()).hint
    );
}

/// The screen draws the button to the key editor where a press is tested for it, at the size the
/// screen is laid out for and at full HD.
#[test]
fn the_button_to_the_key_editor_is_drawn_where_it_is_pressed() {
    use crate::stage::{Canvas, HeadlessCanvas};
    use rbms_render::Renderer;
    for size in DRAWN_SIZES {
        let mut app = app();
        let mut state = SettingsState::new();
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        state.draw(&mut ctx(&mut app), &mut Canvas::Headless(&mut pixels));
        if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
            let folder = std::path::Path::new(&folder);
            std::fs::create_dir_all(folder).expect("the capture folder can be created");
            let file = folder.join(format!("settings-key-config-button-{}x{}.png", size.0, size.1));
            image::save_buffer(file, pixels.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
        }
        let link = key_config_link();
        let scale = size.0 as f32 / CW as f32;
        let corner = pixels.pixel_at(((link.x + INSIDE_THE_BUTTON) * scale) as u32, ((link.y + INSIDE_THE_BUTTON) * scale) as u32);
        assert_eq!(corner, rbms_render::theme().panel_hi, "the button is not drawn where a press is tested for it");
        assert_eq!(pixels.size(), size);
    }
}
