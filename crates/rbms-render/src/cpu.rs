use std::collections::HashMap;

use crate::{BYTES_PER_PIXEL, BlendFactor, BlendMode, CHANNEL_MAX, Color, QuadParams, Rect, Renderer, TextureFilter, TextureId, UvRect};

/// Bits dropped from each averaged channel in [`CpuCanvas::block_signature`]. Quantizing to
/// `256 >> SIGNATURE_QUANT_SHIFT` levels keeps a golden signature stable against sub-pixel
/// antialiasing differences while still catching layout, colour and content regressions. Three
/// bits (32 levels per channel) is fine enough that a single relabelled text run moves the block
/// mean past a step, which four bits was measured not to do.
const SIGNATURE_QUANT_SHIFT: u32 = 3;

/// FNV-1a 64-bit parameters, used to fold a block signature into one comparable number.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// One registered texture: RGBA8 pixels, not premultiplied.
struct Texture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Multiply a sampled texel by a quad's tint, channel by channel including alpha. `255` is the
/// identity, so an untinted quad samples through unchanged.
pub fn apply_tint(src: Color, tint: Color) -> Color {
    let mul = |a: u8, b: u8| ((a as u32 * b as u32) / CHANNEL_MAX) as u8;
    Color { r: mul(src.r, tint.r), g: mul(src.g, tint.g), b: mul(src.b, tint.b), a: mul(src.a, tint.a) }
}

/// Evaluate one blend factor in `0..=CHANNEL_MAX` fixed point for a single channel.
fn blend_factor(f: BlendFactor, src_channel: u32, src_alpha: u32, dst_channel: u32) -> u32 {
    match f {
        BlendFactor::Zero => 0,
        BlendFactor::One => CHANNEL_MAX,
        BlendFactor::SrcAlpha => src_alpha,
        BlendFactor::OneMinusSrcAlpha => CHANNEL_MAX - src_alpha,
        BlendFactor::SrcColor => src_channel,
        BlendFactor::OneMinusDstColor => CHANNEL_MAX - dst_channel,
    }
}

fn blend_channel(src: u8, src_alpha: u8, dst: u8, src_factor: BlendFactor, dst_factor: BlendFactor) -> u8 {
    let (s, sa, d) = (src as u32, src_alpha as u32, dst as u32);
    let weighted = s * blend_factor(src_factor, s, sa, d) + d * blend_factor(dst_factor, s, sa, d);
    (weighted / CHANNEL_MAX).min(CHANNEL_MAX) as u8
}

/// Combine an already-tinted source colour with what the target holds, under one blend mode.
///
/// This is the reference the golden images are measured against, and the same
/// [`BlendMode::factors`] table the GPU pipelines are built from, so both backends agree by
/// construction. Weighted sums are truncated in `0..=CHANNEL_MAX` fixed point and saturated at the
/// top, matching [`CpuCanvas::fill_rect`]'s existing source-over arithmetic exactly for
/// [`BlendMode::Alpha`].
pub fn apply_blend(src: Color, dst: Color, mode: BlendMode) -> Color {
    let f = mode.factors();
    Color {
        r: blend_channel(src.r, src.a, dst.r, f.src_color, f.dst_color),
        g: blend_channel(src.g, src.a, dst.g, f.src_color, f.dst_color),
        b: blend_channel(src.b, src.a, dst.b, f.src_color, f.dst_color),
        a: blend_channel(src.a, src.a, dst.a, f.src_alpha, f.dst_alpha),
    }
}

/// Software RGBA8 canvas. Deterministic reference backend for tests and headless checks, and the
/// truth the golden images are compared against.
pub struct CpuCanvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    /// Registered textures by handle. A released slot stays as `None` so its id is never handed out
    /// again.
    textures: Vec<Option<Texture>>,
    keys: HashMap<String, TextureId>,
    /// One entry per live `push_clip`, each already intersected with the clip below it. `None`
    /// means the intersection came out empty and nothing may draw.
    clips: Vec<Option<Rect>>,
}

impl CpuCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        CpuCanvas {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * BYTES_PER_PIXEL],
            textures: Vec::new(),
            keys: HashMap::new(),
            clips: Vec::new(),
        }
    }

    /// How many textures are live, so a test can prove a reload released what it replaced.
    pub fn live_texture_count(&self) -> usize {
        self.textures.iter().filter(|t| t.is_some()).count()
    }

    /// Depth of the clip stack, so a test can prove pushes and pops balance.
    pub fn clip_depth(&self) -> usize {
        self.clips.len()
    }

    /// The pixel rectangle drawing is currently confined to as `(x0, y0, x1, y1)` with the far
    /// edges exclusive, or `None` when nothing can draw.
    ///
    /// Logical and physical pixels are the same size here, so the clip only has to be rounded to
    /// whole pixels and held inside the canvas. Rounding matches what the GPU backend does before
    /// handing the rectangle to a scissor test, so both backends clip the same pixels.
    fn clip_bounds(&self) -> Option<(i64, i64, i64, i64)> {
        let rect = match self.clips.last() {
            Some(None) => return None,
            Some(Some(r)) => *r,
            None => Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
        };
        let x0 = (rect.x.round() as i64).max(0);
        let y0 = (rect.y.round() as i64).max(0);
        let x1 = ((rect.x + rect.w).round() as i64).min(self.width as i64);
        let y1 = ((rect.y + rect.h).round() as i64).min(self.height as i64);
        (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel_at(&self, x: u32, y: u32) -> Color {
        let i = ((y * self.width + x) * 4) as usize;
        Color { r: self.pixels[i], g: self.pixels[i + 1], b: self.pixels[i + 2], a: self.pixels[i + 3] }
    }

    /// Coarse content signature: the canvas is split into a `cols`×`rows` grid, each block's mean
    /// RGB is quantized (see [`SIGNATURE_QUANT_SHIFT`]) and emitted as three bytes in row-major
    /// order. Golden tests compare this instead of raw pixels so a one-pixel antialiasing shift
    /// does not fail the test while a moved panel or a recoloured row does.
    pub fn block_signature(&self, cols: u32, rows: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((cols * rows * 3) as usize);
        if cols == 0 || rows == 0 {
            return out;
        }
        for row in 0..rows {
            let y0 = (row as u64 * self.height as u64 / rows as u64) as u32;
            let y1 = ((row as u64 + 1) * self.height as u64 / rows as u64) as u32;
            for col in 0..cols {
                let x0 = (col as u64 * self.width as u64 / cols as u64) as u32;
                let x1 = ((col as u64 + 1) * self.width as u64 / cols as u64) as u32;
                let (mut sr, mut sg, mut sb, mut n) = (0u64, 0u64, 0u64, 0u64);
                for y in y0..y1 {
                    for x in x0..x1 {
                        let i = ((y * self.width + x) * 4) as usize;
                        sr += self.pixels[i] as u64;
                        sg += self.pixels[i + 1] as u64;
                        sb += self.pixels[i + 2] as u64;
                        n += 1;
                    }
                }
                let mean = |sum: u64| (sum.checked_div(n).unwrap_or(0) as u8) >> SIGNATURE_QUANT_SHIFT;
                out.extend_from_slice(&[mean(sr), mean(sg), mean(sb)]);
            }
        }
        out
    }

    /// FNV-1a 64-bit hash of [`CpuCanvas::block_signature`], so a golden expectation is one literal.
    pub fn signature_hash(&self, cols: u32, rows: u32) -> u64 {
        let mut h = FNV_OFFSET_BASIS;
        for b in self.block_signature(cols, rows) {
            h ^= b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        h
    }
}

/// One texel, with out-of-range coordinates clamped to the edge — the same wrap behaviour the GPU
/// sampler is configured with, so a bilinear tap at a texture border reads the same colour in both
/// backends.
fn texel(t: &Texture, x: i64, y: i64) -> Color {
    let x = x.clamp(0, t.width as i64 - 1) as usize;
    let y = y.clamp(0, t.height as i64 - 1) as usize;
    let i = (y * t.width as usize + x) * BYTES_PER_PIXEL;
    Color { r: t.rgba[i], g: t.rgba[i + 1], b: t.rgba[i + 2], a: t.rgba[i + 3] }
}

fn sample(t: &Texture, uv: (f32, f32), filter: TextureFilter) -> Color {
    let (tw, th) = (t.width as f32, t.height as f32);
    let (fx, fy) = (uv.0 * tw, uv.1 * th);
    match filter {
        TextureFilter::Nearest => texel(t, fx.floor() as i64, fy.floor() as i64),
        TextureFilter::Linear => {
            let (cx, cy) = (fx - 0.5, fy - 0.5);
            let (bx, by) = (cx.floor(), cy.floor());
            let (rx, ry) = (cx - bx, cy - by);
            let (bx, by) = (bx as i64, by as i64);
            let corners = [texel(t, bx, by), texel(t, bx + 1, by), texel(t, bx, by + 1), texel(t, bx + 1, by + 1)];
            let lerp = |a: u8, b: u8, r: f32| a as f32 + (b as f32 - a as f32) * r;
            let mix = |pick: fn(&Color) -> u8| {
                let top = lerp(pick(&corners[0]), pick(&corners[1]), rx);
                let bottom = lerp(pick(&corners[2]), pick(&corners[3]), rx);
                (top + (bottom - top) * ry).round().clamp(0.0, CHANNEL_MAX as f32) as u8
            };
            Color { r: mix(|c| c.r), g: mix(|c| c.g), b: mix(|c| c.b), a: mix(|c| c.a) }
        }
    }
}

/// The screen-to-destination mapping for one quad, resolved once instead of per pixel.
struct QuadMap {
    rotated: bool,
    sin: f32,
    cos: f32,
    anchor: (f32, f32),
    origin: (f32, f32),
    center: (f32, f32),
    size: (f32, f32),
}

impl QuadMap {
    fn new(p: &QuadParams) -> QuadMap {
        let (sin, cos) = p.angle_deg.to_radians().sin_cos();
        QuadMap {
            rotated: p.is_rotated(),
            sin,
            cos,
            anchor: (p.dst.x + p.center.0, p.dst.y + p.center.1),
            origin: (p.dst.x, p.dst.y),
            center: p.center,
            size: (p.dst.w, p.dst.h),
        }
    }

    /// Where a point inside the destination lands on screen. Positive angles turn clockwise, which
    /// is what a skin's counter-clockwise y-up `angle` becomes once the loader negates it.
    fn to_screen(&self, local: (f32, f32)) -> (f32, f32) {
        let (dx, dy) = (local.0 - self.center.0, local.1 - self.center.1);
        (self.anchor.0 + dx * self.cos - dy * self.sin, self.anchor.1 + dx * self.sin + dy * self.cos)
    }

    /// Where the centre of pixel `(x, y)` falls inside the destination, or `None` when it falls
    /// outside it. Sampling by pixel centre is what the GPU rasterizer does, so both backends
    /// cover the same pixels.
    fn local_at(&self, x: i64, y: i64) -> Option<(f32, f32)> {
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        let (lx, ly) = if self.rotated {
            let (dx, dy) = (px - self.anchor.0, py - self.anchor.1);
            (dx * self.cos + dy * self.sin + self.center.0, -dx * self.sin + dy * self.cos + self.center.1)
        } else {
            (px - self.origin.0, py - self.origin.1)
        };
        ((0.0..self.size.0).contains(&lx) && (0.0..self.size.1).contains(&ly)).then_some((lx, ly))
    }

    /// The pixel rectangle the quad can possibly touch, as `(x0, y0, x1, y1)` with the far edges
    /// exclusive.
    fn pixel_bounds(&self) -> (i64, i64, i64, i64) {
        if !self.rotated {
            let (x, y, w, h) = (self.origin.0, self.origin.1, self.size.0, self.size.1);
            return (x.floor() as i64, y.floor() as i64, (x + w).ceil() as i64, (y + h).ceil() as i64);
        }
        let (w, h) = self.size;
        let mut min = (f32::INFINITY, f32::INFINITY);
        let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for local in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
            let (sx, sy) = self.to_screen(local);
            min = (min.0.min(sx), min.1.min(sy));
            max = (max.0.max(sx), max.1.max(sy));
        }
        (min.0.floor() as i64, min.1.floor() as i64, max.0.ceil() as i64, max.1.ceil() as i64)
    }
}

/// Interpolate the source rectangle at a point given as a share of the destination.
fn uv_at(src: &UvRect, local: (f32, f32), size: (f32, f32)) -> (f32, f32) {
    let (rx, ry) = (local.0 / size.0, local.1 / size.1);
    (src.u0 + rx * (src.u1 - src.u0), src.v0 + ry * (src.v1 - src.v0))
}

impl Renderer for CpuCanvas {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn clear(&mut self, color: Color) {
        self.clips.clear();
        for px in self.pixels.chunks_exact_mut(4) {
            px[0] = color.r;
            px[1] = color.g;
            px[2] = color.b;
            px[3] = color.a;
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let Some((cx0, cy0, cx1, cy1)) = self.clip_bounds() else {
            return;
        };
        let x0 = (rect.x.floor() as i64).max(cx0);
        let y0 = (rect.y.floor() as i64).max(cy0);
        let x1 = ((rect.x + rect.w).ceil() as i64).min(cx1);
        let y1 = ((rect.y + rect.h).ceil() as i64).min(cy1);
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

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        let mut data = rgba.to_vec();
        data.resize(width as usize * height as usize * BYTES_PER_PIXEL, 0);
        let texture = Texture { width, height, rgba: data };
        if let Some(&id) = self.keys.get(key)
            && let Some(slot) = self.textures.get_mut(id.0 as usize)
        {
            *slot = Some(texture);
            return id;
        }
        let id = TextureId(self.textures.len() as u32);
        self.textures.push(Some(texture));
        self.keys.insert(key.to_string(), id);
        id
    }

    fn release_texture(&mut self, tex: TextureId) {
        if let Some(slot) = self.textures.get_mut(tex.0 as usize) {
            *slot = None;
        }
        self.keys.retain(|_, id| *id != tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.textures.get(tex.0 as usize).and_then(|t| t.as_ref()).map(|t| (t.width, t.height))
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        if !(params.dst.w > 0.0 && params.dst.h > 0.0) {
            return;
        }
        let Some((cx0, cy0, cx1, cy1)) = self.clip_bounds() else {
            return;
        };
        let Some(Some(texture)) = self.textures.get(tex.0 as usize) else {
            return;
        };
        if texture.width == 0 || texture.height == 0 {
            return;
        }
        let map = QuadMap::new(&params);
        let (bx0, by0, bx1, by1) = map.pixel_bounds();
        let (x0, y0) = (bx0.max(cx0), by0.max(cy0));
        let (x1, y1) = (bx1.min(cx1), by1.min(cy1));
        let stride = self.width as usize;
        let pixels = &mut self.pixels;
        for y in y0..y1 {
            for x in x0..x1 {
                let Some(local) = map.local_at(x, y) else {
                    continue;
                };
                let src = apply_tint(sample(texture, uv_at(&params.src, local, map.size), params.filter), params.tint);
                let i = (y as usize * stride + x as usize) * BYTES_PER_PIXEL;
                let dst = Color { r: pixels[i], g: pixels[i + 1], b: pixels[i + 2], a: pixels[i + 3] };
                let out = apply_blend(src, dst, params.blend);
                pixels[i] = out.r;
                pixels[i + 1] = out.g;
                pixels[i + 2] = out.b;
                pixels[i + 3] = out.a;
            }
        }
    }

    fn push_clip(&mut self, rect: Rect) {
        let next = match self.clips.last() {
            Some(None) => None,
            Some(Some(current)) => current.intersect(&rect),
            None => Some(rect),
        };
        self.clips.push(next);
    }

    fn pop_clip(&mut self) {
        debug_assert!(!self.clips.is_empty(), "pop_clip without a matching push_clip");
        self.clips.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_signature_has_three_quantized_bytes_per_block() {
        let mut c = CpuCanvas::new(8, 4);
        c.clear(Color::rgb(255, 16, 0));
        let sig = c.block_signature(4, 2);
        assert_eq!(sig.len(), 4 * 2 * 3, "one RGB triple per block");
        assert!(sig.chunks_exact(3).all(|b| b == [31, 2, 0]), "a flat canvas quantizes to one repeated triple (255>>3, 16>>3, 0>>3)");
        assert!(c.block_signature(0, 2).is_empty(), "a zero-sized grid yields no signature");
    }

    #[test]
    fn block_signature_averages_each_block_separately() {
        let mut c = CpuCanvas::new(4, 2);
        c.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color::rgb(255, 255, 255));
        c.fill_rect(Rect::new(2.0, 0.0, 2.0, 2.0), Color::rgb(0, 0, 0));
        let sig = c.block_signature(2, 1);
        assert_eq!(sig, vec![31, 31, 31, 0, 0, 0], "left block is full white, right block is full black");
    }

    #[test]
    fn signature_hash_is_stable_and_content_sensitive() {
        let mut a = CpuCanvas::new(16, 16);
        a.clear(Color::rgb(10, 20, 30));
        let h1 = a.signature_hash(4, 4);
        assert_eq!(h1, a.signature_hash(4, 4), "hashing the same canvas twice matches");
        let mut b = CpuCanvas::new(16, 16);
        b.clear(Color::rgb(10, 20, 30));
        assert_eq!(h1, b.signature_hash(4, 4), "identical content hashes identically");
        b.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(240, 0, 0));
        assert_ne!(h1, b.signature_hash(4, 4), "a repainted quadrant changes the hash");
    }

    /// The quantum is `1 << SIGNATURE_QUANT_SHIFT` == 8 raw levels. `100 >> 3 == 12` and
    /// `103 >> 3 == 12`, so a uniform +3 (the scale of antialiasing jitter) is absorbed, while
    /// `108 >> 3 == 13` is not.
    #[test]
    fn signature_absorbs_a_sub_quantum_brightness_shift_but_not_a_full_step() {
        let shade = |v: u8| {
            let mut c = CpuCanvas::new(64, 64);
            c.clear(Color::rgb(v, v, v));
            c.signature_hash(8, 8)
        };
        assert_eq!(shade(100), shade(103), "a uniform shift below one quantization step keeps the signature");
        assert_ne!(shade(100), shade(108), "a uniform shift of one full step changes the signature");
    }

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
        c.fill_rect(Rect::new(1.0, 1.0, 2.0, 2.0), Color { r: 50, g: 60, b: 70, a: 255 });
        assert_eq!(c.pixel_at(1, 1), Color { r: 50, g: 60, b: 70, a: 255 });
        assert_eq!(c.pixel_at(2, 2), Color { r: 50, g: 60, b: 70, a: 255 });
        assert_eq!(c.pixel_at(0, 0), Color { r: 0, g: 0, b: 0, a: 0 });
        assert_eq!(c.pixel_at(3, 3), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    fn fill_rect_alpha_zero_is_a_noop_on_rgb_but_sets_alpha_255() {
        let mut c = CpuCanvas::new(2, 2);
        c.clear(Color { r: 100, g: 110, b: 120, a: 100 });
        c.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Color { r: 5, g: 5, b: 5, a: 0 });
        assert_eq!(c.pixel_at(0, 0), Color { r: 100, g: 110, b: 120, a: 255 });
    }

    #[test]
    fn fill_rect_partial_alpha_matches_integer_source_over_math() {
        let mut c = CpuCanvas::new(1, 1);
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
        c.fill_rect(Rect::new(-5.0, -5.0, 7.0, 7.0), Color::rgb(9, 9, 9));
        assert_eq!(c.pixel_at(0, 0), Color { r: 9, g: 9, b: 9, a: 255 });
        assert_eq!(c.pixel_at(1, 1), Color { r: 9, g: 9, b: 9, a: 255 });
        assert_eq!(c.pixel_at(2, 2), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    fn fill_rect_clips_oversized_rect_to_canvas_bounds_without_panic() {
        let mut c = CpuCanvas::new(2, 2);
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
        c.fill_rect(Rect::new(1.0, 1.0, 0.0, 0.0), Color::rgb(99, 99, 99));
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
        c.fill_rect(Rect::new(1.2, 1.2, 0.1, 0.1), Color::rgb(77, 77, 77));
        assert_eq!(c.pixel_at(1, 1), Color { r: 77, g: 77, b: 77, a: 255 }, "sub-pixel rect rounds up to one cell");
        assert_eq!(c.pixel_at(2, 2), Color { r: 0, g: 0, b: 0, a: 0 });
    }

    #[test]
    #[should_panic]
    fn pixel_at_out_of_bounds_panics() {
        let c = CpuCanvas::new(2, 2);
        let _ = c.pixel_at(2, 2);
    }
}
