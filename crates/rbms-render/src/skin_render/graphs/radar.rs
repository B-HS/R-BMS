//! The NOTES RADAR polygon: the picture a document places by the reference image id
//! `RBMS_IMAGE_NOTES_RADAR`, painted by the renderer from the six display values of the chart in hand.
//!
//! The picture is a square of [`IMAGE_SIZE`] pixels, clear except for an opaque white polygon whose
//! six corners lie on the six axes of the radar. The first axis points straight up and the others
//! follow clockwise, a sixth of a turn apart, as the screen shows them. A corner lies as far from the
//! middle as its value is of the largest display value, with a value of the largest reaching
//! [`OUTER_RADIUS`]. The document supplies the rest: the colour and the alpha of the destination tint
//! the white, and the grid, the labels and the stars are objects of its own.
//!
//! The edges are smoothed by sampling: each row of pixels is sampled [`SAMPLES_PER_ROW`] times down its
//! height, every sample finds the stretches of the row the polygon covers and how much of each pixel
//! they cover, and a pixel's alpha is the mean over the samples. Six equal values make a convex
//! hexagon, but values need not be equal: a smaller one pulls its corner in, so the polygon is the
//! star-shaped outline of six triangles that meet in the middle, and a row can cross it more than
//! twice.
//!
//! The picture is painted again only when the values change. A value is held between nothing and the
//! largest display value, and one that is not a number is read as nothing.

use rbms_chart::{RADAR_AXIS_COUNT, RADAR_DISPLAY_MAX};

use super::pixmap::Pixmap;
use super::{Layer, next_serial, paintable};
use crate::{Renderer, TextureId};

/// The six display values of the radar, in axis order.
pub(crate) type RadarValues = [f32; RADAR_AXIS_COUNT];

/// How many pixels each edge of the picture is.
const IMAGE_SIZE: usize = 256;

/// How far from the middle a corner lies at the largest display value, in pixels.
const OUTER_RADIUS: f64 = 124.0;

/// The middle of the picture, on both axes.
const CENTER: f64 = IMAGE_SIZE as f64 / 2.0;

/// How far round the middle one axis is from the one before it, in radians.
const AXIS_STEP: f64 = std::f64::consts::TAU / RADAR_AXIS_COUNT as f64;

/// How many times each row of pixels is sampled down its height.
const SAMPLES_PER_ROW: usize = 4;

/// What each colour channel of the polygon holds: the polygon is white, and the alpha is the pixel's own.
const WHITE: u8 = u8::MAX;

/// The most alpha a pixel takes.
const FULL_ALPHA: f64 = u8::MAX as f64;

/// A point of the picture, in pixels from its top left corner with the second axis running down.
type Corner = (f64, f64);

/// The display value `value` is drawn at: held between nothing and the largest, with a value that is
/// not a number taken for nothing.
fn reach_of(value: f32) -> f32 {
    if value.is_nan() { 0.0 } else { value.clamp(0.0, RADAR_DISPLAY_MAX) }
}

/// Where the corner on `axis` lies for a display value of `reach`.
fn corner_of(axis: usize, reach: f32) -> Corner {
    let distance = f64::from(reach) / f64::from(RADAR_DISPLAY_MAX) * OUTER_RADIUS;
    let angle = AXIS_STEP * axis as f64;
    (CENTER + distance * angle.sin(), CENTER - distance * angle.cos())
}

/// Adds to `coverage` how much of each pixel the stretch of a row from `left` to `right` covers.
fn add_span(coverage: &mut [f64; IMAGE_SIZE], left: f64, right: f64) {
    let (left, right) = (left.max(0.0), right.min(IMAGE_SIZE as f64));
    if right <= left {
        return;
    }
    let (first, end) = (left as usize, (right.ceil() as usize).min(IMAGE_SIZE));
    for (column, covered) in coverage.iter_mut().enumerate().take(end).skip(first) {
        let near = column as f64;
        *covered += right.min(near + 1.0) - left.max(near);
    }
}

/// The alpha of every pixel of the row numbered `row`.
///
/// Each sample of the row meets the polygon's edges where one end of an edge is at or above the
/// sample and the other below it, which counts a corner that lies on the sample once. A closed
/// outline is met an even number of times, and the stretches between the first and the second, the
/// third and the fourth, and so on, are inside it.
fn row_alphas(corners: &[Corner; RADAR_AXIS_COUNT], row: usize) -> [u8; IMAGE_SIZE] {
    let mut coverage = [0.0_f64; IMAGE_SIZE];
    for sample in 0..SAMPLES_PER_ROW {
        let height = row as f64 + (sample as f64 + 0.5) / SAMPLES_PER_ROW as f64;
        let mut crossings = [0.0_f64; RADAR_AXIS_COUNT];
        let mut met = 0;
        for (index, from) in corners.iter().enumerate() {
            let to = corners[(index + 1) % RADAR_AXIS_COUNT];
            if (from.1 <= height) != (to.1 <= height) {
                crossings[met] = from.0 + (height - from.1) / (to.1 - from.1) * (to.0 - from.0);
                met += 1;
            }
        }
        crossings[..met].sort_by(f64::total_cmp);
        for stretch in crossings[..met].chunks_exact(2) {
            add_span(&mut coverage, stretch[0], stretch[1]);
        }
    }
    coverage.map(|covered| (covered / SAMPLES_PER_ROW as f64 * FULL_ALPHA).round() as u8)
}

/// The picture of the polygon whose corners lie at the display values `reaches`.
fn paint(reaches: &RadarValues) -> Pixmap {
    let corners: [Corner; RADAR_AXIS_COUNT] = std::array::from_fn(|axis| corner_of(axis, reaches[axis]));
    let mut pixmap = Pixmap::new(IMAGE_SIZE, IMAGE_SIZE);
    for row in 0..IMAGE_SIZE {
        let alphas = row_alphas(&corners, row);
        let mut column = 0;
        while column < IMAGE_SIZE {
            let alpha = alphas[column];
            let run = alphas[column..].iter().take_while(|next| **next == alpha).count();
            if alpha > 0 {
                pixmap.fill_rect(column as i32, row as i32, run as i32, 1, [WHITE, WHITE, WHITE, alpha]);
            }
            column += run;
        }
    }
    pixmap
}

/// The NOTES RADAR picture of one reference image object, kept uploaded as a texture of its own.
#[derive(Debug)]
pub(crate) struct RadarImage {
    layer: Layer,
    pixmap: Pixmap,
    /// The display values the pixmap was painted for, once they have been held between nothing and
    /// the largest.
    painted: Option<RadarValues>,
}

impl RadarImage {
    pub(crate) fn new() -> RadarImage {
        RadarImage { layer: Layer::new(next_serial(), "notes-radar"), pixmap: Pixmap::default(), painted: None }
    }

    /// Paints the picture again when `values` are not the ones it was painted for.
    fn refresh(&mut self, values: RadarValues) {
        let reaches = values.map(reach_of);
        if self.painted == Some(reaches) {
            return;
        }
        self.pixmap = paint(&reaches);
        self.painted = Some(reaches);
        self.layer.mark_stale();
    }

    /// The texture holding the picture for `values`, or `None` when the renderer holds no texture
    /// of that size.
    pub(crate) fn texture<R: Renderer>(&mut self, r: &mut R, values: RadarValues) -> Option<TextureId> {
        if !paintable(r, IMAGE_SIZE as i64, IMAGE_SIZE as i64) {
            return None;
        }
        self.refresh(values);
        self.layer.texture(r, &self.pixmap)
    }

    /// Hands the texture back to the renderer it was uploaded to and lets go of the pixels.
    pub(crate) fn release<R: Renderer>(&mut self, r: &mut R) {
        self.layer.release(r);
        self.pixmap = Pixmap::default();
        self.painted = None;
    }
}

#[cfg(test)]
mod tests;
