//! The skin menu written in each language: what its rows read changes with the language and nothing it
//! decides by does. The fonts of the tests have no Korean or Japanese, so these read the words the menu
//! draws rather than the pixels.

use rbms_model::Language;
use rbms_model::translations::Catalog;

use super::*;
use crate::text::menus::skin_info::SkinInfoText;
use crate::text::settings::InstalledLanguage;

impl Menu {
    /// The line of the list that one row of the settings table is drawn from.
    fn line_of_fixed(&self, id: SettingId) -> &SettingsLine {
        &self.state.lines[self.at(SettingRow::Fixed(id))]
    }
}

/// A row that holds nothing says so in the language, and the value the menu compares stays the
/// table's own `(none)`.
#[test]
fn a_row_that_holds_nothing_is_drawn_in_the_language_and_compared_in_english() {
    for (language, shown) in [(Language::English, "(none)"), (Language::Korean, "(없음)"), (Language::Japanese, "(なし)")] {
        let _installed = InstalledLanguage::new(language);
        let menu = Menu::over(bare_app("language-empty"));
        let pack = menu.line_of_fixed(SettingId::SkinPack);
        assert_eq!((pack.value.as_str(), pack.shown.as_str()), ("(none)", shown), "{language:?}");
    }
}

/// The LOADED row of a screen with no document says what draws it in the language, and the menu
/// still notices the row moving by the words it compares, which are the same in every language.
#[test]
fn the_loaded_row_is_worded_in_the_language() {
    for language in Language::ALL {
        let _installed = InstalledLanguage::new(language);
        let menu = Menu::over(bare_app("language-loaded"));
        let loaded = menu.line_of_fixed(SettingId::SkinInfo);
        assert_eq!(loaded.shown, SkinInfoText::BuiltInScreen.get(language), "{language:?}");
        assert_eq!(loaded.value, loaded.shown, "{language:?}: the row is its own value");
    }
}
