//! Which language the player writes its screens in, settled from the DISPLAY tab's LANGUAGE row.
//!
//! A language the player chose is the language. AUTO asks the environment variable
//! [`LANGUAGE_ENV`] first, so a capture or a check can pick a language without touching the
//! settings, and then the system: the first of its preferred languages, as the text engine read it
//! when it was built ([`rbms_render::system_locale`]). A language the screens are not written in
//! comes to English.
//!
//! The settled language is installed on the frame thread ([`rbms_render::text::set_language`]),
//! which is what every built-in screen is drawn in. It is settled again as soon as the row changes,
//! whichever way it changed, in time for the frame that follows the change to be drawn in it. A
//! screen that keeps words it composed earlier compares the language it composed them in with
//! [`rbms_render::text::language`] and composes them again when the two differ.

use rbms_config::LanguageChoice;
use rbms_model::Language;

/// Environment variable naming the language AUTO comes to (`en`, `ko` or `ja`), ahead of the
/// system's own. A value that names no language the screens are written in is passed over.
pub(crate) const LANGUAGE_ENV: &str = "RBMS_LANGUAGE";

/// What the running player has settled the LANGUAGE row to.
#[derive(Debug, Default)]
pub(crate) struct AppLanguage {
    /// The row's value the installed language was settled from; `None` until the first frame.
    followed: Option<LanguageChoice>,
    /// What AUTO comes to on this host, read the first time AUTO is settled.
    host: Option<Language>,
}

impl AppLanguage {
    /// Settle the language for the LANGUAGE row's `choice` and install it, when the row changed
    /// since the last call. The host is asked only when AUTO is settled, and only once.
    pub(crate) fn follow(&mut self, choice: LanguageChoice) {
        if self.followed == Some(choice) {
            return;
        }
        self.followed = Some(choice);
        let language = match fixed(choice) {
            Some(language) => language,
            None => *self.host.get_or_insert_with(|| host_language(std::env::var(LANGUAGE_ENV).ok().as_deref(), &rbms_render::system_locale())),
        };
        rbms_render::text::set_language(language);
    }
}

/// The language a choice names, or `None` for AUTO.
fn fixed(choice: LanguageChoice) -> Option<Language> {
    match choice {
        LanguageChoice::Auto => None,
        LanguageChoice::English => Some(Language::English),
        LanguageChoice::Korean => Some(Language::Korean),
        LanguageChoice::Japanese => Some(Language::Japanese),
    }
}

/// What AUTO comes to on a host whose [`LANGUAGE_ENV`] holds `env` and whose system locale is
/// `system_locale`.
fn host_language(env: Option<&str>, system_locale: &str) -> Language {
    env.and_then(Language::from_code).unwrap_or_else(|| Language::from_locale(system_locale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chosen_language_is_the_language() {
        for (choice, expected) in
            [(LanguageChoice::English, Language::English), (LanguageChoice::Korean, Language::Korean), (LanguageChoice::Japanese, Language::Japanese)]
        {
            assert_eq!(fixed(choice), Some(expected));
        }
        assert_eq!(fixed(LanguageChoice::Auto), None);
    }

    #[test]
    fn auto_takes_the_variable_then_the_system_then_english() {
        assert_eq!(host_language(Some("ko"), "ja-JP"), Language::Korean, "the variable comes before the system");
        assert_eq!(host_language(Some("JA"), "en-US"), Language::Japanese);
        assert_eq!(host_language(Some("en"), "ko-KR"), Language::English);
        assert_eq!(host_language(Some("fr"), "ko-KR"), Language::Korean, "a variable naming no written language is passed over");
        assert_eq!(host_language(Some(""), "ja-JP"), Language::Japanese);
        assert_eq!(host_language(None, "ko-KR"), Language::Korean);
        assert_eq!(host_language(None, "en-KR"), Language::English, "a region never picks the language");
        assert_eq!(host_language(None, "zh-TW"), Language::English);
        assert_eq!(host_language(None, "fr"), Language::English);
        assert_eq!(host_language(None, ""), Language::English);
    }

    #[test]
    fn a_changed_row_is_installed_at_once_and_an_unchanged_one_is_left_alone() {
        let mut language = AppLanguage::default();
        language.follow(LanguageChoice::Korean);
        assert_eq!(rbms_render::text::language(), Language::Korean);
        language.follow(LanguageChoice::Japanese);
        assert_eq!(rbms_render::text::language(), Language::Japanese);

        rbms_render::text::set_language(Language::English);
        language.follow(LanguageChoice::Japanese);
        assert_eq!(rbms_render::text::language(), Language::English, "the row did not change, so nothing is installed again");
        assert_eq!(language.host, None, "a chosen language never asks the host");
    }

    #[test]
    fn auto_asks_the_host_once() {
        let mut language = AppLanguage { followed: None, host: Some(Language::Japanese) };
        language.follow(LanguageChoice::Auto);
        assert_eq!(rbms_render::text::language(), Language::Japanese, "the host's language as it was first read");
        language.follow(LanguageChoice::English);
        language.follow(LanguageChoice::Auto);
        assert_eq!(rbms_render::text::language(), Language::Japanese);
        rbms_render::text::set_language(Language::English);
    }
}
