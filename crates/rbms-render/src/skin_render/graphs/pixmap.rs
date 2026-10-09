//! The pixel buffer the distribution and tempo graphs paint into before it is uploaded as a texture.
//!
//! The reference paints these graphs into a `Pixmap` and draws the pixmap as one image, so the shape
//! of what reaches the screen is decided by that buffer's own pixels and by how it is scaled onto the
//! graph's rectangle. This type is only as much of a pixmap as those graphs use: a transparent
//! buffer, a fill, filled rectangles and an upright line, all written outright rather than blended,
//! because every colour they paint is either opaque or meant to replace what is under it.
//!
//! Row 0 is the top row of the buffer, as it is in a pixmap, but the reference draws the texture
//! with a negative height so that row 0 lands on the bottom of the graph's rectangle. [`Pixmap::bottom_up`]
//! is that flip, done once per upload.

use crate::BYTES_PER_PIXEL;

/// One pixel, red first, not premultiplied.
pub(super) type Rgba = [u8; BYTES_PER_PIXEL];

/// The colour of a pixel nothing has been painted on.
pub(super) const TRANSPARENT: Rgba = [0; BYTES_PER_PIXEL];

/// Most a colour channel holds, as the float the reference scales a unit channel by.
const CHANNEL_SCALE: f32 = 255.0;

/// The colour the reference stores for a hex colour `0xRRGGBB`, opaque.
///
/// `Color.valueOf` divides each hex pair by 255 into a float and a pixmap multiplies it back and
/// truncates, which hands every pair back unchanged in single precision, so the bytes are the pair.
pub(super) fn hex_color(rgb: u32) -> Rgba {
    let [_, red, green, blue] = rgb.to_be_bytes();
    [red, green, blue, u8::MAX]
}

/// The colour the reference stores for a colour given as floats in `0..=1`, truncated per channel
/// (`Pixmap.setColor(float, float, float, float)`).
pub(super) fn unit_color(red: f32, green: f32, blue: f32, alpha: f32) -> Rgba {
    [red, green, blue, alpha].map(|channel| (channel * CHANNEL_SCALE) as u8)
}

/// A buffer of RGBA pixels, transparent until painted.
#[derive(Default, Clone, PartialEq, Eq)]
pub(super) struct Pixmap {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl std::fmt::Debug for Pixmap {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Pixmap").field("width", &self.width).field("height", &self.height).finish_non_exhaustive()
    }
}

impl Pixmap {
    /// A transparent buffer of `width` by `height` pixels.
    pub(super) fn new(width: usize, height: usize) -> Pixmap {
        Pixmap { width, height, rgba: vec![0; width * height * BYTES_PER_PIXEL] }
    }

    pub(super) fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Makes every pixel transparent again.
    pub(super) fn clear(&mut self) {
        self.rgba.fill(0);
    }

    /// Sets every pixel to `color`.
    pub(super) fn fill(&mut self, color: Rgba) {
        for pixel in self.rgba.chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&color);
        }
    }

    /// Sets the pixels of a rectangle to `color`, cut to the buffer. A rectangle with no extent
    /// paints nothing.
    pub(super) fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgba) {
        let (x, y, width, height) = (i64::from(x), i64::from(y), i64::from(width), i64::from(height));
        let left = x.max(0);
        let top = y.max(0);
        let right = (x + width).min(self.width as i64);
        let bottom = (y + height).min(self.height as i64);
        if left >= right {
            return;
        }
        for row in top..bottom {
            let start = (row as usize * self.width + left as usize) * BYTES_PER_PIXEL;
            let end = (row as usize * self.width + right as usize) * BYTES_PER_PIXEL;
            for pixel in self.rgba[start..end].chunks_exact_mut(BYTES_PER_PIXEL) {
                pixel.copy_from_slice(&color);
            }
        }
    }

    /// Paints the upright line from `(x, top)` to `(x, bottom)`, both ends included, one pixel wide
    /// (`Pixmap.drawLine` with both ends on the same column).
    pub(super) fn upright_line(&mut self, x: i32, top: i32, bottom: i32, color: Rgba) {
        self.fill_rect(x, top, 1, bottom - top + 1, color);
    }

    /// The pixel at `(x, y)`, or transparent outside the buffer.
    #[cfg(test)]
    pub(super) fn pixel(&self, x: usize, y: usize) -> Rgba {
        if x >= self.width || y >= self.height {
            return [0; BYTES_PER_PIXEL];
        }
        let at = (y * self.width + x) * BYTES_PER_PIXEL;
        [self.rgba[at], self.rgba[at + 1], self.rgba[at + 2], self.rgba[at + 3]]
    }

    /// The pixels with their rows in reverse order, which is what a texture needs so that the
    /// buffer's first row lands on the bottom of the rectangle it is drawn into.
    pub(super) fn bottom_up(&self) -> Vec<u8> {
        let stride = self.width * BYTES_PER_PIXEL;
        if stride == 0 {
            return Vec::new();
        }
        self.rgba.chunks_exact(stride).rev().flatten().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgba = [255, 0, 0, 255];
    const BLUE: Rgba = [0, 0, 255, 255];
    const CLEAR: Rgba = [0; BYTES_PER_PIXEL];

    #[test]
    fn a_new_buffer_is_transparent_and_a_fill_covers_all_of_it() {
        let mut pixmap = Pixmap::new(3, 2);
        assert_eq!(pixmap.size(), (3, 2));
        assert_eq!(pixmap.pixel(2, 1), CLEAR);
        pixmap.fill(RED);
        assert!((0..3).all(|x| (0..2).all(|y| pixmap.pixel(x, y) == RED)));
        pixmap.clear();
        assert_eq!(pixmap.pixel(0, 0), CLEAR);
    }

    #[test]
    fn a_rectangle_is_cut_to_the_buffer_on_every_side() {
        let mut pixmap = Pixmap::new(4, 4);
        pixmap.fill_rect(-2, -2, 4, 4, RED);
        assert_eq!(pixmap.pixel(1, 1), RED, "the part inside the buffer is painted");
        assert_eq!(pixmap.pixel(2, 2), CLEAR, "and nothing past where the rectangle ends");

        pixmap.fill_rect(3, 3, 10, 10, BLUE);
        assert_eq!(pixmap.pixel(3, 3), BLUE, "a rectangle running off the far edge is painted up to it");

        pixmap.clear();
        pixmap.fill_rect(1, 1, 0, 3, RED);
        pixmap.fill_rect(1, 1, 3, -1, RED);
        pixmap.fill_rect(9, 9, 2, 2, RED);
        assert!((0..4).all(|x| (0..4).all(|y| pixmap.pixel(x, y) == CLEAR)), "no extent or no overlap paints nothing");
    }

    #[test]
    fn a_line_includes_both_of_its_ends() {
        let mut pixmap = Pixmap::new(3, 6);
        pixmap.upright_line(1, 1, 4, RED);
        assert_eq!(pixmap.pixel(1, 0), CLEAR);
        assert_eq!((1..=4).map(|y| pixmap.pixel(1, y)).collect::<Vec<_>>(), vec![RED; 4]);
        assert_eq!(pixmap.pixel(1, 5), CLEAR);
        assert_eq!(pixmap.pixel(0, 2), CLEAR, "and it is one pixel wide");

        pixmap.upright_line(2, 0, 20, BLUE);
        assert_eq!(pixmap.pixel(2, 5), BLUE, "a line that outruns the buffer is cut to it");
    }

    #[test]
    fn a_texture_is_the_buffer_turned_upside_down() {
        let mut pixmap = Pixmap::new(2, 2);
        pixmap.fill_rect(0, 0, 1, 1, RED);
        pixmap.fill_rect(1, 1, 1, 1, BLUE);
        let flipped = pixmap.bottom_up();
        assert_eq!(&flipped[..4], &CLEAR, "the buffer's last row comes first");
        assert_eq!(&flipped[4..8], &BLUE);
        assert_eq!(&flipped[8..12], &RED, "and its first row comes last");
        assert!(Pixmap::default().bottom_up().is_empty());
    }

    #[test]
    fn a_hex_colour_is_its_three_pairs_made_opaque() {
        assert_eq!(hex_color(0x44_FF_44), [0x44, 0xFF, 0x44, 255]);
        assert_eq!(hex_color(0x00_00_00), [0, 0, 0, 255]);
    }

    #[test]
    fn a_unit_colour_is_truncated_not_rounded() {
        assert_eq!(unit_color(0.8, 0.0, 0.0, 1.0), [204, 0, 0, 255]);
        assert_eq!(unit_color(0.25, 0.125, 0.007, 0.5), [63, 31, 1, 127]);
    }
}
