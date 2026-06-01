use crate::{Color, Rect, Renderer};

/// Software RGBA8 canvas. Deterministic reference backend for tests and headless checks.
pub struct CpuCanvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl CpuCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        CpuCanvas { width, height, pixels: vec![0; (width * height * 4) as usize] }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel_at(&self, x: u32, y: u32) -> Color {
        let i = ((y * self.width + x) * 4) as usize;
        Color { r: self.pixels[i], g: self.pixels[i + 1], b: self.pixels[i + 2], a: self.pixels[i + 3] }
    }
}

impl Renderer for CpuCanvas {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn clear(&mut self, color: Color) {
        for px in self.pixels.chunks_exact_mut(4) {
            px[0] = color.r;
            px[1] = color.g;
            px[2] = color.b;
            px[3] = color.a;
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x0 = rect.x.floor().max(0.0) as i64;
        let y0 = rect.y.floor().max(0.0) as i64;
        let x1 = (rect.x + rect.w).ceil().min(self.width as f32) as i64;
        let y1 = (rect.y + rect.h).ceil().min(self.height as f32) as i64;
        let sa = color.a as u32;
        let ia = 255 - sa;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y as u32 * self.width + x as u32) * 4) as usize;
                if sa == 255 {
                    self.pixels[i] = color.r;
                    self.pixels[i + 1] = color.g;
                    self.pixels[i + 2] = color.b;
                } else {
                    self.pixels[i] = ((color.r as u32 * sa + self.pixels[i] as u32 * ia) / 255) as u8;
                    self.pixels[i + 1] = ((color.g as u32 * sa + self.pixels[i + 1] as u32 * ia) / 255) as u8;
                    self.pixels[i + 2] = ((color.b as u32 * sa + self.pixels[i + 2] as u32 * ia) / 255) as u8;
                }
                self.pixels[i + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_canvas_is_zeroed_and_sized() {
        let c = CpuCanvas::new(4, 3);
        assert_eq!(c.size(), (4, 3));
        assert_eq!(c.pixels().len(), 4 * 3 * 4);
        assert!(c.pixels().iter().all(|&b| b == 0), "fresh canvas is fully zeroed");
        assert_eq!(c.pixel_at(0, 0), Color { r: 0, g: 0, b: 0, a: 0 });
        assert_eq!(c.pixel_at(3, 2), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    fn clear_sets_every_pixel_including_alpha() {
        let mut c = CpuCanvas::new(2, 2);
        let col = Color { r: 10, g: 20, b: 30, a: 200 };
        c.clear(col);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(c.pixel_at(x, y), col, "clear writes the full RGBA at ({x},{y})");
            }
        }
    }

    #[test]
    fn fill_rect_opaque_writes_rgb_and_forces_alpha_255() {
        let mut c = CpuCanvas::new(4, 4);
        // Opaque source over a transparent canvas: rgb is copied, stored alpha forced to 255.
        c.fill_rect(Rect::new(1.0, 1.0, 2.0, 2.0), Color { r: 50, g: 60, b: 70, a: 255 });
        assert_eq!(c.pixel_at(1, 1), Color { r: 50, g: 60, b: 70, a: 255 });
        assert_eq!(c.pixel_at(2, 2), Color { r: 50, g: 60, b: 70, a: 255 });
        // Outside the rect remains untouched (still zeroed).
        assert_eq!(c.pixel_at(0, 0), Color { r: 0, g: 0, b: 0, a: 0 });
        assert_eq!(c.pixel_at(3, 3), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    fn fill_rect_alpha_zero_is_a_noop_on_rgb_but_sets_alpha_255() {
        let mut c = CpuCanvas::new(2, 2);
        c.clear(Color { r: 100, g: 110, b: 120, a: 100 });
        // a=0 takes the blend branch: rgb = (c*0 + old*255)/255 = old, so rgb is unchanged.
        c.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color { r: 5, g: 5, b: 5, a: 0 });
        // NOTE: a=0 is NOT a true no-op — the alpha channel is still forced to 255 (rgb preserved).
        assert_eq!(c.pixel_at(0, 0), Color { r: 100, g: 110, b: 120, a: 255 });
    }

    #[test]
    fn fill_rect_partial_alpha_matches_integer_source_over_math() {
        let mut c = CpuCanvas::new(1, 1);
        // Start from opaque black background.
        c.clear(Color { r: 0, g: 0, b: 0, a: 255 });
        let src = Color { r: 235, g: 200, b: 100, a: 128 };
        c.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), src);
        let sa = 128u32;
        let ia = 255 - sa;
        let expect = |s: u8, d: u8| ((s as u32 * sa + d as u32 * ia) / 255) as u8;
        assert_eq!(c.pixel_at(0, 0), Color { r: expect(235, 0), g: expect(200, 0), b: expect(100, 0), a: 255 });
    }

    #[test]
    fn fill_rect_partial_alpha_blends_toward_source_over_repeated_fills() {
        // Repeatedly blending the same translucent colour monotonically approaches it.
        let mut c = CpuCanvas::new(1, 1);
        c.clear(Color { r: 0, g: 0, b: 0, a: 255 });
        let src = Color { r: 255, g: 0, b: 0, a: 60 };
        let mut prev = 0u8;
        for _ in 0..20 {
            c.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), src);
            let r = c.pixel_at(0, 0).r;
            assert!(r >= prev, "red channel never decreases blending toward 255 (prev={prev} now={r})");
            prev = r;
        }
        assert!(prev > 0, "after many blends the pixel is clearly tinted red (got {prev})");
    }

    #[test]
    fn fill_rect_clips_negative_origin_to_canvas() {
        let mut c = CpuCanvas::new(3, 3);
        // Rect starts off the top-left; only the in-bounds portion is drawn, no panic.
        c.fill_rect(Rect::new(-5.0, -5.0, 7.0, 7.0), Color::rgb(9, 9, 9));
        assert_eq!(c.pixel_at(0, 0), Color { r: 9, g: 9, b: 9, a: 255 });
        assert_eq!(c.pixel_at(1, 1), Color { r: 9, g: 9, b: 9, a: 255 });
        // (2,2): rect covers x<2,y<2 (ceil(-5+7)=2 exclusive) so this corner is untouched.
        assert_eq!(c.pixel_at(2, 2), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    fn fill_rect_clips_oversized_rect_to_canvas_bounds_without_panic() {
        let mut c = CpuCanvas::new(2, 2);
        // Rect far larger than the canvas: x1/y1 clamp to width/height, so no out-of-range index.
        c.fill_rect(Rect::new(0.0, 0.0, 1000.0, 1000.0), Color::rgb(7, 8, 9));
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(c.pixel_at(x, y), Color { r: 7, g: 8, b: 9, a: 255 });
            }
        }
    }

    #[test]
    fn fill_rect_fully_out_of_bounds_draws_nothing() {
        let mut c = CpuCanvas::new(4, 4);
        c.clear(Color { r: 1, g: 2, b: 3, a: 255 });
        // Entirely to the right / below the canvas — clamped ranges are empty, canvas unchanged.
        c.fill_rect(Rect::new(100.0, 100.0, 10.0, 10.0), Color::rgb(200, 200, 200));
        c.fill_rect(Rect::new(-50.0, 0.0, 10.0, 4.0), Color::rgb(200, 200, 200));
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(c.pixel_at(x, y), Color { r: 1, g: 2, b: 3, a: 255 });
            }
        }
    }

    #[test]
    fn fill_rect_zero_and_negative_size_draws_nothing() {
        let mut c = CpuCanvas::new(3, 3);
        c.clear(Color { r: 4, g: 5, b: 6, a: 255 });
        // Zero width/height: x0==x1 (or y0==y1) so the loop body never runs.
        c.fill_rect(Rect::new(1.0, 1.0, 0.0, 0.0), Color::rgb(99, 99, 99));
        // Negative width: x1 = ceil(1 + -1) = 0 < x0=1, empty range.
        c.fill_rect(Rect::new(1.0, 1.0, -1.0, 2.0), Color::rgb(99, 99, 99));
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(c.pixel_at(x, y), Color { r: 4, g: 5, b: 6, a: 255 });
            }
        }
    }

    #[test]
    fn fill_rect_fractional_coords_floor_origin_ceil_extent() {
        let mut c = CpuCanvas::new(8, 8);
        // x: floor(1.9)=1 .. ceil(1.9+2.2)=ceil(4.1)=5 -> columns 1,2,3,4
        // y: floor(2.1)=2 .. ceil(2.1+1.0)=ceil(3.1)=4 -> rows 2,3
        c.fill_rect(Rect::new(1.9, 2.1, 2.2, 1.0), Color::rgb(120, 130, 140));
        let lit = |x, y| c.pixel_at(x, y) == Color { r: 120, g: 130, b: 140, a: 255 };
        assert!(lit(1, 2) && lit(4, 2) && lit(1, 3) && lit(4, 3), "covered cells are the floor..ceil span");
        assert!(!lit(0, 2), "column left of floor(x) is untouched");
        assert!(!lit(5, 2), "column at ceil(x+w) is exclusive");
        assert!(!lit(1, 1), "row above floor(y) is untouched");
        assert!(!lit(1, 4), "row at ceil(y+h) is exclusive");
    }

    #[test]
    fn fill_rect_subpixel_rect_still_paints_at_least_one_cell() {
        let mut c = CpuCanvas::new(4, 4);
        // A rect thinner than a pixel: floor(1.2)=1 .. ceil(1.2+0.1)=ceil(1.3)=2 -> exactly column 1.
        c.fill_rect(Rect::new(1.2, 1.2, 0.1, 0.1), Color::rgb(77, 77, 77));
        assert_eq!(c.pixel_at(1, 1), Color { r: 77, g: 77, b: 77, a: 255 }, "sub-pixel rect rounds up to one cell");
        assert_eq!(c.pixel_at(2, 2), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    #[should_panic]
    fn pixel_at_out_of_bounds_panics() {
        let c = CpuCanvas::new(2, 2);
        // pixel_at does no bounds check; reading past the buffer panics on slice index.
        let _ = c.pixel_at(2, 2);
    }
}
