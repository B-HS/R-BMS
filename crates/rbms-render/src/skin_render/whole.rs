//! The one offset that moves a whole play screen rather than an object of it.
//!
//! Every other offset a player sets is added to the destinations that name it. `OFFSET_ALL` is named
//! by no destination: the reference reads it when it makes the renderer of a play skin and sets the
//! sprite batch's transform from it (`Skin.ensureRenderer`), so everything the skin draws afterwards
//! -- images, text, the note field, and the clips it draws them under -- lands moved and stretched
//! by the same amount.
//!
//! The move is a share of the screen and the stretch a share of itself, both in percent, and both
//! are measured in the reference's own space: y-up, with the origin at the bottom left. So a
//! positive `y` moves the screen up, and a stretch grows away from the bottom left corner.
//!
//! Only a play skin is drawn this way (`Skin.getOffsetAll`). The reference sets the transform once,
//! on the first frame the skin is drawn, and never again; [`WholeOffset`] is what is kept from that
//! frame, and the pixels it comes to are worked out against the target of each frame, so a window
//! that changes size keeps the same share.

use rbms_skin::dst::SkinOffset;
use rbms_skin::loader::{
    SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS,
    SKIN_TYPE_PLAY_24KEYS_DOUBLE,
};

use crate::{Color, QuadParams, Rect, Renderer, TextureId};

/// What an offset's values are a hundredth of: a move of one is one percent of the screen, and a
/// stretch of one makes the screen one percent larger.
const PERCENT: f32 = 100.0;

/// The skin types the reference reads `OFFSET_ALL` for (`Skin.getOffsetAll`): every play type but
/// the battle ones.
const WHOLE_OFFSET_SKIN_TYPES: &[i32] = &[
    SKIN_TYPE_PLAY_5KEYS,
    SKIN_TYPE_PLAY_7KEYS,
    SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS,
    SKIN_TYPE_PLAY_14KEYS,
    SKIN_TYPE_PLAY_24KEYS,
    SKIN_TYPE_PLAY_24KEYS_DOUBLE,
];

/// Whether a skin of `skin_type` is drawn under the whole-screen offset at all.
pub(super) fn applies_to(skin_type: i32) -> bool {
    WHOLE_OFFSET_SKIN_TYPES.contains(&skin_type)
}

/// The whole-screen offset one skin is drawn under, as the player set it: the move as a percentage
/// of the screen and the stretch as a percentage added to it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct WholeOffset {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl WholeOffset {
    /// The offset the host answers `OFFSET_ALL` with, or no move at all when it answers none.
    pub(super) fn of(offset: Option<SkinOffset>) -> WholeOffset {
        offset.map_or_else(WholeOffset::default, |offset| WholeOffset { x: offset.x, y: offset.y, w: offset.w, h: offset.h })
    }

    /// Whether this leaves every pixel where it is, which is when a frame is drawn straight onto
    /// its target.
    pub(super) fn is_identity(&self) -> bool {
        *self == WholeOffset::default()
    }

    /// A renderer that lands everything drawn on it moved and stretched onto `inner`
    /// (`transform.set(width * x / 100, height * y / 100, ..., (w + 100) / 100, (h + 100) / 100, 1)`).
    pub(super) fn over<R: Renderer>(self, inner: &mut R) -> OffsetRenderer<'_, R> {
        let (width, height) = inner.size();
        let (width, height) = (width as f32, height as f32);
        OffsetRenderer {
            inner,
            translate: (width * self.x / PERCENT, height * self.y / PERCENT),
            scale: ((self.w + PERCENT) / PERCENT, (self.h + PERCENT) / PERCENT),
            height,
        }
    }
}

/// A [`Renderer`] that draws on another through the whole-screen offset.
///
/// It reports the size of the renderer underneath, so a skin lays itself out exactly as it would
/// without it, and every rectangle it is handed -- a fill, a quad, a clip -- is moved and stretched
/// on the way down. The reference's transform works in a y-up space, so a rectangle's distance from
/// the bottom of the screen is what is stretched and added to.
///
/// A rotated quad keeps its angle and has its destination and rotation centre stretched, as
/// [`crate::ScaledRenderer`] does: exact when both axes stretch alike, and a turned rectangle of the
/// stretched size rather than a sheared one when they do not.
pub(super) struct OffsetRenderer<'a, R: Renderer> {
    inner: &'a mut R,
    /// How far everything moves, in the target's pixels: right, and up.
    translate: (f32, f32),
    scale: (f32, f32),
    /// The target's height, which is where the y-up space the offset is measured in begins.
    height: f32,
}

impl<R: Renderer> OffsetRenderer<'_, R> {
    /// Where a rectangle of the screen lands once the whole screen has been moved and stretched.
    fn place(&self, rect: Rect) -> Rect {
        let (move_right, move_up) = self.translate;
        let (scale_x, scale_y) = self.scale;
        Rect { x: rect.x * scale_x + move_right, y: self.height - (self.height - rect.y) * scale_y - move_up, w: rect.w * scale_x, h: rect.h * scale_y }
    }
}

impl<R: Renderer> Renderer for OffsetRenderer<'_, R> {
    fn size(&self) -> (u32, u32) {
        self.inner.size()
    }

    fn clear(&mut self, color: Color) {
        self.inner.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.inner.fill_rect(self.place(rect), color);
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.inner.register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.inner.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.inner.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        let center = (params.center.0 * self.scale.0, params.center.1 * self.scale.1);
        self.inner.draw_textured_quad(tex, QuadParams { dst: self.place(params.dst), center, ..params });
    }

    fn push_clip(&mut self, rect: Rect) {
        self.inner.push_clip(self.place(rect));
    }

    fn pop_clip(&mut self) {
        self.inner.pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        self.inner.max_texture_size()
    }
}

#[cfg(test)]
mod tests {
    use rbms_skin::loader::{
        SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_RESULT,
    };

    use super::*;
    use crate::CpuCanvas;

    /// The target the offset is drawn onto: small, and no wider than it is tall by a round number,
    /// so a percentage of each side is a different count of pixels.
    const TARGET: (u32, u32) = (200, 100);

    const BACKDROP: Color = Color::rgb(10, 20, 30);
    const INK: Color = Color::rgb(200, 100, 50);

    /// The rectangle every test draws, in the target's own y-down pixels.
    const MARK: Rect = Rect { x: 20.0, y: 60.0, w: 40.0, h: 20.0 };

    fn offset(x: f32, y: f32, w: f32, h: f32) -> WholeOffset {
        WholeOffset::of(Some(SkinOffset { x, y, w, h, r: 0.0, a: 0.0 }))
    }

    /// The bounding box of everything drawn in [`INK`], as `(left, top, right, bottom)` with the far
    /// edges exclusive.
    fn inked(canvas: &CpuCanvas) -> Option<(u32, u32, u32, u32)> {
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for y in 0..TARGET.1 {
            for x in 0..TARGET.0 {
                if canvas.pixel_at(x, y) != INK {
                    continue;
                }
                bounds = Some(match bounds {
                    Some((left, top, right, bottom)) => (left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1)),
                    None => (x, y, x + 1, y + 1),
                });
            }
        }
        bounds
    }

    /// Fills [`MARK`] through `whole` and answers where it landed.
    fn marked(whole: WholeOffset) -> Option<(u32, u32, u32, u32)> {
        let mut canvas = CpuCanvas::new(TARGET.0, TARGET.1);
        canvas.clear(BACKDROP);
        whole.over(&mut canvas).fill_rect(MARK, INK);
        inked(&canvas)
    }

    #[test]
    fn only_the_play_types_the_reference_lists_are_moved() {
        for moved in WHOLE_OFFSET_SKIN_TYPES {
            assert!(applies_to(*moved), "play type {moved}");
        }
        for still in [SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_DECIDE, SKIN_TYPE_RESULT, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_PLAY_24KEYS_BATTLE] {
            assert!(!applies_to(still), "type {still} is not drawn under the whole-screen offset");
        }
    }

    #[test]
    fn a_host_with_no_offset_and_one_of_all_zeroes_leave_the_screen_alone() {
        assert!(WholeOffset::of(None).is_identity());
        assert!(offset(0.0, 0.0, 0.0, 0.0).is_identity());
        assert!(!offset(0.0, 0.0, 1.0, 0.0).is_identity(), "a stretch alone is still an offset");
        assert_eq!(marked(WholeOffset::of(None)), Some((20, 60, 60, 80)), "and what is drawn through none lands where it was put");
    }

    /// `width * x / 100` to the right and `height * y / 100` up: ten percent of a 200 by 100 target
    /// is twenty pixels across and ten pixels up.
    #[test]
    fn the_move_is_a_share_of_the_screen_and_a_positive_y_is_up() {
        assert_eq!(marked(offset(10.0, 0.0, 0.0, 0.0)), Some((40, 60, 80, 80)));
        assert_eq!(marked(offset(0.0, 10.0, 0.0, 0.0)), Some((20, 50, 60, 70)));
        assert_eq!(marked(offset(-10.0, -10.0, 0.0, 0.0)), Some((0, 70, 40, 90)));
    }

    /// `(w + 100) / 100` and `(h + 100) / 100` about the bottom left corner: fifty percent wider
    /// puts the left edge at 30 and the width at 60, and fifty percent taller lifts the rectangle's
    /// foot from 20 above the bottom to 30 above it and makes it 30 high.
    #[test]
    fn the_stretch_grows_away_from_the_bottom_left_corner() {
        assert_eq!(marked(offset(0.0, 0.0, 50.0, 0.0)), Some((30, 60, 90, 80)));
        assert_eq!(marked(offset(0.0, 0.0, 0.0, 50.0)), Some((20, 40, 60, 70)));
        assert_eq!(marked(offset(0.0, 0.0, -50.0, -50.0)), Some((10, 80, 30, 90)), "a negative stretch shrinks towards the same corner");
    }

    /// The stretch is applied before the move, as a transform matrix applies them.
    #[test]
    fn a_stretched_screen_is_moved_by_the_share_of_the_unstretched_one() {
        assert_eq!(marked(offset(10.0, 10.0, 50.0, 50.0)), Some((50, 30, 110, 60)));
    }

    #[test]
    fn a_clip_and_a_textured_quad_move_with_everything_else() {
        let whole = offset(10.0, 10.0, 0.0, 0.0);
        let mut canvas = CpuCanvas::new(TARGET.0, TARGET.1);
        canvas.clear(BACKDROP);
        {
            let mut moved = whole.over(&mut canvas);
            moved.push_clip(Rect { x: 20.0, y: 60.0, w: 10.0, h: 10.0 });
            moved.fill_rect(MARK, INK);
            moved.pop_clip();
        }
        assert_eq!(inked(&canvas), Some((40, 50, 50, 60)), "the clip landed where the rectangle it cuts did");

        let mut canvas = CpuCanvas::new(TARGET.0, TARGET.1);
        canvas.clear(BACKDROP);
        let texel = [INK.r, INK.g, INK.b, INK.a];
        {
            let mut moved = whole.over(&mut canvas);
            assert_eq!(moved.size(), TARGET, "a skin lays itself out for the target it is really drawn on");
            let texture = moved.register_texture("whole.texel", &texel, 1, 1);
            moved.draw_textured_quad(texture, QuadParams::new(MARK));
        }
        assert_eq!(inked(&canvas), Some((40, 50, 80, 70)));
    }
}
