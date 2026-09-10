#![forbid(unsafe_code)]

pub mod cpu;
pub mod ctx;
pub mod font;
pub mod glyph_atlas;
pub mod golden;
pub mod hud;
pub mod playfield;
pub mod result;
pub mod select;
pub mod skin;
pub mod skin_render;
pub mod theme;
pub mod toast;

pub use cpu::{CpuCanvas, apply_blend, apply_tint};
pub use ctx::{RenderCtx, with_render_ctx};
pub use font::{
    LAYOUT_CACHE_LIMIT, RUN_CACHE_LIMIT, TextContext, cache_stats, draw_text, draw_text_centered, draw_text_right, fit_text, load_font, reset_ui_family,
    set_ui_family, text_width, with_text_context,
};
pub use glyph_atlas::{ATLAS_MAX_DIM, ATLAS_TEXTURE_KEY, AtlasEntry, GlyphAtlas, GlyphAtlasBinding};
pub use golden::{GOLDEN_UPDATE_ENV, GoldenDiff, GoldenImage, GoldenOptions, PngCodec, assert_golden_png};
pub use hud::{HudPace, HudView, render_hud, render_hud_ctx};
pub use playfield::{LaneShade, PlayfieldView, render_key_bomb, render_lane_cover, render_playfield_view};
pub use result::{
    KEY_LANE_KIND, LANE_KIND_COUNT, RANK_BANDS, RATE_STEPS, ResultExtras, ResultPalette, ResultView, SCRATCH_LANE_KIND, TargetView, dj_rank, dj_rank_label,
    draw_rank_bar, draw_rank_bar_stepped, ex_delta_label, lane_kind_total, rate_27, render_result, render_result_ctx, render_result_with_palette,
    render_result_with_palette_ctx,
};
pub use select::{
    CoverState, DensityView, DetailView, RecordRowView, RecordsView, SelectDetail, SelectHot, SelectModal, SelectRow, SelectView, StatCell, cover_rect,
    render_select, render_select_ctx,
};
pub use skin::{Skin, SkinConfig, SkinError};
pub use skin_render::{
    NoExpressions, PlayTimers, SelectTimers, SkinAssets, SkinDraw, SkinExprEval, SkinFrame, SkinImage, SkinObjectKind, SkinScreen, SkinViewport,
    render_decide_screen, render_keyconfig_screen, render_play_screen, render_result_screen, render_select_screen,
};
pub use theme::{Theme, ThemeConfig, set_theme, theme};
pub use toast::{ToastLevel, ToastView, render_toasts, render_toasts_ctx, toast_color};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(235, 235, 235);
    pub const BLUE: Color = Color::rgb(70, 130, 230);
    pub const RED: Color = Color::rgb(230, 70, 70);
    pub const GREEN: Color = Color::rgb(70, 220, 120);
    pub const YELLOW: Color = Color::rgb(230, 210, 70);
    pub const ORANGE: Color = Color::rgb(230, 150, 60);
    pub const GRAY: Color = Color::rgb(90, 90, 100);
    pub const LANE_BG: Color = Color::rgb(18, 18, 24);
    pub const JUDGE_LINE: Color = Color::rgb(200, 40, 40);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    /// The overlap of two rectangles, or `None` when they do not meet. A rectangle with a
    /// non-positive extent meets nothing.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Highest value any 8-bit colour channel can hold. Blend factors are evaluated in this same
/// fixed-point range so `channel * factor / CHANNEL_MAX` stays in `u8`.
pub const CHANNEL_MAX: u32 = 255;

/// Bytes per pixel in every RGBA8 buffer this crate hands to or takes from a backend.
pub const BYTES_PER_PIXEL: usize = 4;

/// One term of a blend equation, in the OpenGL sense the reference implementation configures.
///
/// Kept as data rather than baked into either backend so the CPU reference canvas and the wgpu
/// pipelines derive their blending from the same table ([`BlendMode::factors`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    SrcAlpha,
    OneMinusSrcAlpha,
    SrcColor,
    OneMinusDstColor,
}

/// The four factors of one blend mode: `dst = src * src_factor + dst * dst_factor`, colour and
/// alpha channels weighted separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendFactors {
    pub src_color: BlendFactor,
    pub dst_color: BlendFactor,
    pub src_alpha: BlendFactor,
    pub dst_alpha: BlendFactor,
}

/// How a textured quad combines with what is already on the target.
///
/// The variants are the blend states the reference implementation actually configures
/// (`Skin.java:636-645`): skin `blend` 2 is additive, 4 multiplies, 9 inverts the destination, and
/// every other integer — 0, 1, and the undefined values — leaves the default alpha blend in place.
/// Skin `blend` 3 sets a subtract equation and restores `FUNC_ADD` before the draw, so it reaches
/// the screen as an ordinary additive quad; there is deliberately no `Subtract` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// `GL(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`. Skin blend 0, 1 and anything unrecognised.
    #[default]
    Alpha,
    /// `GL(SRC_ALPHA, ONE)`. Skin blend 2 and 3.
    Add,
    /// `GL(ZERO, SRC_COLOR)`. Skin blend 4.
    Multiply,
    /// `GL(ONE_MINUS_DST_COLOR, ZERO)`. Skin blend 9.
    InvertDst,
}

impl BlendMode {
    /// The skin file's `blend` integer mapped to a mode. Unrecognised values fall back to
    /// [`BlendMode::Alpha`], which is what leaving the blend function untouched amounts to.
    pub const fn from_skin_blend(blend: i32) -> BlendMode {
        match blend {
            SKIN_BLEND_ADD | SKIN_BLEND_SUBTRACT_RESTORED => BlendMode::Add,
            SKIN_BLEND_MULTIPLY => BlendMode::Multiply,
            SKIN_BLEND_INVERT_DST => BlendMode::InvertDst,
            _ => BlendMode::Alpha,
        }
    }

    /// The blend equation's four factors.
    ///
    /// Only [`BlendMode::Alpha`] weights the target's alpha, and it does so with the `OVER` pair
    /// (`One`, `OneMinusSrcAlpha`) the GPU pipeline uses, which leaves an already-opaque target
    /// opaque. The other three modes leave alpha alone rather than letting an additive or
    /// multiplicative draw make the screen transparent.
    pub const fn factors(self) -> BlendFactors {
        match self {
            BlendMode::Alpha => BlendFactors {
                src_color: BlendFactor::SrcAlpha,
                dst_color: BlendFactor::OneMinusSrcAlpha,
                src_alpha: BlendFactor::One,
                dst_alpha: BlendFactor::OneMinusSrcAlpha,
            },
            BlendMode::Add => {
                BlendFactors { src_color: BlendFactor::SrcAlpha, dst_color: BlendFactor::One, src_alpha: BlendFactor::Zero, dst_alpha: BlendFactor::One }
            }
            BlendMode::Multiply => {
                BlendFactors { src_color: BlendFactor::Zero, dst_color: BlendFactor::SrcColor, src_alpha: BlendFactor::Zero, dst_alpha: BlendFactor::One }
            }
            BlendMode::InvertDst => BlendFactors {
                src_color: BlendFactor::OneMinusDstColor,
                dst_color: BlendFactor::Zero,
                src_alpha: BlendFactor::Zero,
                dst_alpha: BlendFactor::One,
            },
        }
    }
}

/// Skin `blend` value for additive drawing.
const SKIN_BLEND_ADD: i32 = 2;

/// Skin `blend` value that asks for subtraction but restores `FUNC_ADD` before drawing, so it is
/// additive on screen. See [`BlendMode`].
const SKIN_BLEND_SUBTRACT_RESTORED: i32 = 3;

/// Skin `blend` value for multiplicative drawing.
const SKIN_BLEND_MULTIPLY: i32 = 4;

/// Skin `blend` value that inverts the destination.
const SKIN_BLEND_INVERT_DST: i32 = 9;

/// How a texture is sampled when the quad is not drawn at its source size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureFilter {
    /// Nearest texel. The default, and what a 1:1 blit wants.
    #[default]
    Nearest,
    /// Bilinear interpolation of the four surrounding texels, clamped at the texture edge.
    Linear,
}

/// Normalised source rectangle, `(u0, v0)` top left and `(u1, v1)` bottom right. A skin's
/// `divx`/`divy` cell is converted to one of these by the loader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl UvRect {
    /// The whole texture.
    pub const FULL: UvRect = UvRect { u0: 0.0, v0: 0.0, u1: 1.0, v1: 1.0 };

    pub const fn new(u0: f32, v0: f32, u1: f32, v1: f32) -> UvRect {
        UvRect { u0, v0, u1, v1 }
    }

    /// The rectangle covering pixels `[x, x + w) x [y, y + h)` of a `tex_w` x `tex_h` texture.
    pub fn from_pixels(x: u32, y: u32, w: u32, h: u32, tex_w: u32, tex_h: u32) -> UvRect {
        let (tw, th) = (tex_w.max(1) as f32, tex_h.max(1) as f32);
        UvRect { u0: x as f32 / tw, v0: y as f32 / th, u1: (x + w) as f32 / tw, v1: (y + h) as f32 / th }
    }
}

/// Handle to a texture a backend holds. Ids are never reused, so a handle released by one owner
/// cannot start naming another owner's texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(pub u32);

/// Everything one textured quad needs beyond the texture itself.
#[derive(Debug, Clone, Copy)]
pub struct QuadParams {
    /// Where the quad lands, in logical screen pixels.
    pub dst: Rect,
    /// Which part of the texture to read.
    pub src: UvRect,
    /// Multiplied into the sampled texel, alpha included.
    pub tint: Color,
    pub blend: BlendMode,
    pub filter: TextureFilter,
    /// Degrees, positive clockwise on screen. A skin's `angle` is counter-clockwise in a y-up
    /// space, so the loader negates it. Zero skips the rotating path entirely.
    pub angle_deg: f32,
    /// Rotation centre in pixels, relative to `dst`'s top left with y increasing downwards. A
    /// skin's `center` 0..9 is converted by [`skin_center_offset`].
    pub center: (f32, f32),
}

impl QuadParams {
    /// An unrotated, unfiltered, alpha-blended quad drawing the whole texture untinted.
    pub fn new(dst: Rect) -> QuadParams {
        QuadParams {
            dst,
            src: UvRect::FULL,
            tint: Color { r: 255, g: 255, b: 255, a: 255 },
            blend: BlendMode::Alpha,
            filter: TextureFilter::Nearest,
            angle_deg: 0.0,
            center: (0.0, 0.0),
        }
    }

    /// Whether this quad has to go through a backend's rotating path.
    pub fn is_rotated(&self) -> bool {
        self.angle_deg != 0.0
    }
}

/// Horizontal share of the destination each skin `center` anchor sits at, `SkinObject.java:80-81`.
const SKIN_CENTER_X: [f32; 10] = [0.5, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];

/// Vertical share of the destination each skin `center` anchor sits at, measured from the bottom
/// because the reference space is y-up, `SkinObject.java:80-81`.
const SKIN_CENTER_Y: [f32; 10] = [0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0];

/// The skin `center` anchor 0..9 as a pixel offset inside a `w` x `h` destination, y downwards.
///
/// The reference indexes its anchor tables directly, so a value outside 0..9 throws there; this
/// clamps to 0 (the middle) instead, which keeps one mistyped destination from killing a whole
/// skin.
pub fn skin_center_offset(center: i32, w: f32, h: f32) -> (f32, f32) {
    let i = if (0..SKIN_CENTER_X.len() as i32).contains(&center) { center as usize } else { 0 };
    (SKIN_CENTER_X[i] * w, (1.0 - SKIN_CENTER_Y[i]) * h)
}

/// The filter a background image is drawn with: point sampling only when it lands at exactly its
/// own pixel size, and linear whenever it is resized.
///
/// A background is the one quad whose size is decided by the window and the layout rather than by
/// the image, and a chart's frames or a song's cover are routinely a good deal larger than the
/// rectangle they land in. Point sampling a shrink like that turns an animated background into a
/// shimmer, so both background paths -- the built-in layout's slot and a document's own `bga`
/// object -- share this one rule.
pub fn background_filter(dst: Rect, source: (u32, u32)) -> TextureFilter {
    let (width, height) = source;
    if dst.w == width as f32 && dst.h == height as f32 { TextureFilter::Nearest } else { TextureFilter::Linear }
}

/// Backend-agnostic 2D draw surface. The wgpu backend and the CPU reference backend
/// both implement this, so the playfield composer is GPU-independent.
///
/// Draw order is z-order: a backend may merge adjacent draws into one call but must never
/// reorder them.
pub trait Renderer {
    fn size(&self) -> (u32, u32);

    /// Start a frame: paint the whole surface `color`, whatever clip is in force, and leave the
    /// clip stack empty.
    ///
    /// Both halves are part of the contract. A clip never scissors the clear, because a frame
    /// begins by wiping everything; and a frame begins unclipped, so a clip left open by the frame
    /// before it cannot silently narrow this one. Anything already submitted this frame is
    /// discarded.
    fn clear(&mut self, color: Color);

    fn fill_rect(&mut self, rect: Rect, color: Color);

    /// Take ownership of `width` x `height` RGBA8 pixels (not premultiplied) under `key`.
    ///
    /// Registering a `key` that is already known replaces its pixels and returns the handle it
    /// already had, which is how a per-frame image such as a video background is refreshed.
    ///
    /// A buffer that does not hold exactly `width * height * 4` bytes is still accepted: a short
    /// one is padded with transparent black and a long one is truncated. That is deliberate -- a
    /// frame that arrives half decoded draws dark rather than killing the process -- so a caller
    /// that cares about the length checks it before registering, because the handle that comes back
    /// says nothing about it.
    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId;

    /// Drop a texture. An unknown or already released handle is ignored.
    fn release_texture(&mut self, tex: TextureId);

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)>;

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams);

    /// Restrict drawing to `rect`, in logical screen pixels. Nested pushes intersect, so an empty
    /// intersection discards every draw until the matching [`Renderer::pop_clip`].
    fn push_clip(&mut self, rect: Rect);

    /// Undo the innermost [`Renderer::push_clip`]. Popping an empty stack is ignored.
    fn pop_clip(&mut self);
}
