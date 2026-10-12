//! The key that puts a row of the previewed play skin onto the other screens: the five play modes
//! are drawn by five documents that declare most of their options under the same names, and a
//! player who sets one of them means it for every mode far more often than not.
//!
//! The menu is opened over the pack this repository ships.

use super::*;
use crate::stage::skin_menu::APPLY_TO_ALL_KEY;

impl Menu {
    /// What the play skin of another mode reads for the row called `label`.
    fn option_of_mode(&self, screen: i32, label: &str) -> String {
        let (skins, config) = (&self.app.shared.skins, &self.app.shared.config);
        let row = skins.rows_for(config, screen).into_iter().find(|row| skins.line_for(config, screen, *row).0 == label);
        skins.line_for(config, screen, row.unwrap_or_else(|| panic!("the play skin of screen {screen} has no {label}"))).1
    }
}

/// An option every play mode's skin declares says so under the preview, and the key puts the value
/// it stands on here onto the skins of the other modes, which no step of the row alone does. The
/// settings file is written. On a row that is not a skin's the key does nothing.
#[test]
fn the_key_puts_an_option_of_the_play_skin_onto_the_other_modes() {
    let mut menu = Menu::over(app_with_shipped_pack("apply-all"));
    let option = menu.skin_option(LANE_WIDTH_OPTION);
    let others: Vec<i32> = PREVIEW_MODES[1..].iter().map(|(screen, _)| *screen).collect();
    let note = menu.state.peers_note(&menu.app.shared, option).expect("the other modes declare the option");
    assert!(note.starts_with("ALSO ON ") && note.contains(" OTHER SCREENS"), "{note}");
    assert_eq!(menu.state.peers_note(&menu.app.shared, Row::Setting(SettingId::JudgeOffset)), None);

    menu.focus(option);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.line(option).value, "WIDE");
    for screen in &others {
        assert_eq!(menu.option_of_mode(*screen, LANE_WIDTH_OPTION), "NORMAL", "stepping one mode's option moved another's");
    }
    menu.frame();
    menu.save("play-menu-apply-note");

    menu.focus_setting(SettingId::JudgeOffset);
    let before = menu.app.shared.config.clone();
    menu.key(APPLY_TO_ALL_KEY);
    assert_eq!(menu.app.shared.config, before, "the key changed something from a row that is not a skin's");

    let _ = std::fs::remove_file(&menu.app.shared.settings_path);
    menu.focus(option);
    assert!(matches!(menu.key(APPLY_TO_ALL_KEY), Transition::Stay));
    for screen in &others {
        assert_eq!(menu.option_of_mode(*screen, LANE_WIDTH_OPTION), "WIDE", "the option was not put onto the play skin of screen {screen}");
    }
    assert_eq!(menu.line(option).value, "WIDE");
    let written = rbms_config::load(&menu.app.shared.settings_path).expect("the key wrote the settings file").config;
    assert_eq!(written.skin, menu.app.shared.config.skin, "what was put onto the other modes is not in the settings file");

    menu.key(KeyCode::Tab);
    assert_eq!(menu.line(menu.skin_option(LANE_WIDTH_OPTION)).value, "WIDE", "the next mode's own row does not show what was put onto it");
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "the next mode's preview did not come back with the option moved");
}
