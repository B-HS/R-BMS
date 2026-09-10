//! The transient message strip every screen can show over itself.
//!
//! The player collects what would otherwise only have reached the terminal and hands the live ones
//! here; this file is only the drawing, so the queue's lifetime rules stay with the program that
//! owns the clock.

use crate::ctx::{RenderCtx, with_render_ctx};
use crate::theme::theme;
use crate::{Color, Rect, Renderer};

/// How loud a message is, which decides the colour of its bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Warn,
    Error,
}

/// One message on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastView {
    pub level: ToastLevel,
    pub text: String,
}

/// Text scale of a message.
const TOAST_SCALE: f32 = 1.3;

/// Height of one message strip.
const TOAST_H: f32 = 30.0;

/// Vertical gap between two message strips.
const TOAST_GAP: f32 = 6.0;

/// Space either side of the text inside a strip.
const TOAST_PAD_X: f32 = 14.0;

/// Width of the colour bar down the leading edge of a strip.
const TOAST_BAR_W: f32 = 4.0;

/// How far the stack sits from the right edge of the screen.
const TOAST_MARGIN_X: f32 = 16.0;

/// How far the stack sits from the bottom of the screen.
const TOAST_MARGIN_Y: f32 = 16.0;

/// Opacity of the strip behind the text.
const TOAST_BG_ALPHA: u8 = 220;

/// The colour a level's bar is drawn in.
pub fn toast_color(level: ToastLevel) -> Color {
    match level {
        ToastLevel::Info => theme().accent,
        ToastLevel::Warn => Color::ORANGE,
        ToastLevel::Error => Color::RED,
    }
}

/// The widest a strip's text may be before it is cut, on a screen `screen_w` wide.
///
/// A strip is laid out from its own text and right-aligned, so a message wider than the screen
/// would start off the left edge and take its readable half with it. What is reported is still cut
/// short of the edge by the same margin the stack keeps on the right.
fn toast_text_limit(screen_w: f32) -> f32 {
    (screen_w - TOAST_MARGIN_X * 2.0 - TOAST_BAR_W - TOAST_PAD_X * 2.0).max(0.0)
}

/// One message's text as it is drawn: cut to what the screen holds.
fn toast_text(ctx: &mut RenderCtx<'_>, toast: &ToastView, screen_w: f32) -> String {
    ctx.fit_text(&toast.text, TOAST_SCALE, toast_text_limit(screen_w))
}

/// Width one message needs, so the stack can be right-aligned without measuring twice.
fn toast_width(ctx: &mut RenderCtx<'_>, text: &str) -> f32 {
    TOAST_BAR_W + TOAST_PAD_X * 2.0 + ctx.text_width(text, TOAST_SCALE)
}

/// Draw the live messages bottom-right, newest at the bottom. Returns the height the stack took, so
/// a caller can keep something else clear of it.
pub fn render_toasts<R: Renderer>(r: &mut R, toasts: &[ToastView]) -> f32 {
    with_render_ctx(|ctx| render_toasts_ctx(ctx, r, toasts))
}

/// [`render_toasts`] against an already-borrowed text context.
pub fn render_toasts_ctx<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, toasts: &[ToastView]) -> f32 {
    if toasts.is_empty() {
        return 0.0;
    }
    let (screen_w, screen_h) = r.size();
    let mut bottom = screen_h as f32 - TOAST_MARGIN_Y;
    for toast in toasts.iter().rev() {
        let text = toast_text(ctx, toast, screen_w as f32);
        let w = toast_width(ctx, &text);
        let x = screen_w as f32 - TOAST_MARGIN_X - w;
        let y = bottom - TOAST_H;
        let accent = toast_color(toast.level);
        r.fill_rect(Rect::new(x, y, w, TOAST_H), Color { r: 0, g: 0, b: 0, a: TOAST_BG_ALPHA });
        r.fill_rect(Rect::new(x, y, TOAST_BAR_W, TOAST_H), accent);
        ctx.draw_text(r, x + TOAST_BAR_W + TOAST_PAD_X, y + (TOAST_H - TOAST_SCALE * 8.0) * 0.5, TOAST_SCALE, accent, &text);
        bottom = y - TOAST_GAP;
    }
    screen_h as f32 - TOAST_MARGIN_Y - bottom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CpuCanvas;

    fn blank() -> CpuCanvas {
        crate::font::use_embedded_fonts_only();
        CpuCanvas::new(1280, 720)
    }

    fn painted(canvas: &CpuCanvas) -> usize {
        canvas.pixels().chunks_exact(4).filter(|p| p[3] != 0).count()
    }

    #[test]
    fn nothing_is_drawn_and_no_room_is_taken_when_there_is_nothing_to_say() {
        let mut canvas = blank();
        assert_eq!(render_toasts(&mut canvas, &[]), 0.0);
        assert_eq!(painted(&canvas), 0);
    }

    #[test]
    fn a_message_is_drawn_and_reports_the_room_it_took() {
        let mut canvas = blank();
        let took = render_toasts(&mut canvas, &[ToastView { level: ToastLevel::Error, text: "chart not found".into() }]);
        assert!(took >= TOAST_H, "one strip is at least its own height: {took}");
        assert!(painted(&canvas) > 0, "the strip has to be visible");
    }

    #[test]
    fn each_extra_message_stacks_a_strip_higher() {
        let mut canvas = blank();
        let one = render_toasts(&mut canvas, &[ToastView { level: ToastLevel::Info, text: "one".into() }]);
        let mut canvas = blank();
        let two =
            render_toasts(&mut canvas, &[ToastView { level: ToastLevel::Info, text: "one".into() }, ToastView { level: ToastLevel::Warn, text: "two".into() }]);
        assert!((two - one - TOAST_H - TOAST_GAP).abs() < 0.01, "one strip and a gap taller: {one} then {two}");
    }

    /// A failure quoting a long path must not start off the left edge of the screen, which is where
    /// a strip laid out from an uncut message would put it.
    #[test]
    fn a_message_wider_than_the_screen_is_cut_to_fit_inside_it() {
        let mut canvas = blank();
        let (screen_w, _) = canvas.size();
        let long = ToastView { level: ToastLevel::Error, text: "/songs/".to_string() + &"very-long-directory-name/".repeat(40) + "chart.bms" };
        render_toasts(&mut canvas, std::slice::from_ref(&long));
        let cut = with_render_ctx(|ctx| toast_text(ctx, &long, screen_w as f32));
        let drawn = with_render_ctx(|ctx| toast_width(ctx, &cut));
        assert!(drawn <= screen_w as f32 - TOAST_MARGIN_X * 2.0, "the strip is {drawn} wide on a {screen_w} screen");
        assert!(drawn > 0.0);
    }

    #[test]
    fn a_message_that_already_fits_is_drawn_as_it_was_reported() {
        let short = ToastView { level: ToastLevel::Info, text: "scanned 1204 charts".into() };
        let kept = with_render_ctx(|ctx| toast_text(ctx, &short, 1280.0));
        assert_eq!(kept, short.text, "cutting a message that fits would change every strip on screen");
    }

    /// A window narrower than one strip's own padding leaves no room for text at all; asking for a
    /// negative width would be a panic rather than an empty strip.
    #[test]
    fn a_screen_with_no_room_for_text_asks_for_none() {
        assert_eq!(toast_text_limit(0.0), 0.0);
        assert!(toast_text_limit(1280.0) > 0.0);
    }

    #[test]
    fn the_three_levels_are_told_apart_by_colour() {
        let colors = [toast_color(ToastLevel::Info), toast_color(ToastLevel::Warn), toast_color(ToastLevel::Error)];
        assert_ne!(colors[0], colors[1]);
        assert_ne!(colors[1], colors[2]);
        assert_ne!(colors[0], colors[2]);
    }
}
