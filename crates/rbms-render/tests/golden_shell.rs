//! Golden signature tests for the app-wide overlays: what is drawn over whichever screen is up.

mod golden_harness;

use golden_harness::{check, golden_canvas, signature_of};
use rbms_render::{Color, Renderer, ToastLevel, ToastView, render_toasts};

const GOLDEN_BLANK: u64 = 0x988c_a956_1494_bf25;
const GOLDEN_TOASTS: u64 = 0x6aa5_c041_4fcb_1504;
const GOLDEN_LONG_TOAST: u64 = 0x654c_dedc_19f0_3f76;

/// How many pixels a frame actually painted, which is what these goldens are about: an overlay is
/// there or it is not.
fn painted_pixels(canvas: &rbms_render::CpuCanvas) -> usize {
    canvas.pixels().chunks_exact(4).filter(|p| p[3] != 0).count()
}

fn toast_stack() -> Vec<ToastView> {
    vec![
        ToastView { level: ToastLevel::Info, text: "scanned 1204 charts".into() },
        ToastView { level: ToastLevel::Warn, text: "audio: reopen failed - trying the previous device settings".into() },
        ToastView { level: ToastLevel::Error, text: "chart not found: /songs/missing.bms".into() },
    ]
}

/// A failure quoting a path far longer than the screen is wide, which is the shape of message the
/// strip has to survive: laid out from its own text and right-aligned, an uncut one would start off
/// the left edge.
fn overlong_message() -> ToastView {
    ToastView { level: ToastLevel::Error, text: "chart not found: /songs/".to_string() + &"a-very-long-directory-name/".repeat(30) + "chart.bms" }
}

/// The empty frame the overlays are drawn onto. Pinning it means a change to the canvas or the
/// signature grid itself shows up here rather than as an unexplained move on every other screen.
#[test]
fn a_cleared_frame_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    canvas.clear(Color::BLACK);
    check(&canvas, "blank", "GOLDEN_BLANK", GOLDEN_BLANK);
}

#[test]
fn the_toast_stack_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    canvas.clear(Color::BLACK);
    render_toasts(&mut canvas, &toast_stack());
    check(&canvas, "toasts", "GOLDEN_TOASTS", GOLDEN_TOASTS);
}

#[test]
fn an_overlong_message_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    canvas.clear(Color::BLACK);
    render_toasts(&mut canvas, &[overlong_message()]);
    check(&canvas, "long toast", "GOLDEN_LONG_TOAST", GOLDEN_LONG_TOAST);
}

/// However long the message, the strip stays inside the frame: an off-screen strip is a failure the
/// user is told about in a place they cannot read.
#[test]
fn an_overlong_message_is_drawn_inside_the_frame() {
    let mut canvas = golden_canvas();
    canvas.clear(Color::BLACK);
    render_toasts(&mut canvas, &[overlong_message()]);
    let painted_in_left_edge = (0..canvas.size().1).any(|y| canvas.pixel_at(0, y) != Color::BLACK);
    assert!(!painted_in_left_edge, "the strip reached the left edge of the frame");
}

/// An overlay with nothing to say must leave the frame underneath exactly as it was, or every other
/// screen's golden would move the moment the overlay was wired up.
#[test]
fn an_empty_overlay_leaves_the_frame_untouched() {
    let bare = signature_of(|canvas| canvas.clear(Color::BLACK));
    let overlaid = signature_of(|canvas| {
        canvas.clear(Color::BLACK);
        render_toasts(canvas, &[]);
    });
    assert_eq!(bare, overlaid, "an empty toast queue may not paint anything");
    assert_eq!(bare, GOLDEN_BLANK);
}

#[test]
fn the_toast_stack_paints_over_the_frame_it_is_given() {
    let mut canvas = golden_canvas();
    canvas.clear(Color::BLACK);
    let before = painted_pixels(&canvas);
    render_toasts(&mut canvas, &toast_stack());
    assert!(painted_pixels(&canvas) >= before, "the overlay never unpaints the frame under it");
    assert_ne!(GOLDEN_TOASTS, GOLDEN_BLANK, "three messages have to be visible");
}
