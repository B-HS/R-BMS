//! Render context — the palette and text engine a screen composer draws with, passed in as an
//! argument instead of read from a global.
//!
//! Every composed screen has two forms: a `*_ctx` entry point taking a [`RenderCtx`], and a legacy
//! wrapper of the same name minus the suffix that builds one from this thread's
//! [`crate::theme::set_theme`] palette and shared [`TextContext`] via [`with_render_ctx`]. New code
//! should take a context; the wrappers exist so the player keeps compiling while it migrates.

use crate::font::TextContext;
use crate::theme::Theme;
use crate::{Color, Renderer};

/// The inputs every screen composer needs: the resolved UI palette and the text engine to shape and
/// rasterize with. Cheap to build — the palette is a `Copy` struct and the engine is borrowed.
pub struct RenderCtx<'a> {
    pub theme: Theme,
    pub text: &'a mut TextContext,
}

impl<'a> RenderCtx<'a> {
    pub fn new(theme: Theme, text: &'a mut TextContext) -> Self {
        RenderCtx { theme, text }
    }

    /// Draw a left-aligned string with its top-left near `(x, y)`.
    pub fn draw_text<R: Renderer>(&mut self, r: &mut R, x: f32, y: f32, scale: f32, color: Color, text: &str) {
        self.text.draw_text(r, x, y, scale, color, text);
    }

    /// Draw a string horizontally centred on `center_x`.
    pub fn draw_text_centered<R: Renderer>(&mut self, r: &mut R, center_x: f32, y: f32, scale: f32, color: Color, text: &str) {
        self.text.draw_text_centered(r, center_x, y, scale, color, text);
    }

    /// Draw a string whose right edge lands on `right_x`.
    pub fn draw_text_right<R: Renderer>(&mut self, r: &mut R, right_x: f32, y: f32, scale: f32, color: Color, text: &str) {
        self.text.draw_text_right(r, right_x, y, scale, color, text);
    }

    /// Width in pixels the string occupies at `scale`.
    pub fn text_width(&mut self, text: &str, scale: f32) -> f32 {
        self.text.text_width(text, scale)
    }

    /// Ellipsis-truncate `text` to `max_width` px at `scale`.
    pub fn fit_text(&mut self, text: &str, scale: f32, max_width: f32) -> String {
        self.text.fit_text(text, scale, max_width)
    }
}

/// Build a context from this thread's installed theme and shared text engine and run `f` with it.
/// Backs every legacy no-context render entry point; `f` must not call a global text helper, since
/// the engine is borrowed for the whole closure.
pub fn with_render_ctx<T>(f: impl FnOnce(&mut RenderCtx<'_>) -> T) -> T {
    let theme = crate::theme::theme();
    crate::font::with_text_context(|text| f(&mut RenderCtx::new(theme, text)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{TextContext, use_embedded_fonts_only};
    use crate::theme::theme;
    use crate::{Color, CpuCanvas, ResultView, render_result, render_result_ctx};

    const W: u32 = 320;
    const H: u32 = 240;

    fn view() -> ResultView {
        ResultView {
            title: "CONTEXT".into(),
            counts: [10, 4, 3, 2, 1, 0],
            ex_score: 24,
            max_score: 40,
            max_combo: 12,
            total_notes: 20,
            fast: 3,
            slow: 2,
            gauge: 64.0,
            clear_label: "CLEAR",
            clear_color: Color::GREEN,
            prev_best_ex: Some(20),
            prev_ex: None,
            show_graph: true,
        }
    }

    /// The legacy no-context entry point is exactly the injected one fed this thread's globals, so
    /// migrating a call site to a context cannot move a pixel.
    #[test]
    fn the_legacy_wrapper_matches_an_explicitly_built_context() {
        use_embedded_fonts_only();
        let mut via_globals = CpuCanvas::new(W, H);
        render_result(&mut via_globals, &view());

        let mut text = TextContext::embedded_only();
        let mut ctx = RenderCtx::new(theme(), &mut text);
        let mut via_ctx = CpuCanvas::new(W, H);
        render_result_ctx(&mut ctx, &mut via_ctx, &view());

        assert_eq!(via_globals.pixels(), via_ctx.pixels(), "the wrapper only supplies the globals the context would carry");
    }

    /// The palette a screen paints with comes from the context argument, not from whatever theme is
    /// installed on the thread — that is the point of the injection.
    #[test]
    fn the_context_palette_is_used_instead_of_the_installed_theme() {
        use_embedded_fonts_only();
        let marker = Color::rgb(3, 251, 5);
        let mut text = TextContext::embedded_only();
        let mut ctx = RenderCtx::new(Theme { bg: marker, ..Theme::default() }, &mut text);
        let mut canvas = CpuCanvas::new(W, H);
        render_result_ctx(&mut ctx, &mut canvas, &view());

        assert_eq!(canvas.pixel_at(0, H - 1), marker, "the background is cleared with the context's palette");
        assert_eq!(theme(), Theme::default(), "rendering through a context leaves the installed theme untouched");
    }

    /// A context-owned text engine is independent of the thread-local one: text drawn through it is
    /// identical to the global helper's output at the same origin.
    #[test]
    fn context_text_matches_the_global_helper() {
        use_embedded_fonts_only();
        let mut via_global = CpuCanvas::new(W, 40);
        crate::font::draw_text(&mut via_global, 4.0, 4.0, 2.0, Color::WHITE, "INJECTED");

        let mut text = TextContext::embedded_only();
        let mut ctx = RenderCtx::new(Theme::default(), &mut text);
        let mut via_ctx = CpuCanvas::new(W, 40);
        ctx.draw_text(&mut via_ctx, 4.0, 4.0, 2.0, Color::WHITE, "INJECTED");

        assert_eq!(via_global.pixels(), via_ctx.pixels(), "an owned text engine rasterizes the same glyphs");
        assert!(ctx.text_width("INJECTED", 2.0) > 0.0, "the context measures text too");
    }
}
