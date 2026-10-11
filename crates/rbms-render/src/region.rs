//! A renderer that is one rectangle of another.
//!
//! Everything that draws a whole screen asks its renderer how large it is and lays itself out for
//! that: a skin document scales the size it was authored at onto [`Renderer::size`], and starts its
//! frame by clearing. [`RegionRenderer`] is what lets such a screen be drawn as a part of another
//! one. It reports the size of a rectangle of the renderer underneath, moves every rectangle it is
//! handed by that rectangle's origin, and keeps a clip open over it for as long as it lives, so
//! nothing drawn through it lands outside.
//!
//! Nothing is added to a backend for this. Moving is arithmetic on the way down, and the clip is the
//! one [`Renderer::push_clip`] every backend already has, which is why the CPU canvas and the GPU
//! draw a region alike.

use crate::{Color, DistanceFieldParams, QuadParams, Rect, Renderer, TextureId};

/// The fewest pixels a region is across or down. A rectangle with no extent still reports a size a
/// screen can divide by.
const MIN_REGION_EXTENT: f32 = 1.0;

/// Where a coordinate that is not a number is taken to be.
const UNPLACED: f32 = 0.0;

/// `rect` on whole pixels: each edge moved to the nearest one, and at least one pixel across and
/// down. An edge that is not a number is taken to be at the origin.
fn snapped(rect: Rect) -> Rect {
    let whole = |value: f32| if value.is_finite() { value.round() } else { UNPLACED };
    let (left, top) = (whole(rect.x), whole(rect.y));
    let (right, bottom) = (whole(rect.x + rect.w), whole(rect.y + rect.h));
    Rect::new(left, top, (right - left).max(MIN_REGION_EXTENT), (bottom - top).max(MIN_REGION_EXTENT))
}

/// A [`Renderer`] that draws into one rectangle of another.
///
/// It reports the rectangle's size as its own, so what is drawn on it is laid out for the rectangle
/// rather than for the whole target, and a coordinate handed to it is measured from the rectangle's
/// top left corner. The rectangle is put on whole pixels of the renderer underneath when the region
/// is made, because a size is a whole number of pixels.
///
/// A clip over the rectangle is pushed when the region is made and popped when it is dropped. The
/// clips the caller pushes nest inside it, and a [`Renderer::pop_clip`] the caller did not push is
/// ignored rather than taken out of the region's own.
///
/// [`Renderer::clear`] paints the rectangle and nothing else, and leaves the caller's clips closed:
/// what the renderer underneath has drawn outside the rectangle, and its own frame, are untouched.
/// The colour is filled like any other, so a translucent one is laid over what was there.
///
/// Textures are not a matter of coordinates and pass straight through. A rotated quad keeps its
/// angle and its rotation centre, which is measured from the quad's own corner.
pub struct RegionRenderer<'a, R: Renderer> {
    inner: &'a mut R,
    /// The rectangle on the renderer underneath, on whole pixels.
    rect: Rect,
    /// How many clips the caller has open inside the region's own.
    open_clips: usize,
}

impl<'a, R: Renderer> RegionRenderer<'a, R> {
    /// Draw on `rect` of `inner` as though it were a target of its own. `rect` is in the coordinates
    /// `inner` is drawn on in.
    pub fn new(inner: &'a mut R, rect: Rect) -> Self {
        let rect = snapped(rect);
        inner.push_clip(rect);
        RegionRenderer { inner, rect, open_clips: 0 }
    }

    /// The rectangle of the renderer underneath this draws into, on the whole pixels it was put on.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Where a rectangle of the region lands on the renderer underneath.
    fn place(&self, rect: Rect) -> Rect {
        Rect::new(rect.x + self.rect.x, rect.y + self.rect.y, rect.w, rect.h)
    }

    /// Close every clip the caller left open, leaving the region's own in force.
    fn close_clips(&mut self) {
        for _ in 0..std::mem::take(&mut self.open_clips) {
            self.inner.pop_clip();
        }
    }
}

impl<R: Renderer> Drop for RegionRenderer<'_, R> {
    fn drop(&mut self) {
        self.close_clips();
        self.inner.pop_clip();
    }
}

impl<R: Renderer> Renderer for RegionRenderer<'_, R> {
    fn size(&self) -> (u32, u32) {
        (self.rect.w as u32, self.rect.h as u32)
    }

    fn clear(&mut self, color: Color) {
        self.close_clips();
        self.inner.fill_rect(self.rect, color);
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
        self.inner.draw_textured_quad(tex, QuadParams { dst: self.place(params.dst), ..params });
    }

    fn draw_distance_field_quad(&mut self, tex: TextureId, params: DistanceFieldParams) {
        self.inner.draw_distance_field_quad(tex, DistanceFieldParams { dst: self.place(params.dst), ..params });
    }

    fn push_clip(&mut self, rect: Rect) {
        self.open_clips += 1;
        self.inner.push_clip(self.place(rect));
    }

    fn pop_clip(&mut self) {
        if self.open_clips == 0 {
            return;
        }
        self.open_clips -= 1;
        self.inner.pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        self.inner.max_texture_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CpuCanvas, ScaledRenderer, UNLIMITED_TEXTURE_SIZE};

    /// The target the regions of these tests are cut out of.
    const TARGET: (u32, u32) = (96, 54);

    /// A region well inside the target, on whole pixels, in the shape of the target.
    const REGION: Rect = Rect { x: 32.0, y: 9.0, w: 48.0, h: 27.0 };

    const BACKDROP: Color = Color::rgb(10, 20, 30);
    const WASH: Color = Color::rgb(40, 40, 60);
    const INK: Color = Color::rgb(200, 100, 50);

    /// Edge of the checker texture the quad tests sample.
    const CHECKER: u32 = 4;

    /// A texture with a different colour in every texel, so a quad drawn from the wrong place cannot
    /// come out looking right.
    fn checker() -> Vec<u8> {
        (0..CHECKER * CHECKER).flat_map(|i| [(i * 16) as u8, 255 - (i * 16) as u8, (i * 5) as u8, 255]).collect()
    }

    /// A little of everything a screen draws, laid out for a target the size of [`REGION`]: a wash,
    /// a rectangle, a clipped one that would otherwise cover everything, and a textured quad.
    fn paint<R: Renderer>(r: &mut R) {
        r.clear(WASH);
        r.fill_rect(Rect::new(2.0, 3.0, 10.0, 6.0), INK);
        r.push_clip(Rect::new(20.0, 2.0, 8.0, 8.0));
        r.fill_rect(Rect::new(-100.0, -100.0, 400.0, 400.0), Color::GREEN);
        r.pop_clip();
        let tex = r.register_texture("region.checker", &checker(), CHECKER, CHECKER);
        r.draw_textured_quad(tex, QuadParams::new(Rect::new(36.0, 16.0, 8.0, 8.0)));
    }

    fn target() -> CpuCanvas {
        let mut canvas = CpuCanvas::new(TARGET.0, TARGET.1);
        canvas.clear(BACKDROP);
        canvas
    }

    #[test]
    fn a_region_reports_its_own_size_and_not_the_targets() {
        let mut canvas = target();
        let region = RegionRenderer::new(&mut canvas, REGION);
        assert_eq!(region.size(), (REGION.w as u32, REGION.h as u32));
        assert_eq!(region.rect(), REGION);
    }

    /// The property a screen drawn in a region depends on: every pixel of the region is the pixel a
    /// target of the region's own size would have had, and no pixel outside it is touched.
    #[test]
    fn a_frame_drawn_in_a_region_is_the_frame_drawn_on_a_target_of_that_size() {
        let mut alone = CpuCanvas::new(REGION.w as u32, REGION.h as u32);
        paint(&mut alone);

        let mut canvas = target();
        paint(&mut RegionRenderer::new(&mut canvas, REGION));

        let (left, top) = (REGION.x as u32, REGION.y as u32);
        for y in 0..TARGET.1 {
            for x in 0..TARGET.0 {
                let inside = x >= left && x < left + REGION.w as u32 && y >= top && y < top + REGION.h as u32;
                let expected = if inside { alone.pixel_at(x - left, y - top) } else { BACKDROP };
                assert_eq!(canvas.pixel_at(x, y), expected, "pixel ({x}, {y})");
            }
        }
    }

    #[test]
    fn nothing_drawn_through_a_region_lands_outside_it() {
        let mut canvas = target();
        {
            let mut region = RegionRenderer::new(&mut canvas, REGION);
            region.fill_rect(Rect::new(-500.0, -500.0, 2000.0, 2000.0), INK);
            let tex = region.register_texture("region.checker", &checker(), CHECKER, CHECKER);
            region.draw_textured_quad(tex, QuadParams::new(Rect::new(-20.0, -20.0, 200.0, 200.0)));
        }
        assert_eq!(canvas.pixel_at(REGION.x as u32 - 1, REGION.y as u32), BACKDROP);
        assert_eq!(canvas.pixel_at(REGION.x as u32, REGION.y as u32 - 1), BACKDROP);
        assert_eq!(canvas.pixel_at((REGION.x + REGION.w) as u32, (REGION.y + REGION.h) as u32 - 1), BACKDROP);
        assert_eq!(canvas.pixel_at((REGION.x + REGION.w) as u32 - 1, (REGION.y + REGION.h) as u32), BACKDROP);
        assert_ne!(canvas.pixel_at(REGION.x as u32, REGION.y as u32), BACKDROP, "the region itself was drawn on");
    }

    /// A region ends where it is dropped: what is drawn on the target afterwards is clipped by
    /// nothing, even when the screen drawn in the region left a clip of its own open.
    #[test]
    fn dropping_a_region_gives_the_target_back_unclipped() {
        let mut canvas = target();
        {
            let mut region = RegionRenderer::new(&mut canvas, REGION);
            region.push_clip(Rect::new(0.0, 0.0, 4.0, 4.0));
            region.push_clip(Rect::new(0.0, 0.0, 2.0, 2.0));
        }
        canvas.fill_rect(Rect::new(0.0, 0.0, TARGET.0 as f32, TARGET.1 as f32), INK);
        assert_eq!(canvas.pixel_at(0, 0), INK);
        assert_eq!(canvas.pixel_at(TARGET.0 - 1, TARGET.1 - 1), INK);
    }

    /// A screen that pops a clip it never pushed must not open the region up to the whole target.
    #[test]
    fn a_pop_the_caller_did_not_push_leaves_the_regions_own_clip_in_force() {
        let mut canvas = target();
        {
            let mut region = RegionRenderer::new(&mut canvas, REGION);
            region.pop_clip();
            region.pop_clip();
            region.fill_rect(Rect::new(-500.0, -500.0, 2000.0, 2000.0), INK);
        }
        assert_eq!(canvas.pixel_at(0, 0), BACKDROP);
        assert_eq!(canvas.pixel_at(REGION.x as u32, REGION.y as u32), INK);
    }

    /// A clear starts the region's frame and nobody else's: it paints the whole region whatever clip
    /// the caller had open, closes that clip, and leaves the rest of the target as it was.
    #[test]
    fn a_clear_paints_the_whole_region_and_only_the_region() {
        let mut canvas = target();
        canvas.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), INK);
        {
            let mut region = RegionRenderer::new(&mut canvas, REGION);
            region.push_clip(Rect::new(0.0, 0.0, 1.0, 1.0));
            region.clear(WASH);
            region.fill_rect(Rect::new(4.0, 4.0, 2.0, 2.0), INK);
        }
        assert_eq!(canvas.pixel_at(4, 4), INK, "what the target drew before the region is still there");
        assert_eq!(canvas.pixel_at(REGION.x as u32, REGION.y as u32), WASH);
        assert_eq!(canvas.pixel_at((REGION.x + REGION.w) as u32 - 1, (REGION.y + REGION.h) as u32 - 1), WASH);
        assert_eq!(canvas.pixel_at(REGION.x as u32 + 4, REGION.y as u32 + 4), INK, "the clip open before the clear was closed by it");
        assert_eq!(canvas.pixel_at(TARGET.0 - 1, 0), BACKDROP);
    }

    #[test]
    fn a_region_is_put_on_whole_pixels() {
        let mut canvas = target();
        let region = RegionRenderer::new(&mut canvas, Rect::new(10.4, 5.6, 20.3, 10.2));
        assert_eq!(region.rect(), Rect::new(10.0, 6.0, 21.0, 10.0));
        assert_eq!(region.size(), (21, 10));
    }

    #[test]
    fn a_region_with_no_extent_or_no_place_still_has_a_size() {
        let mut canvas = target();
        assert_eq!(RegionRenderer::new(&mut canvas, Rect::new(4.0, 4.0, 0.0, -3.0)).size(), (1, 1));
        let lost = RegionRenderer::new(&mut canvas, Rect::new(f32::NAN, f32::INFINITY, f32::NAN, 8.0));
        assert_eq!((lost.rect().x, lost.rect().y), (0.0, 0.0));
        assert_eq!(lost.size(), (1, 1));
    }

    /// A rotation centre is measured from the quad's own corner, so moving the quad must not move it.
    #[test]
    fn a_rotated_quad_turns_about_the_same_point_of_itself() {
        let turned = QuadParams { angle_deg: 90.0, center: (4.0, 4.0), ..QuadParams::new(Rect::new(20.0, 12.0, 8.0, 8.0)) };

        let mut alone = CpuCanvas::new(REGION.w as u32, REGION.h as u32);
        alone.clear(BACKDROP);
        let tex = alone.register_texture("region.checker", &checker(), CHECKER, CHECKER);
        alone.draw_textured_quad(tex, turned);

        let mut canvas = target();
        {
            let mut region = RegionRenderer::new(&mut canvas, REGION);
            let tex = region.register_texture("region.checker", &checker(), CHECKER, CHECKER);
            region.draw_textured_quad(tex, turned);
        }
        for (x, y) in [(21, 13), (26, 13), (21, 18), (26, 18)] {
            assert_eq!(canvas.pixel_at(x + REGION.x as u32, y + REGION.y as u32), alone.pixel_at(x, y), "the quarter of the quad at ({x}, {y})");
        }
    }

    /// A region composes with the adapter the built-in screens are drawn through: a rectangle given
    /// in the fixed space lands where that space is scaled to, and what is drawn in it is laid out
    /// for the pixels it really has.
    #[test]
    fn a_region_of_a_scaled_renderer_is_placed_by_the_scale_and_drawn_in_real_pixels() {
        const ENLARGEMENT: u32 = 2;
        let mut canvas = CpuCanvas::new(TARGET.0 * ENLARGEMENT, TARGET.1 * ENLARGEMENT);
        canvas.clear(BACKDROP);
        let scale = ScaledRenderer::new(&mut canvas, TARGET).scale();
        let placed = crate::scale_rect(REGION, scale);
        {
            let mut region = RegionRenderer::new(&mut canvas, placed);
            assert_eq!(region.size(), (REGION.w as u32 * ENLARGEMENT, REGION.h as u32 * ENLARGEMENT));
            region.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), INK);
        }
        assert_eq!(canvas.pixel_at(REGION.x as u32 * ENLARGEMENT, REGION.y as u32 * ENLARGEMENT), INK);
        assert_eq!(
            canvas.pixel_at(REGION.x as u32 * ENLARGEMENT + 1, REGION.y as u32 * ENLARGEMENT),
            BACKDROP,
            "one pixel of the region is one pixel of the target"
        );
    }

    #[test]
    fn textures_and_their_limit_pass_straight_through() {
        let mut canvas = target();
        {
            let mut region = RegionRenderer::new(&mut canvas, REGION);
            let tex = region.register_texture("region.checker", &checker(), CHECKER, CHECKER);
            assert_eq!(region.texture_size(tex), Some((CHECKER, CHECKER)));
            assert_eq!(region.max_texture_size(), UNLIMITED_TEXTURE_SIZE);
            region.release_texture(tex);
            assert_eq!(region.texture_size(tex), None);
        }
        assert_eq!(canvas.live_texture_count(), 0);
    }
}
