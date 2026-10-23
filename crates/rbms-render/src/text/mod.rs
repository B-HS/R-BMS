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

pub(crate) mod hud;
pub(crate) mod result;
pub(crate) mod select;

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

/// Drawing a screen in a chosen language, for the tests of the screens that read the catalogs.
///
/// The test engine holds the bundled font alone, which has no Korean or Japanese letters, so a
/// screen in either language cannot be checked by its glyphs. What can be checked is that the words
/// the screen is drawn with are the language's: a region that holds a fixed word of the catalog
/// paints differently in another language than in English.
#[cfg(test)]
pub(crate) mod testing {
    use rbms_model::Language;

    use crate::font::TextContext;
    use crate::theme::Theme;
    use crate::{CpuCanvas, Rect, RenderCtx};

    /// A blank canvas of `width` x `height` that `draw` paints through a context writing in `language`.
    pub(crate) fn drawn_in(language: Language, width: u32, height: u32, draw: impl FnOnce(&mut RenderCtx<'_>, &mut CpuCanvas)) -> CpuCanvas {
        let mut text = TextContext::embedded_only();
        let mut ctx = RenderCtx::new(Theme::default(), &mut text).with_language(language);
        let mut canvas = CpuCanvas::new(width, height);
        draw(&mut ctx, &mut canvas);
        canvas
    }

    /// Whether the two canvases paint any pixel of `rect` differently.
    pub(crate) fn differs_within(a: &CpuCanvas, b: &CpuCanvas, rect: Rect) -> bool {
        let (x0, y0) = (rect.x as u32, rect.y as u32);
        (y0..y0 + rect.h as u32).any(|y| (x0..x0 + rect.w as u32).any(|x| a.pixel_at(x, y) != b.pixel_at(x, y)))
    }
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
