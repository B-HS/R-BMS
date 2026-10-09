//! How a drawn rectangle and its sampling filter are chosen once the destination is resolved.
//!
//! Both rules come from the reference implementation: `StretchType`'s eleven fit modes, and the
//! `dstfilter`-to-image-type switch in `SkinObject.draw`. They are pure arithmetic on sizes, which
//! is why they live in the data crate rather than in the renderer.

use crate::dst::SkinRect;

/// The scale [`StretchKind::NoExpanding`] never goes above: an image is shrunk to fit, not enlarged.
const NO_EXPANDING_MAX_SCALE: f32 = 1.0;

/// The scale the modes that do not resize an image draw it at.
const NO_RESIZE_SCALE: f32 = 1.0;

/// How an image is fitted into the rectangle a destination resolved to (`StretchType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StretchKind {
    /// Fill the rectangle, aspect ratio be damned. The default, and what an unset `stretch` means.
    #[default]
    Stretch,
    /// Keep the aspect ratio and fit inside the rectangle, letterboxing it.
    FitInner,
    /// Keep the aspect ratio and cover the rectangle, overflowing it.
    FitOuter,
    /// As [`Self::FitOuter`], with the overflow trimmed out of the source instead of drawn past the
    /// rectangle.
    FitOuterTrimmed,
    /// Keep the aspect ratio and match the rectangle's width.
    FitWidth,
    /// As [`Self::FitWidth`], trimming the source where the image would be taller than the rectangle.
    FitWidthTrimmed,
    /// Keep the aspect ratio and match the rectangle's height.
    FitHeight,
    /// As [`Self::FitHeight`], trimming the source where the image would be wider than the rectangle.
    FitHeightTrimmed,
    /// Keep the aspect ratio and shrink to fit, but never enlarge.
    NoExpanding,
    /// Draw at the source's own pixel size, centred.
    NoResize,
    /// As [`Self::NoResize`], trimming the source where the image is larger than the rectangle.
    NoResizeTrimmed,
}

impl StretchKind {
    /// Reads the document's integer. An unset or unknown value stretches, as the reference's
    /// `stretch = -1` default does.
    pub const fn from_id(value: i32) -> Self {
        match value {
            1 => Self::FitInner,
            2 => Self::FitOuter,
            3 => Self::FitOuterTrimmed,
            4 => Self::FitWidth,
            5 => Self::FitWidthTrimmed,
            6 => Self::FitHeight,
            7 => Self::FitHeightTrimmed,
            8 => Self::NoExpanding,
            9 => Self::NoResize,
            10 => Self::NoResizeTrimmed,
            _ => Self::Stretch,
        }
    }
}

/// Java's `(int)` cast of a float: towards zero, saturating at the type's ends, and zero for a NaN.
/// The trimming helpers land a source region on whole pixels with it.
fn java_int(value: f32) -> f32 {
    value as i32 as f32
}

/// Resizes a rectangle about its own centre, the way `StretchType.fitWidth` does.
fn fit_width(rect: SkinRect, width: f32) -> SkinRect {
    let centre = rect.x + rect.w / 2.0;
    SkinRect { x: centre - width / 2.0, y: rect.y, w: width, h: rect.h }
}

/// Resizes a rectangle about its own centre, the way `StretchType.fitHeight` does.
fn fit_height(rect: SkinRect, height: f32) -> SkinRect {
    let centre = rect.y + rect.h / 2.0;
    SkinRect { x: rect.x, y: centre - height / 2.0, w: rect.w, h: height }
}

/// `StretchType.fitWidthTrimmed`: an image that would overflow the rectangle sideways at `scale`
/// loses the overflow from its source instead, about the source's own centre and on whole pixels;
/// one that would not is drawn at its scaled width, centred.
fn fit_width_trimmed(rect: SkinRect, scale: f32, source: SkinRect) -> (SkinRect, SkinRect) {
    let width = scale * source.w;
    if rect.w < width {
        let centre = source.x + source.w * 0.5;
        let visible = rect.w / scale;
        (rect, SkinRect { x: java_int(centre - visible * 0.5), w: java_int(visible), ..source })
    } else {
        (fit_width(rect, width), source)
    }
}

/// `StretchType.fitHeightTrimmed`, the vertical twin of [`fit_width_trimmed`].
fn fit_height_trimmed(rect: SkinRect, scale: f32, source: SkinRect) -> (SkinRect, SkinRect) {
    let height = scale * source.h;
    if rect.h < height {
        let centre = source.y + source.h * 0.5;
        let visible = rect.h / scale;
        (rect, SkinRect { y: java_int(centre - visible * 0.5), h: java_int(visible), ..source })
    } else {
        (fit_height(rect, height), source)
    }
}

/// The rectangle an image is actually drawn into and the part of its source that is read, in that
/// order (`StretchType.stretchRect`).
///
/// `source` is the image's region in its texture's pixels. Only the trimming modes change it; every
/// other mode hands it back as it came. Each arm is the reference's expression, operand order
/// included. A source with no area leaves both rectangles as they came, since every mode but the
/// first divides by its size.
pub fn stretch_rect(kind: StretchKind, rect: SkinRect, source: SkinRect) -> (SkinRect, SkinRect) {
    if source.w <= 0.0 || source.h <= 0.0 {
        return (rect, source);
    }
    let scale_x = rect.w / source.w;
    let scale_y = rect.h / source.h;
    match kind {
        StretchKind::Stretch => (rect, source),
        StretchKind::FitInner => {
            let fitted = if scale_x <= scale_y { fit_height(rect, source.h * scale_x) } else { fit_width(rect, source.w * scale_y) };
            (fitted, source)
        }
        StretchKind::FitOuter => {
            let fitted = if scale_x >= scale_y { fit_height(rect, source.h * scale_x) } else { fit_width(rect, source.w * scale_y) };
            (fitted, source)
        }
        StretchKind::FitOuterTrimmed => {
            if scale_x >= scale_y {
                fit_height_trimmed(rect, scale_x, source)
            } else {
                fit_width_trimmed(rect, scale_y, source)
            }
        }
        StretchKind::FitWidth => (fit_height(rect, source.h * rect.w / source.w), source),
        StretchKind::FitWidthTrimmed => fit_height_trimmed(rect, scale_x, source),
        StretchKind::FitHeight => (fit_width(rect, source.w * rect.h / source.h), source),
        StretchKind::FitHeightTrimmed => fit_width_trimmed(rect, scale_y, source),
        StretchKind::NoExpanding => {
            let scale = NO_EXPANDING_MAX_SCALE.min(scale_x.min(scale_y));
            (fit_height(fit_width(rect, source.w * scale), source.h * scale), source)
        }
        StretchKind::NoResize => (fit_height(fit_width(rect, source.w), source.h), source),
        StretchKind::NoResizeTrimmed => {
            let (rect, source) = fit_width_trimmed(rect, NO_RESIZE_SCALE, source);
            fit_height_trimmed(rect, NO_RESIZE_SCALE, source)
        }
    }
}

/// How a texture is sampled when it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filtering {
    /// Take the nearest texel. Sharp, and what a document that asks for no filtering gets.
    #[default]
    Nearest,
    /// Blend the four surrounding texels.
    Linear,
}

/// The filter a destination draws with, following `SkinObject.draw`'s `dstfilter` switch.
///
/// `rect` and `source` are the pair [`stretch_rect`] answered with, the source by its size alone:
/// the reference compares the fitted rectangle with the region it is about to read, trimmed or not.
///
/// The reference reaches for a dedicated bilinear shader when a filtered image is resized; this
/// approximates that with hardware linear sampling, which is the one visible divergence in the
/// skin pipeline and is recorded as such.
pub fn filtering_for(dstfilter: i32, rect: SkinRect, source: (f32, f32)) -> Filtering {
    let (source_w, source_h) = source;
    let one_to_one = rect.w == source_w && rect.h == source_h;
    if dstfilter == 0 || one_to_one { Filtering::Nearest } else { Filtering::Linear }
}
