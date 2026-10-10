//! Turning decoded Y'CbCr 4:2:0 frames into RGBA pixels.
//!
//! A decoder hands back three planes: luma at full size and two chroma planes at half size each
//! way. Which numbers turn those into red, green and blue is not fixed: it depends on the matrix
//! the video was encoded with and on whether its samples use the whole byte or the studio range.
//! A stream says both in its sequence parameter set, a file may say them again in a colour box,
//! and a stream that says neither is read by its size, the way players have always guessed
//! ([`ColorSpec::resolve`]).
//!
//! The conversion itself is a pure function of the planes and the spec ([`yuv420_to_rgba`]), in
//! whole numbers, so a frame converts to the same bytes on every machine.

/// The `matrix_coefficients` code point of BT.709 (H.264 table E-5).
const MATRIX_BT709: u32 = 1;

/// The code points that name the BT.601 matrix: BT.470BG and SMPTE 170M.
const MATRIX_BT601: [u32; 2] = [5, 6];

/// The frame height from which an unlabelled stream is taken for high definition, and so BT.709.
pub const HIGH_DEFINITION_HEIGHT: u32 = 720;

/// Bytes one RGBA pixel takes.
pub const RGBA_BYTES: usize = 4;

/// The alpha every converted pixel is given.
const OPAQUE: u8 = u8::MAX;

/// Fraction bits of the fixed point the conversion is done in.
const FRACTION_BITS: u32 = 16;

/// One half in that fixed point, added before a shift so it rounds.
const ROUND_HALF: i32 = 1 << (FRACTION_BITS - 1);

/// The value chroma is centred on.
const CHROMA_ZERO: i32 = 128;

/// The luma of black in the studio range.
const STUDIO_BLACK: i32 = 16;

/// How many luma steps the studio range spans, black to white.
const STUDIO_LUMA_SPAN: f64 = 219.0;

/// How many chroma steps the studio range spans.
const STUDIO_CHROMA_SPAN: f64 = 224.0;

/// How many steps a full range sample spans.
const FULL_SPAN: f64 = 255.0;

/// The weights of red and blue in BT.601 luma.
const BT601_WEIGHTS: (f64, f64) = (0.299, 0.114);

/// The weights of red and blue in BT.709 luma.
const BT709_WEIGHTS: (f64, f64) = (0.2126, 0.0722);

/// How many luma samples one chroma sample covers along each axis of a 4:2:0 frame.
const CHROMA_SPAN: usize = 2;

/// Which matrix a stream's chroma was made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMatrix {
    /// Standard definition ([`BT601_WEIGHTS`]).
    Bt601,
    /// High definition ([`BT709_WEIGHTS`]).
    Bt709,
}

impl ColorMatrix {
    /// The weights of red and blue in luma.
    fn weights(self) -> (f64, f64) {
        match self {
            ColorMatrix::Bt601 => BT601_WEIGHTS,
            ColorMatrix::Bt709 => BT709_WEIGHTS,
        }
    }
}

/// Whether a stream's samples use the studio range or the whole byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRange {
    /// Luma 16..=235 and chroma 16..=240.
    Limited,
    /// 0..=255 for all three.
    Full,
}

/// What a stream or its file says about its colour, each part only when it is said.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ColorHint {
    /// The `matrix_coefficients` code point.
    pub matrix_coefficients: Option<u32>,
    pub full_range: Option<bool>,
}

impl ColorHint {
    /// This hint, with whatever it leaves unsaid taken from `other`.
    pub fn or(self, other: ColorHint) -> ColorHint {
        ColorHint { matrix_coefficients: self.matrix_coefficients.or(other.matrix_coefficients), full_range: self.full_range.or(other.full_range) }
    }
}

/// Everything the conversion needs to know about a stream's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorSpec {
    pub matrix: ColorMatrix,
    pub range: ColorRange,
}

impl ColorSpec {
    /// The spec of a stream `height` pixels high that says `hint` about itself.
    ///
    /// A matrix the hint names is taken as named. One it does not name, or names as unspecified or
    /// as something this converter has no numbers for, is guessed from the height: BT.709 from
    /// [`HIGH_DEFINITION_HEIGHT`] up and BT.601 below. The range is the studio range unless the
    /// hint says otherwise.
    pub fn resolve(hint: ColorHint, height: u32) -> ColorSpec {
        let matrix = match hint.matrix_coefficients {
            Some(MATRIX_BT709) => ColorMatrix::Bt709,
            Some(code) if MATRIX_BT601.contains(&code) => ColorMatrix::Bt601,
            _ if height >= HIGH_DEFINITION_HEIGHT => ColorMatrix::Bt709,
            _ => ColorMatrix::Bt601,
        };
        let range = if hint.full_range == Some(true) { ColorRange::Full } else { ColorRange::Limited };
        ColorSpec { matrix, range }
    }
}

/// The conversion's multipliers, in fixed point.
#[derive(Debug, Clone, Copy)]
struct Multipliers {
    luma_offset: i32,
    luma: i32,
    red_from_cr: i32,
    green_from_cb: i32,
    green_from_cr: i32,
    blue_from_cb: i32,
}

impl Multipliers {
    fn of(spec: ColorSpec) -> Multipliers {
        let (red, blue) = spec.matrix.weights();
        let green = 1.0 - red - blue;
        let (luma_offset, luma_scale, chroma_scale) = match spec.range {
            ColorRange::Limited => (STUDIO_BLACK, FULL_SPAN / STUDIO_LUMA_SPAN, FULL_SPAN / STUDIO_CHROMA_SPAN),
            ColorRange::Full => (0, 1.0, 1.0),
        };
        let fixed = |value: f64| (value * f64::from(1u32 << FRACTION_BITS)).round() as i32;
        Multipliers {
            luma_offset,
            luma: fixed(luma_scale),
            red_from_cr: fixed(2.0 * (1.0 - red) * chroma_scale),
            green_from_cb: fixed(-2.0 * (1.0 - blue) * blue / green * chroma_scale),
            green_from_cr: fixed(-2.0 * (1.0 - red) * red / green * chroma_scale),
            blue_from_cb: fixed(2.0 * (1.0 - blue) * chroma_scale),
        }
    }

    /// One pixel: its luma with the chroma sample it shares with its neighbours.
    #[inline(always)]
    fn pixel(&self, luma: u8, cb: u8, cr: u8) -> [u8; RGBA_BYTES] {
        let scaled = (i32::from(luma) - self.luma_offset) * self.luma + ROUND_HALF;
        let (cb, cr) = (i32::from(cb) - CHROMA_ZERO, i32::from(cr) - CHROMA_ZERO);
        let channel = |value: i32| (value >> FRACTION_BITS).clamp(0, i32::from(u8::MAX)) as u8;
        [
            channel(scaled + self.red_from_cr * cr),
            channel(scaled + self.green_from_cb * cb + self.green_from_cr * cr),
            channel(scaled + self.blue_from_cb * cb),
            OPAQUE,
        ]
    }
}

/// One decoded 4:2:0 frame, as three planes that each may be wider than the picture.
#[derive(Debug, Clone, Copy)]
pub struct Yuv420<'a> {
    pub y: &'a [u8],
    pub u: &'a [u8],
    pub v: &'a [u8],
    /// Bytes from one luma row to the next.
    pub y_stride: usize,
    /// Bytes from one chroma row to the next, in either chroma plane.
    pub uv_stride: usize,
    /// The picture's size in pixels.
    pub width: usize,
    pub height: usize,
}

impl Yuv420<'_> {
    /// Whether every plane holds the rows and columns the picture's size asks of it.
    pub fn is_whole(&self) -> bool {
        let (chroma_width, chroma_height) = (self.width.div_ceil(CHROMA_SPAN), self.height.div_ceil(CHROMA_SPAN));
        let reaches = |plane: &[u8], stride: usize, width: usize, height: usize| {
            width <= stride
                && height.checked_sub(1).is_none_or(|last| last.checked_mul(stride).and_then(|at| at.checked_add(width)).is_some_and(|end| end <= plane.len()))
        };
        reaches(self.y, self.y_stride, self.width, self.height)
            && reaches(self.u, self.uv_stride, chroma_width, chroma_height)
            && reaches(self.v, self.uv_stride, chroma_width, chroma_height)
    }
}

/// Converts `frame` to RGBA, top row first, into `out`, and answers whether it did.
///
/// `out` has to hold exactly `width * height * 4` bytes and every plane has to reach as far as the
/// picture's size says; a frame that falls short of either is left unconverted rather than read
/// past its end. Each chroma sample colours the two by two block of pixels it was averaged from.
pub fn yuv420_to_rgba(spec: ColorSpec, frame: &Yuv420<'_>, out: &mut [u8]) -> bool {
    let Some(wanted) = frame.width.checked_mul(frame.height).and_then(|pixels| pixels.checked_mul(RGBA_BYTES)) else {
        return false;
    };
    if out.len() != wanted || !frame.is_whole() {
        return false;
    }
    if wanted == 0 {
        return true;
    }
    let multipliers = Multipliers::of(spec);
    let (width, chroma_width) = (frame.width, frame.width.div_ceil(CHROMA_SPAN));
    for (row, target) in out.chunks_exact_mut(width * RGBA_BYTES).enumerate() {
        let luma = &frame.y[row * frame.y_stride..][..width];
        let cb = &frame.u[(row / CHROMA_SPAN) * frame.uv_stride..][..chroma_width];
        let cr = &frame.v[(row / CHROMA_SPAN) * frame.uv_stride..][..chroma_width];
        let pairs = target.chunks_mut(CHROMA_SPAN * RGBA_BYTES).zip(luma.chunks(CHROMA_SPAN)).zip(cb.iter().zip(cr));
        for ((pixels, lumas), (&cb, &cr)) in pairs {
            for (pixel, &luma) in pixels.chunks_exact_mut(RGBA_BYTES).zip(lumas) {
                pixel.copy_from_slice(&multipliers.pixel(luma, cb, cr));
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITED_709: ColorSpec = ColorSpec { matrix: ColorMatrix::Bt709, range: ColorRange::Limited };
    const LIMITED_601: ColorSpec = ColorSpec { matrix: ColorMatrix::Bt601, range: ColorRange::Limited };
    const FULL_601: ColorSpec = ColorSpec { matrix: ColorMatrix::Bt601, range: ColorRange::Full };

    /// How far a converted channel may sit from the textbook value: the fixed point rounds once.
    const TOLERANCE: i32 = 1;

    /// One pixel of `(y, cb, cr)` converted under `spec`.
    fn one(spec: ColorSpec, y: u8, cb: u8, cr: u8) -> [u8; 4] {
        let mut out = [0u8; 4];
        let frame = Yuv420 { y: &[y], u: &[cb], v: &[cr], y_stride: 1, uv_stride: 1, width: 1, height: 1 };
        assert!(yuv420_to_rgba(spec, &frame, &mut out));
        out
    }

    fn assert_close(got: [u8; 4], wanted: [u8; 3], what: &str) {
        for channel in 0..3 {
            assert!((i32::from(got[channel]) - i32::from(wanted[channel])).abs() <= TOLERANCE, "{what}: got {got:?}, wanted {wanted:?}");
        }
        assert_eq!(got[3], u8::MAX, "{what}: a converted pixel is opaque");
    }

    #[test]
    fn studio_black_grey_and_white_convert_to_their_full_range_greys() {
        for spec in [LIMITED_601, LIMITED_709] {
            assert_eq!(one(spec, 16, 128, 128), [0, 0, 0, 255]);
            assert_eq!(one(spec, 235, 128, 128), [255, 255, 255, 255]);
            assert_close(one(spec, 126, 128, 128), [128, 128, 128], "mid grey");
        }
    }

    #[test]
    fn the_primaries_of_each_matrix_convert_back_to_themselves() {
        assert_close(one(LIMITED_601, 81, 90, 240), [255, 0, 0], "BT.601 red");
        assert_close(one(LIMITED_601, 145, 54, 34), [0, 255, 0], "BT.601 green");
        assert_close(one(LIMITED_601, 41, 240, 110), [0, 0, 255], "BT.601 blue");
        assert_close(one(LIMITED_709, 63, 102, 240), [255, 0, 0], "BT.709 red");
        assert_close(one(LIMITED_709, 173, 42, 26), [0, 255, 0], "BT.709 green");
        assert_close(one(LIMITED_709, 32, 240, 118), [0, 0, 255], "BT.709 blue");
    }

    #[test]
    fn the_two_matrices_disagree_about_the_same_samples() {
        let (as_601, as_709) = (one(LIMITED_601, 81, 90, 240), one(LIMITED_709, 81, 90, 240));
        assert_ne!(as_601, as_709, "a matrix that changes nothing is not being applied");
        assert!(as_709[1] > as_601[1], "BT.709 reads BT.601 red with green in it: {as_709:?}");
    }

    #[test]
    fn full_range_samples_are_not_stretched() {
        assert_eq!(one(FULL_601, 0, 128, 128), [0, 0, 0, 255]);
        assert_eq!(one(FULL_601, 255, 128, 128), [255, 255, 255, 255]);
        assert_eq!(one(FULL_601, 100, 128, 128), [100, 100, 100, 255]);
        assert_close(one(FULL_601, 76, 85, 255), [255, 0, 0], "full range red");
    }

    #[test]
    fn samples_outside_the_studio_range_clamp_rather_than_wrap() {
        assert_eq!(one(LIMITED_709, 0, 128, 128), [0, 0, 0, 255]);
        assert_eq!(one(LIMITED_709, 255, 128, 128), [255, 255, 255, 255]);
        assert_eq!(one(LIMITED_709, 235, 255, 255)[2], 255);
        assert_eq!(one(LIMITED_709, 16, 0, 0)[0], 0);
    }

    #[test]
    fn each_chroma_sample_colours_its_block_and_strides_are_skipped() {
        let y = [81, 81, 145, 0, 81, 81, 145, 0, 41, 41, 16, 0];
        let u = [90, 54, 0, 240, 128, 0];
        let v = [240, 34, 0, 110, 128, 0];
        let frame = Yuv420 { y: &y, u: &u, v: &v, y_stride: 4, uv_stride: 3, width: 3, height: 3 };
        let mut out = vec![0u8; 3 * 3 * 4];
        assert!(yuv420_to_rgba(LIMITED_601, &frame, &mut out));
        let pixel = |x: usize, y: usize| [out[(y * 3 + x) * 4], out[(y * 3 + x) * 4 + 1], out[(y * 3 + x) * 4 + 2], out[(y * 3 + x) * 4 + 3]];
        for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            assert_close(pixel(x, y), [255, 0, 0], "the top left block");
        }
        assert_close(pixel(2, 0), [0, 255, 0], "the odd column of the first row");
        assert_close(pixel(2, 1), [0, 255, 0], "the odd column of the second row");
        assert_close(pixel(0, 2), [0, 0, 255], "the odd row");
        assert_close(pixel(2, 2), [0, 0, 0], "the last pixel");
    }

    #[test]
    fn a_frame_that_falls_short_is_left_unconverted() {
        let whole = Yuv420 { y: &[16; 4], u: &[128], v: &[128], y_stride: 2, uv_stride: 1, width: 2, height: 2 };
        let mut out = vec![7u8; 16];
        assert!(yuv420_to_rgba(LIMITED_709, &whole, &mut out));
        assert!(!yuv420_to_rgba(LIMITED_709, &whole, &mut [0u8; 12]), "a short target was written into");
        assert!(!yuv420_to_rgba(LIMITED_709, &Yuv420 { y: &[16; 3], ..whole }, &mut out), "a short luma plane was read");
        assert!(!yuv420_to_rgba(LIMITED_709, &Yuv420 { u: &[], ..whole }, &mut out), "a missing chroma plane was read");
        assert!(!yuv420_to_rgba(LIMITED_709, &Yuv420 { y_stride: 1, ..whole }, &mut out), "a stride narrower than the picture was read");
        assert!(!yuv420_to_rgba(LIMITED_709, &Yuv420 { width: usize::MAX, height: usize::MAX, ..whole }, &mut out), "an impossible size was multiplied");
        let empty = Yuv420 { y: &[], u: &[], v: &[], y_stride: 0, uv_stride: 0, width: 0, height: 0 };
        assert!(yuv420_to_rgba(LIMITED_709, &empty, &mut []));
    }

    #[test]
    fn a_stream_is_read_by_what_it_says_and_otherwise_by_its_height() {
        let said = |matrix: u32| ColorHint { matrix_coefficients: Some(matrix), full_range: None };
        assert_eq!(ColorSpec::resolve(said(1), 480).matrix, ColorMatrix::Bt709);
        assert_eq!(ColorSpec::resolve(said(6), 1080).matrix, ColorMatrix::Bt601);
        assert_eq!(ColorSpec::resolve(said(5), 1080).matrix, ColorMatrix::Bt601);
        assert_eq!(ColorSpec::resolve(said(2), 1080).matrix, ColorMatrix::Bt709, "unspecified is guessed");
        assert_eq!(ColorSpec::resolve(ColorHint::default(), 720).matrix, ColorMatrix::Bt709);
        assert_eq!(ColorSpec::resolve(ColorHint::default(), 719).matrix, ColorMatrix::Bt601);
        assert_eq!(ColorSpec::resolve(ColorHint::default(), 1080).range, ColorRange::Limited);
        assert_eq!(ColorSpec::resolve(ColorHint { full_range: Some(true), ..ColorHint::default() }, 1080).range, ColorRange::Full);
    }

    #[test]
    fn a_hint_fills_its_gaps_from_another() {
        let stream = ColorHint { matrix_coefficients: None, full_range: Some(false) };
        let file = ColorHint { matrix_coefficients: Some(6), full_range: Some(true) };
        assert_eq!(stream.or(file), ColorHint { matrix_coefficients: Some(6), full_range: Some(false) });
    }
}
