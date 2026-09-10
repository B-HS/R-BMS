//! Shared machinery for the per-screen golden signature tests.
//!
//! Each screen's file renders a fixed view into a [`CpuCanvas`] and compares
//! `CpuCanvas::signature_hash(GOLDEN_COLS, GOLDEN_ROWS)` — a coarse, quantized block signature — to
//! a baked literal. That catches a moved panel, a dropped element or a recoloured row while
//! tolerating sub-pixel antialiasing differences.
//!
//! Every screen here is rendered through [`rbms_render::font::use_embedded_fonts_only`]. The
//! default text engine scans installed system fonts and falls back to them for codepoints the
//! bundled font lacks, so the same string rasterizes differently on each operating system and the
//! signature is host-specific. With only the bundled face loaded the render depends on nothing
//! outside this repository, so one literal holds on every host.
//!
//! Regenerating a golden after an intentional visual change: run the screen's own test (e.g.
//! `cargo test -p rbms-render --test golden_result`); the failing assertion prints the constant
//! line ready to paste over the one in that file. The screens are split one file per branch of the
//! work, so two people changing two screens never edit the same file.

use rbms_render::CpuCanvas;

/// Signature grid. 64x36 keeps the 16:9 reference aspect, so each block is a square 20x20 region.
/// A 16x9 grid was measured to be too coarse: an 80x80 block averaged a whole text run away, so a
/// changed EX score digit or a swapped clear label hashed identically. See the sensitivity tests.
pub const GOLDEN_COLS: u32 = 64;

/// Rows of the signature grid.
pub const GOLDEN_ROWS: u32 = 36;

/// Width of every golden screen.
pub const SCREEN_W: u32 = 1280;

/// Height of every golden screen.
pub const SCREEN_H: u32 = 720;

/// A blank screen-sized canvas whose text is rendered from the bundled font alone, so the result is
/// identical on every host. See the module docs.
pub fn golden_canvas() -> CpuCanvas {
    rbms_render::font::use_embedded_fonts_only();
    CpuCanvas::new(SCREEN_W, SCREEN_H)
}

/// The signature of one drawn frame.
pub fn signature_of(draw: impl FnOnce(&mut CpuCanvas)) -> u64 {
    let mut canvas = golden_canvas();
    draw(&mut canvas);
    canvas.signature_hash(GOLDEN_COLS, GOLDEN_ROWS)
}

/// The `const` line for one golden as it should read right now, so a failing run regenerates it.
pub fn regenerated_constant(name: &str, hash: u64) -> String {
    let (hi, lo) = ((hash >> 32) as u32, hash as u32);
    format!("const {name}: u64 = 0x{:04x}_{:04x}_{:04x}_{:04x};", hi >> 16, hi & 0xffff, lo >> 16, lo & 0xffff)
}

/// Compare a drawn canvas against its baked signature, naming the constant to edit when it moves.
pub fn check(canvas: &CpuCanvas, screen: &str, name: &str, expected: u64) {
    let actual = canvas.signature_hash(GOLDEN_COLS, GOLDEN_ROWS);
    assert_eq!(actual, expected, "{screen} golden signature changed; replace {name} with:\n{}", regenerated_constant(name, actual));
}
