//! Planar YUV 4:2:0 to RGBA, in the crate rather than through a dependency.
//!
//! Every decoder here hands back three 8-bit planes, and the renderer only takes RGBA, so the
//! conversion sits between them. It is the ITU-R BT.601 studio-swing matrix, which is what both
//! MPEG-1 video and the standard-definition H.264 a chart ships are authored against.

/// Black level of a studio-swing luma sample: 16, not 0.
const LUMA_FLOOR: i32 = 16;

/// The neutral value of a chroma sample, which the two difference components are measured from.
const CHROMA_NEUTRAL: i32 = 128;

/// Fractional bits the matrix coefficients below are scaled by, so the whole conversion is integer.
const FIXED_SHIFT: u32 = 16;

/// Half of one output step, added before the shift so the fixed-point result rounds rather than
/// truncates.
const FIXED_ROUND: i32 = 1 << (FIXED_SHIFT - 1);

/// BT.601 luma gain, 255/219 scaled by `FIXED_SHIFT`: studio swing stretched to the full range.
const LUMA_GAIN: i32 = 76309;

/// BT.601 red-from-Cr coefficient, 1.596027 scaled by `FIXED_SHIFT`.
const RED_FROM_V: i32 = 104597;

/// BT.601 green-from-Cb coefficient, -0.391762 scaled by `FIXED_SHIFT`.
const GREEN_FROM_U: i32 = -25675;

/// BT.601 green-from-Cr coefficient, -0.812968 scaled by `FIXED_SHIFT`.
const GREEN_FROM_V: i32 = -53279;

/// BT.601 blue-from-Cb coefficient, 2.017232 scaled by `FIXED_SHIFT`.
const BLUE_FROM_U: i32 = 132201;

/// Largest value an 8-bit component can take.
const COMPONENT_MAX: i32 = 255;

/// Bytes one RGBA pixel occupies.
const RGBA_BYTES: usize = 4;

/// How many luma samples one chroma sample covers along each axis in 4:2:0.
const CHROMA_SUBSAMPLING: usize = 2;

/// The geometry one conversion is asked for: the luma size and the size of the two chroma planes,
/// which a decoder rounds up independently of the visible luma size on an odd dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlaneGeometry {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) chroma_width: usize,
    pub(crate) chroma_height: usize,
}

/// Clamp one fixed-point component back into the 8-bit range the texture takes.
fn clamp_component(value: i32) -> u8 {
    value.clamp(0, COMPONENT_MAX) as u8
}

/// Convert one 4:2:0 frame into a tightly packed RGBA buffer, opaque throughout.
///
/// Chroma is read at the nearest sample rather than interpolated: a background is stretched over
/// the screen anyway, and the sharper edge costs nothing per pixel. Returns `None` when a plane is
/// shorter than the geometry it claims, which is the one way a malformed stream could read past
/// the end of a buffer.
pub(crate) fn yuv420_to_rgba(geometry: PlaneGeometry, y: &[u8], u: &[u8], v: &[u8]) -> Option<Vec<u8>> {
    let PlaneGeometry { width, height, chroma_width, chroma_height } = geometry;
    if width == 0 || height == 0 || chroma_width == 0 || chroma_height == 0 {
        return None;
    }
    if y.len() < width * height || u.len() < chroma_width * chroma_height || v.len() < chroma_width * chroma_height {
        return None;
    }
    let mut rgba = vec![u8::MAX; width * height * RGBA_BYTES];
    for row in 0..height {
        let chroma_row = (row / CHROMA_SUBSAMPLING).min(chroma_height - 1);
        for column in 0..width {
            let chroma_column = (column / CHROMA_SUBSAMPLING).min(chroma_width - 1);
            let luma = (i32::from(y[row * width + column]) - LUMA_FLOOR) * LUMA_GAIN;
            let cb = i32::from(u[chroma_row * chroma_width + chroma_column]) - CHROMA_NEUTRAL;
            let cr = i32::from(v[chroma_row * chroma_width + chroma_column]) - CHROMA_NEUTRAL;
            let pixel = (row * width + column) * RGBA_BYTES;
            rgba[pixel] = clamp_component((luma + RED_FROM_V * cr + FIXED_ROUND) >> FIXED_SHIFT);
            rgba[pixel + 1] = clamp_component((luma + GREEN_FROM_U * cb + GREEN_FROM_V * cr + FIXED_ROUND) >> FIXED_SHIFT);
            rgba[pixel + 2] = clamp_component((luma + BLUE_FROM_U * cb + FIXED_ROUND) >> FIXED_SHIFT);
        }
    }
    Some(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-pixel frame of the three planes a caller would hand over.
    fn single_pixel(luma: u8, cb: u8, cr: u8) -> Option<Vec<u8>> {
        let geometry = PlaneGeometry { width: 2, height: 2, chroma_width: 1, chroma_height: 1 };
        yuv420_to_rgba(geometry, &[luma; 4], &[cb], &[cr])
    }

    #[test]
    fn studio_black_and_white_reach_the_ends_of_the_range() {
        let black = single_pixel(16, 128, 128).expect("convert studio black");
        assert_eq!(&black[..4], &[0, 0, 0, 255]);
        let white = single_pixel(235, 128, 128).expect("convert studio white");
        assert_eq!(&white[..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn chroma_extremes_land_on_red_and_blue() {
        let red = single_pixel(81, 90, 240).expect("convert the BT.601 red bar");
        assert_eq!(&red[..4], &[254, 0, 0, 255]);
        let blue = single_pixel(41, 240, 110).expect("convert the BT.601 blue bar");
        assert_eq!(&blue[..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn every_pixel_of_a_frame_is_written_opaque() {
        let geometry = PlaneGeometry { width: 4, height: 2, chroma_width: 2, chroma_height: 1 };
        let rgba = yuv420_to_rgba(geometry, &[128; 8], &[128; 2], &[128; 2]).expect("convert a mid-grey frame");
        assert_eq!(rgba.len(), 4 * 2 * 4);
        assert!(rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
        assert!(rgba.chunks_exact(4).all(|pixel| pixel[0] == 130 && pixel[1] == 130 && pixel[2] == 130));
    }

    #[test]
    fn a_plane_shorter_than_its_geometry_is_refused() {
        let geometry = PlaneGeometry { width: 4, height: 2, chroma_width: 2, chroma_height: 1 };
        assert!(yuv420_to_rgba(geometry, &[128; 7], &[128; 2], &[128; 2]).is_none());
        assert!(yuv420_to_rgba(geometry, &[128; 8], &[128; 1], &[128; 2]).is_none());
    }
}
