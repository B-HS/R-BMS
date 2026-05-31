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
