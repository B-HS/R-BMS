//! What the settings screen still has to do with skins: nothing but the way to the skin menu.
//!
//! The skin settings were a tab of this screen and are now the skin menu's
//! ([`crate::stage::SkinMenuState`]), whose own tests hold what each of its rows does. What is held
//! here is that the tab is gone from the strip, that the key and the button in its place open the
//! menu over this screen, and that the menu comes back to it.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, every tab is also saved there as `settings-<tab>.png`,
//! which is how the strip with its button is looked at.

use std::path::PathBuf;

use super::tests::press;
use super::*;
use crate::settings_view::{settings_hint, visible_rows};
use crate::stage::render_tests::app;
use crate::stage::select::SKIN_MENU_KEY;
use crate::stage::{Canvas, HeadlessCanvas, StageId};

/// Environment variable naming the folder captures are saved in, the one the screen capture harness
/// reads. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
}

/// An app whose settings file is in a folder of this test's own, so a screen that writes it on the
/// way out writes nothing another test reads.
fn app_in(tag: &str) -> App {
    let mut app = app();
    let root = std::env::temp_dir().join(format!("rbms-settings-skin-link-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the fixture folder is writable");
    app.shared.settings_path = root.join("settings.ron");
    app.shared.config.skin = rbms_config::SkinOptions::default();
    app.shared.skins = crate::skin_select::SkinLibrary::new(&app.shared.settings_path, &app.shared.config);
    app
}

/// Draw the screen that is up onto a headless canvas, with the app's clickable regions rebuilt for
/// the frame.
fn draw(app: &mut App) -> HeadlessCanvas {
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let mut canvas = Canvas::Headless(&mut pixels);
    app.shared.hot.clear();
    let now = std::time::Instant::now();
    app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut canvas);
    pixels
}

/// Hand one key to the screen that is up and make the change of screen it asks for.
fn key(app: &mut App, code: KeyCode) {
    let now = std::time::Instant::now();
    let asked = app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(code));
    app.switch(asked);
}

/// Press the left button in the middle of what the frame drawn last made clickable for `hot`, and
/// make the change of screen that asks for.
fn click(app: &mut App, hot: Hot) {
    let (rect, _) = *app.shared.hot.iter().find(|(_, entry)| *entry == hot).unwrap_or_else(|| panic!("{hot:?} is not on the screen"));
    let at = (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
    let now = std::time::Instant::now();
    let input = crate::pointer::PointerInput::Button { button: MouseButton::Left, pressed: true };
    let asked = app.stage.handle_pointer(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, at, input);
    app.switch(asked);
}

/// The tab the settings screen that is up is on.
fn open_tab(app: &App) -> SettingTab {
    match &app.stage {
        Stage::Settings(state) => state.current_tab(),
        other => panic!("{other:?} is up, not the settings screen"),
    }
}

/// The strip has a tab for every group of rows but the skin menu's, and Tab walks them all and
/// comes back round without ever landing on that one.
#[test]
fn the_tab_strip_has_no_skin_tab_and_tab_walks_the_tabs_it_has() {
    let mut app = app_in("strip");
    app.switch(Transition::Open(Stage::Settings(SettingsState::new())));
    draw(&mut app);
    let tabs = app.shared.hot.iter().filter(|(_, hot)| matches!(hot, Hot::SettingTab(_))).count();
    assert_eq!(tabs, SettingTab::SHOWN.len());
    assert!(!SettingTab::SHOWN.contains(&SettingTab::Skin));

    let mut walked = vec![open_tab(&app)];
    for _ in 1..SettingTab::SHOWN.len() {
        key(&mut app, KeyCode::Tab);
        walked.push(open_tab(&app));
    }
    assert_eq!(walked, SettingTab::SHOWN.to_vec());
    key(&mut app, KeyCode::Tab);
    assert_eq!(open_tab(&app), SettingTab::SHOWN[0], "Tab does not come back round to the first tab");

    for (place, tab) in SettingTab::SHOWN.iter().enumerate() {
        draw(&mut app);
        click(&mut app, Hot::SettingTab(place));
        assert_eq!(open_tab(&app), *tab, "a press on the tab at place {place}");
    }
}

/// No tab lists a row of the skin menu's, and a screen asked to open on the tab that is gone opens
/// on its first.
#[test]
fn no_tab_lists_a_row_of_the_skin_menus() {
    let mut app = app_in("rows");
    let skin_rows = app.shared.settings_rows(SettingTab::Skin);
    assert!(!skin_rows.is_empty());
    for tab in SettingTab::SHOWN {
        let mut state = SettingsState::on_tab(tab);
        state.on_enter(&mut ctx(&mut app));
        assert_eq!(state.current_tab(), tab);
        assert!(state.rows.iter().all(|row| !skin_rows.contains(row)), "{tab:?} lists a row of the skin menu's");
    }
    assert_eq!(SettingsState::on_tab(SettingTab::Skin).current_tab(), SettingTab::SHOWN[0]);
    let display = app.shared.settings_rows(SettingTab::Display);
    assert!(display.contains(&SettingRow::Fixed(SettingId::Skin)), "the layout of the built-in note field left the DISPLAY tab");
}

/// The key opens the skin menu over the settings screen, from whichever tab is open, and Escape on
/// the menu comes back to the settings screen on the tab and the row it was left on.
#[test]
fn the_key_opens_the_skin_menu_and_escape_comes_back_to_the_settings() {
    let mut app = app_in("key");
    app.switch(Transition::Open(Stage::Settings(SettingsState::on_tab(SettingTab::Display))));
    key(&mut app, KeyCode::ArrowDown);
    key(&mut app, KeyCode::ArrowDown);
    let before = draw(&mut app).pixel_checksum();

    key(&mut app, SKIN_MENU_KEY);
    assert_eq!(app.stage.id(), StageId::SkinMenu, "the key did not open the skin menu");
    assert_eq!(app.suspended.len(), 2, "the settings screen and the browser are not both parked under the menu");

    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Settings, "the menu did not come back to the settings screen");
    assert_eq!(open_tab(&app), SettingTab::Display, "the settings screen came back on another tab");
    assert_eq!(draw(&mut app).pixel_checksum(), before, "the settings screen came back looking different");

    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Select);
    assert!(app.suspended.is_empty());
}

/// The button at the end of the tab strip opens the skin menu the way the key does.
#[test]
fn the_button_on_the_tab_strip_opens_the_skin_menu() {
    let mut app = app_in("button");
    app.switch(Transition::Open(Stage::Settings(SettingsState::new())));
    draw(&mut app);
    assert_eq!(app.shared.hot.iter().filter(|(_, hot)| *hot == Hot::SkinMenuLink).count(), 1, "the button is not on the screen once");
    click(&mut app, Hot::SkinMenuLink);
    assert_eq!(app.stage.id(), StageId::SkinMenu, "the button did not open the skin menu");
    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Settings);
}

/// Every drawn row, every tab and the button are clickable, and nothing else is.
#[test]
fn every_row_every_tab_and_the_button_are_clickable() {
    let mut app = app_in("regions");
    for tab in SettingTab::SHOWN {
        let rows = app.shared.settings_rows(tab).len();
        app.stage = Stage::Settings(SettingsState::on_tab(tab));
        draw(&mut app);
        assert_eq!(app.shared.hot.len(), rows.min(visible_rows()) + SettingTab::SHOWN.len() + 1, "{tab:?}");
    }
}

/// While a line is being typed into a row, or one of the inline lists is up, the key and the button
/// are that editor's or that list's and open nothing.
#[test]
fn the_way_to_the_skin_menu_is_shut_while_an_editor_or_a_list_is_open() {
    let mut app = app_in("shut");
    let mut editing = SettingsState::on_tab(SettingTab::Network);
    editing.on_enter(&mut ctx(&mut app));
    editing.sel = editing.rows.iter().position(|row| *row == SettingRow::Fixed(SettingId::Email)).expect("the NETWORK tab has an EMAIL row");
    editing.row_key(&mut ctx(&mut app), &press(KeyCode::Enter));
    assert!(editing.text_input.is_some(), "the fixture needs a row being typed into");
    assert!(matches!(editing.handle_key(&mut ctx(&mut app), press(SKIN_MENU_KEY)), Transition::Stay), "the key opened the menu from under an editor");
    assert!(editing.text_input.is_some(), "the key closed the editor");
    app.stage = Stage::Settings(editing);
    draw(&mut app);
    click(&mut app, Hot::SkinMenuLink);
    assert_eq!(app.stage.id(), StageId::Settings, "the button opened the menu from under an editor");
    assert!(matches!(&app.stage, Stage::Settings(state) if state.text_input.is_some()), "the button closed the editor");

    for list in [SettingId::Rivals, SettingId::IrProfiles] {
        let mut state = SettingsState::on_tab(SettingTab::Network);
        state.on_enter(&mut ctx(&mut app));
        state.sel = state.rows.iter().position(|row| *row == SettingRow::Fixed(list)).unwrap_or_else(|| panic!("the NETWORK tab has no {list:?} row"));
        state.row_key(&mut ctx(&mut app), &press(KeyCode::Enter));
        assert!(state.rivals_open || state.profiles_open, "{list:?} did not open its list");
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(SKIN_MENU_KEY)), Transition::Stay), "the key opened the menu from under {list:?}");

        app.stage = Stage::Settings(state);
        draw(&mut app);
        click(&mut app, Hot::SkinMenuLink);
        assert_eq!(app.stage.id(), StageId::Settings, "the button opened the menu from under {list:?}");
    }
}

/// The hint under the title names the key, and the key is the one that opens the menu from the song
/// browser.
#[test]
fn the_hint_names_the_key_that_opens_the_menu() {
    assert_eq!(SKIN_MENU_KEY, KeyCode::F12);
    let hint = settings_hint(crate::keyconfig::key_name(SKIN_MENU_KEY), "F5");
    assert!(hint.contains("F12 SKINS"), "{hint}");
}

/// Every tab of the screen, drawn and saved for a person to look at when a capture folder was named.
#[test]
fn every_tab_draws_and_can_be_captured() {
    let mut app = app_in("capture");
    for tab in SettingTab::SHOWN {
        app.stage = Stage::Settings(SettingsState::on_tab(tab));
        let pixels = draw(&mut app);
        assert!(pixels.painted_pixels() > 0, "{tab:?} drew nothing");
        let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV).map(PathBuf::from) else {
            continue;
        };
        std::fs::create_dir_all(&folder).expect("the capture folder can be created");
        let path = folder.join(format!("settings-{}.png", tab.label().to_ascii_lowercase()));
        image::save_buffer(&path, pixels.rgba(), CW, CH, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}
