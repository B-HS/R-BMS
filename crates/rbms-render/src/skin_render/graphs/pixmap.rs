//! The pixel buffer the distribution and tempo graphs paint into before it is uploaded as a texture.
//!
//! The reference paints these graphs into a `Pixmap` and draws the pixmap as one image, so the shape
//! of what reaches the screen is decided by that buffer's own pixels and by how it is scaled onto the
//! graph's rectangle. This type is only as much of a pixmap as those graphs use: a transparent
//! buffer, a fill, filled rectangles and an upright line.
//!
//! A rectangle or a line can be written two ways. [`Pixmap::fill_rect`] replaces what is under it,
//! which is all a graph painting opaque colours needs. [`Pixmap::blend_rect`] mixes the colour into
//! what is under it the way the reference's pixmap does, which is what a graph whose record names
//! translucent colours needs: that mixing is not the usual one, and a translucent colour painted
//! over nothing comes out darker than it was written.
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

/// Most a colour channel holds, as the whole number the reference's blend divides by.
const CHANNEL_MAX: i32 = 255;

/// One channel of `over` mixed into the same channel of `under` at `alpha`: the difference scaled
/// by the alpha with the remainder dropped towards zero, as C's division drops it.
fn mix_channel(over: u8, under: u8, alpha: u8) -> u8 {
    (i32::from(under) + i32::from(alpha) * (i32::from(over) - i32::from(under)) / CHANNEL_MAX) as u8
}

/// The colour a pixel holding `under` is left with after `over` is painted on it with the
/// reference's blending switched on, which is how every pixmap there is made (`gdx2d.c`'s `blend`
/// in the build the reference ships with).
///
/// The colour channels move towards the new colour by its alpha, whatever the pixel's own alpha was,
/// so a translucent colour painted over a transparent pixel is darkened towards the black a
/// transparent pixel holds. The alpha is the usual one for a layer over a layer, computed in single
/// precision and truncated.
pub(super) fn blend_over(over: Rgba, under: Rgba) -> Rgba {
    let [red, green, blue, alpha] = over;
    let through = 1.0 - f32::from(alpha) / CHANNEL_SCALE;
    let covered = 1.0 - (1.0 - f32::from(under[3]) / CHANNEL_SCALE) * through;
    [mix_channel(red, under[0], alpha), mix_channel(green, under[1], alpha), mix_channel(blue, under[2], alpha), (covered * CHANNEL_SCALE) as u8]
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

    /// Mixes `color` into the pixels of a rectangle the way `Pixmap.fillRectangle` does
    /// ([`blend_over`]), cut to the buffer.
    ///
    /// The rectangle is the reference's, corner case for corner case (`gdx2d_fill_rect` and the
    /// `hline` it paints each row with). A rectangle of no height paints nothing. A rectangle of no
    /// width does paint: its row runs from `x` to `x - 1`, the row painter puts the two ends in order
    /// and paints both, so it comes out two pixels wide, one of them left of `x`; a negative width
    /// reaches further left the same way. Only a rectangle that starts on the left edge escapes
    /// that, because its far end is then off the buffer. A negative height paints nothing here.
    pub(super) fn blend_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgba) {
        let (x, y, width, height) = (i64::from(x), i64::from(y), i64::from(width), i64::from(height));
        let (columns, rows) = (self.width as i64, self.height as i64);
        let (far, last) = (x + width - 1, y + height - 1);
        if x >= columns || y >= rows || far < 0 || last < 0 {
            return;
        }
        let (near, far) = (x.max(0), far.min(columns - 1));
        let (left, right) = (near.min(far), near.max(far));
        for row in y.max(0)..=last.min(rows - 1) {
            let start = (row as usize * self.width + left as usize) * BYTES_PER_PIXEL;
            let end = (row as usize * self.width + right as usize + 1) * BYTES_PER_PIXEL;
            for pixel in self.rgba[start..end].chunks_exact_mut(BYTES_PER_PIXEL) {
                let mixed = blend_over(color, [pixel[0], pixel[1], pixel[2], pixel[3]]);
                pixel.copy_from_slice(&mixed);
            }
        }
    }

    /// Mixes `color` into the upright line from `(x, top)` to `(x, bottom)`, both ends included.
    pub(super) fn blend_upright_line(&mut self, x: i32, top: i32, bottom: i32, color: Rgba) {
        self.blend_rect(x, top, 1, bottom - top + 1, color);
    }

    /// The pixels as they were painted, first row first, which is what a texture needs when the
    /// buffer's first row is to land on the top of the rectangle it is drawn into.
    pub(super) fn top_down(&self) -> &[u8] {
        &self.rgba
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
    fn an_opaque_colour_blended_in_replaces_what_was_there() {
        let mut pixmap = Pixmap::new(2, 2);
        pixmap.fill(BLUE);
        pixmap.blend_rect(0, 0, 1, 2, RED);
        assert_eq!((pixmap.pixel(0, 1), pixmap.pixel(1, 1)), (RED, BLUE));
    }

    #[test]
    fn a_translucent_colour_over_nothing_is_darkened_by_its_own_alpha() {
        assert_eq!(blend_over([0x44, 0x00, 0x44, 0x55], CLEAR), [22, 0, 22, 0x55], "0x44 * 0x55 / 255 with the remainder dropped");
        assert_eq!(blend_over([255, 255, 255, 128], CLEAR), [128, 128, 128, 128]);
        assert_eq!(blend_over([200, 100, 50, 0], RED), RED, "a colour with no alpha leaves the pixel as it was");
    }

    #[test]
    fn a_translucent_colour_over_a_translucent_one_moves_towards_it_and_gains_alpha() {
        let mixed = blend_over([0x44, 0x00, 0x00, 0x55], [0x44, 0x00, 0x44, 0x55]);
        assert_eq!(mixed, [0x44, 0, 46, 141], "blue falls by 85 * 68 / 255 truncated towards zero, and two thirds squared is left uncovered");
        assert_eq!(blend_over([0, 0, 0, 63], [0, 0x88, 0, 255]), [0, 103, 0, 255], "a quarter black over an opaque green");
    }

    #[test]
    fn a_blended_line_includes_both_of_its_ends_and_is_cut_to_the_buffer() {
        let mut pixmap = Pixmap::new(2, 3);
        pixmap.blend_upright_line(1, 0, 1, RED);
        assert_eq!((pixmap.pixel(1, 0), pixmap.pixel(1, 1), pixmap.pixel(1, 2)), (RED, RED, CLEAR));
        pixmap.blend_upright_line(0, 0, 40, BLUE);
        assert_eq!(pixmap.pixel(0, 2), BLUE);
        pixmap.blend_rect(5, 5, 3, 3, RED);
        pixmap.blend_rect(0, 0, 3, 0, RED);
        pixmap.blend_rect(0, 2, 3, -2, RED);
        assert_eq!(pixmap.pixel(0, 0), BLUE, "no overlap and no height paint nothing");
    }

    #[test]
    fn a_blended_rectangle_of_no_width_is_two_pixels_wide_as_the_reference_paints_it() {
        let mut pixmap = Pixmap::new(6, 2);
        pixmap.blend_rect(3, 0, 0, 1, RED);
        assert_eq!((0..6).map(|x| pixmap.pixel(x, 0)).collect::<Vec<_>>(), vec![CLEAR, CLEAR, RED, RED, CLEAR, CLEAR], "it runs from x back to x - 1");
        assert_eq!(pixmap.pixel(3, 1), CLEAR, "over the rows it was given and no more");

        pixmap.clear();
        pixmap.blend_rect(0, 0, 0, 2, RED);
        assert!((0..6).all(|x| pixmap.pixel(x, 0) == CLEAR), "on the left edge its far end is off the buffer and nothing is painted");

        pixmap.blend_rect(4, 0, -2, 1, BLUE);
        assert_eq!((0..6).map(|x| pixmap.pixel(x, 0)).collect::<Vec<_>>(), vec![CLEAR, BLUE, BLUE, BLUE, BLUE, CLEAR], "a negative width reaches further left");

        pixmap.clear();
        pixmap.blend_rect(-3, 0, 5, 1, RED);
        pixmap.blend_rect(4, 1, 9, 9, BLUE);
        assert_eq!((pixmap.pixel(0, 0), pixmap.pixel(1, 0), pixmap.pixel(2, 0)), (RED, RED, CLEAR), "an ordinary rectangle is cut on the left");
        assert_eq!((pixmap.pixel(3, 1), pixmap.pixel(4, 1), pixmap.pixel(5, 1)), (CLEAR, BLUE, BLUE), "and on the right and below");
    }

    #[test]
    fn the_painted_rows_are_handed_over_in_the_order_they_were_painted() {
        let mut pixmap = Pixmap::new(1, 2);
        pixmap.fill_rect(0, 0, 1, 1, RED);
        assert_eq!(pixmap.top_down(), &[255, 0, 0, 255, 0, 0, 0, 0]);
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
