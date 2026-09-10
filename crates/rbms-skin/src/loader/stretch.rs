//! How a drawn rectangle and its sampling filter are chosen once the destination is resolved.
//!
//! Both rules come from the reference implementation: `StretchType`'s eleven fit modes, and the
//! `dstfilter`-to-image-type switch in `SkinObject.draw`. They are pure arithmetic on sizes, which
//! is why they live in the data crate rather than in the renderer.

use crate::dst::SkinRect;

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
    /// As [`Self::FitOuter`], with the overflow trimmed out of the source instead.
    FitOuterTrimmed,
    /// Keep the aspect ratio and match the rectangle's width.
    FitWidth,
    /// As [`Self::FitWidth`], trimming the source.
    FitWidthTrimmed,
    /// Keep the aspect ratio and match the rectangle's height.
    FitHeight,
    /// As [`Self::FitHeight`], trimming the source.
    FitHeightTrimmed,
    /// Keep the aspect ratio and shrink to fit, but never enlarge.
    NoExpanding,
    /// Draw at the source's own pixel size, centred.
    NoResize,
    /// As [`Self::NoResize`], trimming the source.
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

    /// Whether [`stretch_rect`] reproduces this mode rather than falling back to plain stretching.
    ///
    /// The trimming modes narrow the source region instead of the destination rectangle, and their
    /// helper arithmetic is not settled yet; a document that asks for one is drawn stretched and
    /// warned about rather than dropped.
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Stretch | Self::FitInner | Self::FitOuter | Self::NoResize)
    }
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

/// The rectangle an image of `source` pixels is actually drawn into.
///
/// A source with no area, or a mode this build does not reproduce, leaves the rectangle as the
/// destination resolved it.
pub fn stretch_rect(kind: StretchKind, rect: SkinRect, source: (f32, f32)) -> SkinRect {
    let (source_w, source_h) = source;
    if source_w <= 0.0 || source_h <= 0.0 {
        return rect;
    }
    let scale_x = rect.w / source_w;
    let scale_y = rect.h / source_h;
    match kind {
        StretchKind::FitInner => {
            if scale_x <= scale_y {
                fit_height(rect, source_h * scale_x)
            } else {
                fit_width(rect, source_w * scale_y)
            }
        }
        StretchKind::FitOuter => {
            if scale_x >= scale_y {
                fit_height(rect, source_h * scale_x)
            } else {
                fit_width(rect, source_w * scale_y)
            }
        }
        StretchKind::NoResize => fit_height(fit_width(rect, source_w), source_h),
        _ => rect,
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
/// The reference reaches for a dedicated bilinear shader when a filtered image is resized; this
/// approximates that with hardware linear sampling, which is the one visible divergence in the
/// skin pipeline and is recorded as such.
pub fn filtering_for(dstfilter: i32, rect: SkinRect, source: (f32, f32)) -> Filtering {
    let (source_w, source_h) = source;
    let one_to_one = rect.w == source_w && rect.h == source_h;
    if dstfilter == 0 || one_to_one { Filtering::Nearest } else { Filtering::Linear }
}
