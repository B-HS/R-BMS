//! The language the built-in screens are written in, and the catalogs of their fixed words.
//!
//! Each screen's words are a catalog of their own (`rbms_model::translations!`) in a module here,
//! read in the language of the [`crate::RenderCtx`] the screen is drawn with. The legacy entry
//! points that build their context from this thread's globals ([`crate::with_render_ctx`]) take the
//! language installed with [`set_language`], the way they take the theme installed with
//! [`crate::set_theme`].
//!
//! The installed language is per thread rather than one process-wide value, so tests running side
//! by side never see each other's, and a thread that installs none draws in English. A screen that
//! keeps words it composed for later frames remembers the language it composed them in and composes
//! them again once [`language`] answers another.

mod hud;
mod result;
mod select;

use std::cell::Cell;

use rbms_model::Language;

thread_local! {
    static LANGUAGE: Cell<Language> = const { Cell::new(Language::English) };
}

/// Install the language this thread's built-in screens are written in, from the next draw on.
pub fn set_language(language: Language) {
    LANGUAGE.with(|cell| cell.set(language));
}

/// The language this thread's built-in screens are written in: the one last installed with
/// [`set_language`], English until then.
pub fn language() -> Language {
    LANGUAGE.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thread_draws_in_english_until_another_language_is_installed() {
        let fresh = std::thread::spawn(language).join().expect("the thread ran");
        assert_eq!(fresh, Language::English);
        let installed = std::thread::spawn(|| {
            set_language(Language::Japanese);
            language()
        })
        .join()
        .expect("the thread ran");
        assert_eq!(installed, Language::Japanese);
    }

    #[test]
    fn the_installed_language_is_the_installing_threads_alone() {
        set_language(Language::Korean);
        let other = std::thread::spawn(language).join().expect("the thread ran");
        assert_eq!(other, Language::English, "another thread keeps its own");
        assert_eq!(language(), Language::Korean);
        set_language(Language::English);
    }
}
