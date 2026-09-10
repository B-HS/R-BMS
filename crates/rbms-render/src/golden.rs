//! Golden image harness: compare a rendered [`CpuCanvas`] against a PNG committed next to the
//! tests.
//!
//! The block signature ([`CpuCanvas::signature_hash`]) stays the fast regression net — one `u64`
//! per screen, insensitive to sub-pixel noise. This adds the other half: for the small,
//! code-generated scenes that exercise the draw primitives, the expected pixels themselves are
//! committed, so a regression can be looked at rather than only detected.
//!
//! PNG encoding and decoding is injected as a [`PngCodec`] rather than linked in. A codec is only
//! ever needed by tests, and the crate must not grow a runtime image dependency to hold goldens.

use std::path::PathBuf;

use crate::{BYTES_PER_PIXEL, CHANNEL_MAX, CpuCanvas, Renderer};

/// Set this to `1` to write goldens instead of comparing against them: a missing file is created
/// and a mismatching one is replaced, and the run passes. Unset, a missing golden is a failure —
/// a golden that quietly creates itself on the first CI run asserts nothing.
pub const GOLDEN_UPDATE_ENV: &str = "RBMS_GOLDEN_UPDATE";

/// Directory under the crate root holding the committed goldens.
const GOLDEN_DIR: &str = "tests/golden";

/// Directory under the build target holding what a failing comparison actually drew.
const GOLDEN_OUT_DIR: &str = "golden-out";

/// An RGBA8 image, the only currency this module deals in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl GoldenImage {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> GoldenImage {
        GoldenImage { width, height, rgba }
    }

    pub fn from_canvas(canvas: &CpuCanvas) -> GoldenImage {
        let (width, height) = canvas.size();
        GoldenImage { width, height, rgba: canvas.pixels().to_vec() }
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; BYTES_PER_PIXEL] {
        let i = (y as usize * self.width as usize + x as usize) * BYTES_PER_PIXEL;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }
}

/// How much difference a comparison tolerates, and how finely it localises what moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoldenOptions {
    /// Columns of the grid a difference is reported in.
    pub cols: u32,
    /// Rows of the grid a difference is reported in.
    pub rows: u32,
    /// Per-channel difference a pixel may have and still count as unchanged.
    pub max_channel_delta: u8,
    /// Share of the image that may differ before the comparison fails.
    pub max_diff_ratio: f32,
}

impl Default for GoldenOptions {
    fn default() -> Self {
        GoldenOptions { cols: 16, rows: 9, max_channel_delta: 2, max_diff_ratio: 0.002 }
    }
}

/// What a comparison found.
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenDiff {
    pub total_pixels: usize,
    pub differing_pixels: usize,
    /// Largest single-channel difference seen anywhere.
    pub max_channel_delta: u8,
    /// Grid cell holding the most differing pixels, as `(col, row, count)`.
    pub worst_cell: Option<(u32, u32, usize)>,
}

impl GoldenDiff {
    pub fn ratio(&self) -> f32 {
        if self.total_pixels == 0 { 0.0 } else { self.differing_pixels as f32 / self.total_pixels as f32 }
    }

    pub fn within(&self, opts: &GoldenOptions) -> bool {
        self.differing_pixels == 0 || self.ratio() <= opts.max_diff_ratio
    }
}

/// Compare two images, tolerating `opts.max_channel_delta` per channel.
pub fn compare(actual: &GoldenImage, expected: &GoldenImage, opts: &GoldenOptions) -> Result<GoldenDiff, String> {
    if actual.width != expected.width || actual.height != expected.height {
        return Err(format!("size changed: golden is {}x{}, render is {}x{}", expected.width, expected.height, actual.width, actual.height));
    }
    let (cols, rows) = (opts.cols.max(1), opts.rows.max(1));
    let mut cells = vec![0usize; (cols * rows) as usize];
    let mut differing = 0usize;
    let mut worst_delta = 0u8;
    for y in 0..actual.height {
        for x in 0..actual.width {
            let (a, e) = (actual.pixel(x, y), expected.pixel(x, y));
            let delta = (0..BYTES_PER_PIXEL).map(|c| a[c].abs_diff(e[c])).max().unwrap_or(0);
            worst_delta = worst_delta.max(delta);
            if delta > opts.max_channel_delta {
                differing += 1;
                let col = (x as u64 * cols as u64 / actual.width.max(1) as u64) as u32;
                let row = (y as u64 * rows as u64 / actual.height.max(1) as u64) as u32;
                cells[(row * cols + col) as usize] += 1;
            }
        }
    }
    let worst_cell = cells.iter().enumerate().max_by_key(|(_, n)| **n).filter(|(_, n)| **n > 0).map(|(i, n)| (i as u32 % cols, i as u32 / cols, *n));
    Ok(GoldenDiff { total_pixels: (actual.width as usize) * (actual.height as usize), differing_pixels: differing, max_channel_delta: worst_delta, worst_cell })
}

/// A picture of where two images disagree: differing pixels in red, the rest dimmed, so the shape
/// of a regression is readable at a glance.
pub fn diff_image(actual: &GoldenImage, expected: &GoldenImage, opts: &GoldenOptions) -> GoldenImage {
    let mut rgba = vec![CHANNEL_MAX as u8; actual.rgba.len()];
    for y in 0..actual.height {
        for x in 0..actual.width {
            let (a, e) = (actual.pixel(x, y), expected.pixel(x, y));
            let delta = (0..BYTES_PER_PIXEL).map(|c| a[c].abs_diff(e[c])).max().unwrap_or(0);
            let i = (y as usize * actual.width as usize + x as usize) * BYTES_PER_PIXEL;
            let shade = [a[0] / DIFF_DIM_DIVISOR, a[1] / DIFF_DIM_DIVISOR, a[2] / DIFF_DIM_DIVISOR];
            let marked = if delta > opts.max_channel_delta { [CHANNEL_MAX as u8, 0, 0] } else { shade };
            rgba[i..i + 3].copy_from_slice(&marked);
            rgba[i + 3] = CHANNEL_MAX as u8;
        }
    }
    GoldenImage { width: actual.width, height: actual.height, rgba }
}

/// How much the unchanged part of a diff picture is dimmed, so the marked pixels stand out.
const DIFF_DIM_DIVISOR: u8 = 4;

/// PNG read and write, supplied by whoever runs the comparison.
pub trait PngCodec {
    fn encode(&self, image: &GoldenImage) -> Result<Vec<u8>, String>;
    fn decode(&self, bytes: &[u8]) -> Result<GoldenImage, String>;
}

/// Where a golden lives, given the crate root the caller was compiled from.
pub fn golden_path(manifest_dir: &str, name: &str) -> PathBuf {
    PathBuf::from(manifest_dir).join(GOLDEN_DIR).join(format!("{name}.png"))
}

fn golden_out_dir(manifest_dir: &str) -> PathBuf {
    match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join(GOLDEN_OUT_DIR),
        _ => PathBuf::from(manifest_dir).join("..").join("..").join("target").join(GOLDEN_OUT_DIR),
    }
}

fn update_requested() -> bool {
    std::env::var(GOLDEN_UPDATE_ENV).is_ok_and(|v| v == "1")
}

/// Compare `canvas` against `tests/golden/{name}.png`.
///
/// `manifest_dir` is the crate root (`env!("CARGO_MANIFEST_DIR")` at the call site), and `codec`
/// supplies PNG encoding and decoding — see the module docs for why it is injected. On a mismatch
/// the rendered frame and a diff picture are written under `target/golden-out/` before the error is
/// returned. With [`GOLDEN_UPDATE_ENV`] set the golden is written instead and the check passes.
pub fn assert_golden_png(canvas: &CpuCanvas, manifest_dir: &str, name: &str, opts: GoldenOptions, codec: &impl PngCodec) -> Result<(), String> {
    let actual = GoldenImage::from_canvas(canvas);
    let path = golden_path(manifest_dir, name);
    let write_golden = || {
        let dir = path.parent().ok_or_else(|| format!("{name}: golden path has no directory"))?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{name}: cannot create {}: {e}", dir.display()))?;
        let bytes = codec.encode(&actual)?;
        std::fs::write(&path, bytes).map_err(|e| format!("{name}: cannot write {}: {e}", path.display()))
    };
    let stored = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) if update_requested() => return write_golden(),
        Err(e) => {
            return Err(format!("{name}: no golden at {} ({e}); re-run with {GOLDEN_UPDATE_ENV}=1 to create it", path.display()));
        }
    };
    let expected = codec.decode(&stored)?;
    let diff = match compare(&actual, &expected, &opts) {
        Ok(diff) => diff,
        Err(message) if update_requested() => {
            let _ = message;
            return write_golden();
        }
        Err(message) => return Err(format!("{name}: {message}")),
    };
    if diff.within(&opts) {
        return Ok(());
    }
    if update_requested() {
        return write_golden();
    }
    let out = golden_out_dir(manifest_dir);
    let mut written = Vec::new();
    if std::fs::create_dir_all(&out).is_ok() {
        for (suffix, image) in [("actual", actual.clone()), ("diff", diff_image(&actual, &expected, &opts))] {
            let target = out.join(format!("{name}.{suffix}.png"));
            if let Ok(bytes) = codec.encode(&image)
                && std::fs::write(&target, bytes).is_ok()
            {
                written.push(target.display().to_string());
            }
        }
    }
    Err(format!(
        "{name}: {} of {} pixels differ ({:.4} > {:.4}), worst channel delta {}, worst cell {:?}; wrote {}",
        diff.differing_pixels,
        diff.total_pixels,
        diff.ratio(),
        opts.max_diff_ratio,
        diff.max_channel_delta,
        diff.worst_cell,
        if written.is_empty() { "nothing".to_string() } else { written.join(", ") }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Rect};

    fn image(width: u32, height: u32, fill: [u8; BYTES_PER_PIXEL]) -> GoldenImage {
        GoldenImage::new(width, height, fill.repeat((width * height) as usize))
    }

    #[test]
    fn identical_images_have_no_difference() {
        let opts = GoldenOptions::default();
        let diff = compare(&image(4, 4, [1, 2, 3, 255]), &image(4, 4, [1, 2, 3, 255]), &opts).expect("same size");
        assert_eq!(diff.differing_pixels, 0);
        assert_eq!(diff.max_channel_delta, 0);
        assert_eq!(diff.worst_cell, None);
        assert!(diff.within(&opts));
    }

    #[test]
    fn a_difference_under_the_channel_tolerance_is_not_counted() {
        let opts = GoldenOptions::default();
        let diff = compare(&image(4, 4, [10, 10, 10, 255]), &image(4, 4, [12, 10, 10, 255]), &opts).expect("same size");
        assert_eq!(diff.differing_pixels, 0, "a delta of 2 is within the default tolerance");
        assert_eq!(diff.max_channel_delta, 2, "but it is still reported as the worst delta seen");
    }

    #[test]
    fn a_difference_over_the_channel_tolerance_fails_and_is_localised() {
        let opts = GoldenOptions { cols: 2, rows: 2, ..GoldenOptions::default() };
        let mut actual = image(4, 4, [0, 0, 0, 255]);
        let i = (3 * 4 + 3) * BYTES_PER_PIXEL;
        actual.rgba[i] = 200;
        let diff = compare(&actual, &image(4, 4, [0, 0, 0, 255]), &opts).expect("same size");
        assert_eq!(diff.differing_pixels, 1);
        assert_eq!(diff.max_channel_delta, 200);
        assert_eq!(diff.worst_cell, Some((1, 1, 1)), "the changed pixel is in the bottom right cell");
        assert!(!diff.within(&opts), "one pixel of sixteen is far over the 0.002 ratio");
    }

    #[test]
    fn a_size_change_is_reported_rather_than_compared() {
        let err = compare(&image(4, 4, [0, 0, 0, 255]), &image(4, 5, [0, 0, 0, 255]), &GoldenOptions::default()).expect_err("sizes differ");
        assert!(err.contains("4x5") && err.contains("4x4"), "the message names both sizes: {err}");
    }

    #[test]
    fn the_diff_picture_marks_only_the_changed_pixels() {
        let opts = GoldenOptions::default();
        let mut actual = image(2, 1, [40, 40, 40, 255]);
        actual.rgba[0] = 200;
        let picture = diff_image(&actual, &image(2, 1, [40, 40, 40, 255]), &opts);
        assert_eq!(picture.pixel(0, 0), [255, 0, 0, 255], "the changed pixel is marked red");
        assert_eq!(picture.pixel(1, 0), [10, 10, 10, 255], "an unchanged pixel is dimmed to a quarter");
    }

    #[test]
    fn an_image_from_a_canvas_carries_its_pixels() {
        let mut canvas = CpuCanvas::new(2, 2);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::rgb(9, 8, 7));
        let image = GoldenImage::from_canvas(&canvas);
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.pixel(0, 0), [9, 8, 7, 255]);
        assert_eq!(image.pixel(1, 1), [0, 0, 0, 255]);
    }

    /// A trivial round-trippable encoding, so the harness itself can be checked without pulling a
    /// PNG codec into the crate.
    struct RawCodec;

    impl PngCodec for RawCodec {
        fn encode(&self, image: &GoldenImage) -> Result<Vec<u8>, String> {
            let mut out = image.width.to_le_bytes().to_vec();
            out.extend_from_slice(&image.height.to_le_bytes());
            out.extend_from_slice(&image.rgba);
            Ok(out)
        }

        fn decode(&self, bytes: &[u8]) -> Result<GoldenImage, String> {
            let header = 2 * size_of::<u32>();
            if bytes.len() < header {
                return Err("truncated".to_string());
            }
            let read = |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            Ok(GoldenImage::new(read(0), read(size_of::<u32>()), bytes[header..].to_vec()))
        }
    }

    /// The rule that keeps a golden honest: with no expectation on disk and no explicit request to
    /// write one, the check fails rather than quietly minting whatever was just drawn.
    #[test]
    fn a_missing_golden_fails_instead_of_creating_itself() {
        let mut canvas = CpuCanvas::new(2, 2);
        canvas.clear(Color::BLACK);
        let temp = std::env::temp_dir().join(format!("rbms-golden-{}", std::process::id()));
        let error = assert_golden_png(&canvas, &temp.display().to_string(), "absent", GoldenOptions::default(), &RawCodec).expect_err("no golden on disk");
        assert!(error.contains(GOLDEN_UPDATE_ENV), "the message says how to create one: {error}");
        assert!(!golden_path(&temp.display().to_string(), "absent").exists(), "and nothing was written");
    }

    #[test]
    fn a_goldens_path_sits_under_the_crates_test_directory() {
        let path = golden_path("/crate", "scene");
        assert!(path.ends_with("tests/golden/scene.png"), "unexpected path {}", path.display());
    }
}
