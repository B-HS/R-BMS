//! What the settings screen still has to do with the options of a sitting: nothing but the way to
//! the play menu.
//!
//! How fast the notes scroll, what the lane is covered with, when a press counts and how a chart is
//! dealt out were rows of this screen and are now the play menu's
//! ([`crate::stage::PlayMenuState`]), whose own tests hold what each of its rows does. What is held
//! here is that no tab lists one of them, that the rows that stayed are where they were said to go,
//! that the key and the button open the menu over this screen, and that the menu comes back to it.

use rbms_config::tab_rows;

use super::skin_tests::{app_in, click, ctx, draw, key, open_tab};
use super::tests::press;
use super::*;
use crate::stage::StageId;
use crate::stage::select::PLAY_MENU_KEY;

/// No tab lists a row of the play menu's, the rows that stayed behind are on the tabs they were
/// moved to, and a screen asked to open on a group that is the play menu's opens on its first tab.
#[test]
fn no_tab_lists_a_row_of_the_play_menus() {
    let mut app = app_in("play-rows");
    let moved: Vec<SettingRow> = SettingTab::PLAY_MENU.iter().flat_map(|group| tab_rows(*group, &app.shared.config)).map(SettingRow::Fixed).collect();
    assert!(moved.contains(&SettingRow::Fixed(SettingId::HiSpeed)) && moved.contains(&SettingRow::Fixed(SettingId::JudgeOffset)));
    for tab in SettingTab::SHOWN {
        let mut state = SettingsState::on_tab(tab);
        state.on_enter(&mut ctx(&mut app));
        assert!(state.rows.iter().all(|row| !moved.contains(row)), "{tab:?} lists a row of the play menu's");
    }
    for group in SettingTab::PLAY_MENU {
        assert_eq!(SettingsState::on_tab(group).current_tab(), SettingTab::SHOWN[0], "{group:?}");
    }
    let on = |tab: SettingTab, id: SettingId| app.shared.settings_rows(tab).contains(&SettingRow::Fixed(id));
    assert!(on(SettingTab::Display, SettingId::FiveKeyLayout), "the layout of a five-key chart is not on the DISPLAY tab");
    assert!(on(SettingTab::Judge, SettingId::Total), "the gauge total is not on the JUDGE tab");
    assert!(on(SettingTab::Judge, SettingId::JudgeAlgorithm) && on(SettingTab::Judge, SettingId::GaugeSet), "a rule of the judge left the JUDGE tab");
    assert!(on(SettingTab::Input, SettingId::ScratchSide) && on(SettingTab::Input, SettingId::KeyConfig));
}

/// The key opens the play menu over the settings screen, from whichever tab is open, and Escape on
/// the menu comes back to the settings screen on the tab and the row it was left on.
#[test]
fn the_key_opens_the_play_menu_and_escape_comes_back_to_the_settings() {
    let mut app = app_in("play-key");
    app.switch(Transition::Open(Stage::Settings(SettingsState::on_tab(SettingTab::Display))));
    key(&mut app, KeyCode::ArrowDown);
    key(&mut app, KeyCode::ArrowDown);
    let before = draw(&mut app).pixel_checksum();

    key(&mut app, PLAY_MENU_KEY);
    assert_eq!(app.stage.id(), StageId::PlayMenu, "the key did not open the play menu");
    assert_eq!(app.suspended.len(), 2, "the settings screen and the browser are not both parked under the menu");

    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Settings, "the menu did not come back to the settings screen");
    assert_eq!(open_tab(&app), SettingTab::Display, "the settings screen came back on another tab");
    assert_eq!(draw(&mut app).pixel_checksum(), before, "the settings screen came back looking different");

    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Select);
    assert!(app.suspended.is_empty());
}

/// A value the play menu changed is what the run reads when the menu was reached through the
/// settings screen: the two screens write one configuration.
#[test]
fn what_the_play_menu_changed_is_in_the_configuration_the_settings_screen_shares() {
    let mut app = app_in("play-shared");
    app.switch(Transition::Open(Stage::Settings(SettingsState::new())));
    key(&mut app, PLAY_MENU_KEY);
    key(&mut app, KeyCode::ArrowDown);
    key(&mut app, KeyCode::ArrowRight);
    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Settings);
    assert!(app.shared.config.play.keep_green_number, "the row stepped on the play menu is not in the configuration");
    let stored = rbms_config::load(&app.shared.settings_path).expect("the settings file the menu wrote is read").config;
    assert!(stored.play.keep_green_number, "leaving the play menu did not write what it changed");
}

/// The button at the foot of the rail opens the play menu the way the key does, and stands over the
/// one to the skin menu and under the last group without touching either.
#[test]
fn the_button_on_the_rail_opens_the_play_menu() {
    let mut app = app_in("play-button");
    app.switch(Transition::Open(Stage::Settings(SettingsState::new())));
    draw(&mut app);
    let rect_of = |app: &App, hot: Hot| {
        let found: Vec<_> = app.shared.hot.iter().filter(|(_, entry)| *entry == hot).map(|(rect, _)| *rect).collect();
        assert_eq!(found.len(), 1, "{hot:?} is not on the screen once");
        found[0]
    };
    let (play, skin) = (rect_of(&app, Hot::PlayMenuLink), rect_of(&app, Hot::SkinMenuLink));
    assert!(play.y + play.h < skin.y, "the two buttons meet: {play:?} and {skin:?}");
    let last_tab = rect_of(&app, Hot::SettingTab(SettingTab::SHOWN.len() - 1));
    assert!(last_tab.y + last_tab.h < play.y, "the last group runs under the button: {last_tab:?} and {play:?}");

    click(&mut app, Hot::PlayMenuLink);
    assert_eq!(app.stage.id(), StageId::PlayMenu, "the button did not open the play menu");
    key(&mut app, KeyCode::Escape);
    assert_eq!(app.stage.id(), StageId::Settings);
}

/// While a line is being typed into a row, or one of the inline lists is up, the key and the button
/// are that editor's or that list's and open nothing.
#[test]
fn the_way_to_the_play_menu_is_shut_while_an_editor_or_a_list_is_open() {
    let mut app = app_in("play-shut");
    let mut editing = SettingsState::on_tab(SettingTab::Network);
    editing.on_enter(&mut ctx(&mut app));
    editing.sel = editing.rows.iter().position(|row| *row == SettingRow::Fixed(SettingId::Email)).expect("the NETWORK tab has an EMAIL row");
    editing.row_key(&mut ctx(&mut app), &press(KeyCode::Enter));
    assert!(editing.text_input.is_some(), "the fixture needs a row being typed into");
    assert!(matches!(editing.handle_key(&mut ctx(&mut app), press(PLAY_MENU_KEY)), Transition::Stay), "the key opened the menu from under an editor");
    assert!(editing.text_input.is_some(), "the key closed the editor");
    app.stage = Stage::Settings(editing);
    draw(&mut app);
    click(&mut app, Hot::PlayMenuLink);
    assert_eq!(app.stage.id(), StageId::Settings, "the button opened the menu from under an editor");
    assert!(matches!(&app.stage, Stage::Settings(state) if state.text_input.is_some()), "the button closed the editor");

    for list in [SettingId::Rivals, SettingId::IrProfiles] {
        let mut state = SettingsState::on_tab(SettingTab::Network);
        state.on_enter(&mut ctx(&mut app));
        state.sel = state.rows.iter().position(|row| *row == SettingRow::Fixed(list)).unwrap_or_else(|| panic!("the NETWORK tab has no {list:?} row"));
        state.row_key(&mut ctx(&mut app), &press(KeyCode::Enter));
        assert!(state.rivals_open || state.profiles_open, "{list:?} did not open its list");
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(PLAY_MENU_KEY)), Transition::Stay), "the key opened the menu from under {list:?}");

        app.stage = Stage::Settings(state);
        draw(&mut app);
        click(&mut app, Hot::PlayMenuLink);
        assert_eq!(app.stage.id(), StageId::Settings, "the button opened the menu from under {list:?}");
    }
}
