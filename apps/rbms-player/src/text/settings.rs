//! Fixed words of the settings screen, of the option panel over the browser and of the rows they
//! share with the skin menu and the play menu, by language.
//!
//! The rows are named once in `rbms_config::SETTINGS`, in English, and the screen compares what it
//! reads there (a switch that is `ON`, a choice at one of its values, a row that is `(none)`). So
//! this file writes what the screen draws and never what it compares: [`row_label`] names a row,
//! [`shown_value`] writes what a row holds, and everything the screen tells a row apart by stays in
//! the table's English.

pub(crate) mod descriptions;
mod labels;
pub(crate) mod panel;
pub(crate) mod screen;
mod values;

pub(crate) use labels::{group_summary, other_heading, row_label, section_heading, tab_label};
pub(crate) use values::{shown_choice, shown_note, shown_value};

/// Installs a language on the test's thread and puts English back when the test is over, so that a
/// test which draws a screen in another language leaves the thread as the next test on it expects.
#[cfg(test)]
pub(crate) struct InstalledLanguage;

#[cfg(test)]
impl InstalledLanguage {
    pub(crate) fn new(language: rbms_model::Language) -> InstalledLanguage {
        rbms_render::text::set_language(language);
        InstalledLanguage
    }
}

#[cfg(test)]
impl Drop for InstalledLanguage {
    fn drop(&mut self) {
        rbms_render::text::set_language(rbms_model::Language::English);
    }
}
