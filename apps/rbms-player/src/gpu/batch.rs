//! Draw-call bookkeeping for the GPU backend: what the frame asked for, in the order it asked.
//!
//! Skins draw back to front, so submission order *is* z-order. Nothing here ever reorders a draw.
//! Merging is only ever applied to a run of adjacent draws that agree on everything a draw call
//! carries — texture, blend, filter, whether they rotate, and the clip in force. The moment one of
//! those differs the run ends and a new batch starts, which is what keeps a translucent quad from
//! being lifted over something drawn after it.

use std::borrow::Cow;

use rbms_render::{BlendMode, Color, QuadParams, Rect, TextureFilter, TextureId};

/// Bytes a texture upload's rows are padded to. `wgpu` requires this of a buffer-to-texture copy,
/// and a padded upload is correct either way.
pub(crate) const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

/// Bytes per pixel in the RGBA8 textures the registry holds.
pub(crate) const BYTES_PER_PIXEL: u32 = 4;

/// One flat-coloured quad, as the existing instanced pipeline consumes it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ColoredInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) color: [f32; 4],
}

impl ColoredInstance {
    pub(crate) fn new(rect: Rect, color: Color) -> ColoredInstance {
        let channel = |c: u8| c as f32 / u8::MAX as f32;
        ColoredInstance { rect: [rect.x, rect.y, rect.w, rect.h], color: [channel(color.r), channel(color.g), channel(color.b), channel(color.a)] }
    }
}

/// One textured quad: where it lands, what it reads, what it is multiplied by, and the rotation
/// packed as `(cos, sin, centre x, centre y)`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct TexturedInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) tint: [f32; 4],
    pub(crate) rotation: [f32; 4],
}

impl TexturedInstance {
    pub(crate) fn new(params: &QuadParams) -> TexturedInstance {
        let channel = |c: u8| c as f32 / u8::MAX as f32;
        let (sin, cos) = params.angle_deg.to_radians().sin_cos();
        TexturedInstance {
            rect: [params.dst.x, params.dst.y, params.dst.w, params.dst.h],
            uv: [params.src.u0, params.src.v0, params.src.u1, params.src.v1],
            tint: [channel(params.tint.r), channel(params.tint.g), channel(params.tint.b), channel(params.tint.a)],
            rotation: [cos, sin, params.center.0, params.center.1],
        }
    }
}

/// What kind of draw call a batch turns into, and everything about it that a GPU state change
/// would be needed for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BatchKind {
    Colored,
    Textured { tex: TextureId, blend: BlendMode, filter: TextureFilter, rotated: bool },
}

/// The clip in force for a batch. Compared as part of the batch key, so a clip change always ends
/// the run.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ClipState {
    /// The clip rectangle in logical screen pixels, or `None` when drawing is unclipped.
    pub(crate) rect: Option<[f32; 4]>,
    /// Set when nested clips stopped overlapping, in which case nothing may draw at all.
    pub(crate) empty: bool,
}

impl ClipState {
    fn intersect(self, rect: Rect) -> ClipState {
        if self.empty {
            return self;
        }
        match self.rect {
            None => ClipState { rect: Some([rect.x, rect.y, rect.w, rect.h]), empty: false },
            Some([x, y, w, h]) => match Rect::new(x, y, w, h).intersect(&rect) {
                Some(r) => ClipState { rect: Some([r.x, r.y, r.w, r.h]), empty: false },
                None => ClipState { rect: None, empty: true },
            },
        }
    }
}

/// One draw call: a contiguous run of instances sharing a kind and a clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Batch {
    pub(crate) kind: BatchKind,
    pub(crate) clip: ClipState,
    pub(crate) first: u32,
    pub(crate) count: u32,
}

/// Everything one frame asked to draw, in submission order.
#[derive(Default)]
pub(crate) struct DrawList {
    pub(crate) colored: Vec<ColoredInstance>,
    pub(crate) textured: Vec<TexturedInstance>,
    pub(crate) batches: Vec<Batch>,
    clips: Vec<ClipState>,
}

impl DrawList {
    /// Drop the frame, clip stack included: a new frame starts unclipped however the last one
    /// ended.
    pub(crate) fn clear(&mut self) {
        self.colored.clear();
        self.textured.clear();
        self.batches.clear();
        self.clips.clear();
    }

    pub(crate) fn current_clip(&self) -> ClipState {
        self.clips.last().copied().unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn clip_depth(&self) -> usize {
        self.clips.len()
    }

    pub(crate) fn push_clip(&mut self, rect: Rect) {
        let next = self.current_clip().intersect(rect);
        self.clips.push(next);
    }

    pub(crate) fn pop_clip(&mut self) {
        debug_assert!(!self.clips.is_empty(), "pop_clip without a matching push_clip");
        self.clips.pop();
    }

    /// How many quads the frame has queued, which is what the debug overlay reports.
    pub(crate) fn quad_count(&self) -> usize {
        self.colored.len() + self.textured.len()
    }

    pub(crate) fn push_colored(&mut self, instance: ColoredInstance) {
        let first = self.colored.len() as u32;
        self.colored.push(instance);
        self.extend(BatchKind::Colored, first);
    }

    pub(crate) fn push_textured(&mut self, kind: BatchKind, instance: TexturedInstance) {
        let first = self.textured.len() as u32;
        self.textured.push(instance);
        self.extend(kind, first);
    }

    /// Add the instance at `first` to the open batch when it can join it, and open a new one when
    /// it cannot. Never touches any batch but the last, so order is preserved by construction.
    fn extend(&mut self, kind: BatchKind, first: u32) {
        let clip = self.current_clip();
        if let Some(last) = self.batches.last_mut()
            && last.kind == kind
            && last.clip == clip
        {
            last.count += 1;
            return;
        }
        self.batches.push(Batch { kind, clip, first, count: 1 });
    }
}

/// The scissor rectangle a batch needs, in surface pixels, or `None` when the batch cannot put
/// anything on screen.
///
/// The logical screen is drawn into `viewport`, which is the whole surface when the window is
/// stretched and a centred 16:9 rectangle when it is fitted. A scissor is given in surface pixels
/// rather than viewport-relative ones, so the clip is scaled by the viewport and then offset by
/// it. Anything outside the surface is trimmed away, because a scissor that leaves its attachment
/// is rejected outright.
pub(crate) fn scissor_rect(clip: ClipState, viewport: (f32, f32, f32, f32), logical: (u32, u32), surface: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
    if clip.empty {
        return None;
    }
    let full = (0, 0, surface.0, surface.1);
    let Some([x, y, w, h]) = clip.rect else {
        return (surface.0 > 0 && surface.1 > 0).then_some(full);
    };
    let (vx, vy, vw, vh) = viewport;
    let (sx, sy) = (vw / logical.0.max(1) as f32, vh / logical.1.max(1) as f32);
    let x0 = (vx + x * sx).round().max(0.0) as u32;
    let y0 = (vy + y * sy).round().max(0.0) as u32;
    let x1 = ((vx + (x + w) * sx).round().max(0.0) as u32).min(surface.0);
    let y1 = ((vy + (y + h) * sy).round().max(0.0) as u32).min(surface.1);
    (x1 > x0 && y1 > y0).then(|| (x0, y0, x1 - x0, y1 - y0))
}

/// An RGBA8 image with each row padded out to [`COPY_BYTES_PER_ROW_ALIGNMENT`], and the padded row
/// stride, ready to be handed to a texture upload.
///
/// A row already on the alignment is borrowed rather than copied, so the common power-of-two
/// texture costs nothing at all -- which matters because a background frame comes back through here
/// on the frames it has not changed on as well. Short input is zero filled rather than rejected, so
/// a truncated frame draws dark instead of killing the process.
pub(crate) fn pad_rows_to_alignment(rgba: &[u8], width: u32, height: u32) -> (Cow<'_, [u8]>, u32) {
    let tight = width * BYTES_PER_PIXEL;
    let padded = tight.div_ceil(COPY_BYTES_PER_ROW_ALIGNMENT) * COPY_BYTES_PER_ROW_ALIGNMENT;
    if padded == tight {
        let needed = (tight * height) as usize;
        if rgba.len() == needed {
            return (Cow::Borrowed(rgba), tight);
        }
        let mut exact = rgba.to_vec();
        exact.resize(needed, 0);
        return (Cow::Owned(exact), tight);
    }
    let mut out = vec![0u8; (padded * height) as usize];
    for row in 0..height as usize {
        let from = row * tight as usize;
        let available = rgba.len().saturating_sub(from).min(tight as usize);
        if available == 0 {
            break;
        }
        let to = row * padded as usize;
        out[to..to + available].copy_from_slice(&rgba[from..from + available]);
    }
    (Cow::Owned(out), padded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_render::UvRect;

    const LOGICAL: (u32, u32) = (1280, 720);

    fn colored(x: f32) -> ColoredInstance {
        ColoredInstance::new(Rect::new(x, 0.0, 1.0, 1.0), Color::WHITE)
    }

    fn textured(tex: u32) -> (BatchKind, TexturedInstance) {
        let kind = BatchKind::Textured { tex: TextureId(tex), blend: BlendMode::Alpha, filter: TextureFilter::Nearest, rotated: false };
        (kind, TexturedInstance::new(&QuadParams::new(Rect::new(0.0, 0.0, 1.0, 1.0))))
    }

    #[test]
    fn adjacent_draws_that_agree_on_everything_become_one_call() {
        let mut list = DrawList::default();
        for i in 0..3 {
            list.push_colored(colored(i as f32));
        }
        assert_eq!(list.batches.len(), 1);
        assert_eq!(list.batches[0].count, 3);
        assert_eq!(list.batches[0].first, 0);
        assert_eq!(list.quad_count(), 3);
    }

    /// The rule the whole module exists for: interleaved kinds keep their order rather than being
    /// gathered per texture, because a skin's draw order is its z-order.
    #[test]
    fn interleaved_draws_keep_their_submission_order_instead_of_being_gathered() {
        let mut list = DrawList::default();
        let (a, inst_a) = textured(1);
        let (b, inst_b) = textured(2);
        list.push_textured(a, inst_a);
        list.push_colored(colored(0.0));
        list.push_textured(b, inst_b);
        list.push_colored(colored(1.0));
        list.push_textured(a, inst_a);

        let kinds: Vec<BatchKind> = list.batches.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, vec![a, BatchKind::Colored, b, BatchKind::Colored, a], "the five draws stayed in order");
        assert!(list.batches.iter().all(|b| b.count == 1), "nothing merged across a differing neighbour");
        assert_eq!(list.textured.len(), 3);
        assert_eq!(list.colored.len(), 2);
    }

    #[test]
    fn a_run_of_one_texture_merges_but_a_different_texture_starts_a_new_call() {
        let mut list = DrawList::default();
        let (a, inst_a) = textured(1);
        let (b, inst_b) = textured(2);
        list.push_textured(a, inst_a);
        list.push_textured(a, inst_a);
        list.push_textured(b, inst_b);
        assert_eq!(list.batches.len(), 2);
        assert_eq!((list.batches[0].first, list.batches[0].count), (0, 2));
        assert_eq!((list.batches[1].first, list.batches[1].count), (2, 1));
    }

    #[test]
    fn a_changed_blend_filter_or_rotation_ends_the_run() {
        let base = QuadParams::new(Rect::new(0.0, 0.0, 1.0, 1.0));
        let plain = BatchKind::Textured { tex: TextureId(1), blend: BlendMode::Alpha, filter: TextureFilter::Nearest, rotated: false };
        for changed in [
            BatchKind::Textured { tex: TextureId(1), blend: BlendMode::Add, filter: TextureFilter::Nearest, rotated: false },
            BatchKind::Textured { tex: TextureId(1), blend: BlendMode::Alpha, filter: TextureFilter::Linear, rotated: false },
            BatchKind::Textured { tex: TextureId(1), blend: BlendMode::Alpha, filter: TextureFilter::Nearest, rotated: true },
        ] {
            let mut list = DrawList::default();
            list.push_textured(plain, TexturedInstance::new(&base));
            list.push_textured(changed, TexturedInstance::new(&base));
            assert_eq!(list.batches.len(), 2, "{changed:?} should not have joined {plain:?}");
        }
    }

    #[test]
    fn a_clip_change_ends_the_run_and_leaving_the_clip_lets_the_next_run_rejoin() {
        let mut list = DrawList::default();
        list.push_colored(colored(0.0));
        list.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        list.push_colored(colored(1.0));
        list.pop_clip();
        list.push_colored(colored(2.0));
        assert_eq!(list.batches.len(), 3, "the clipped draw is its own call");
        assert_eq!(list.batches[1].clip.rect, Some([0.0, 0.0, 10.0, 10.0]));
        assert_eq!(list.batches[2].clip.rect, None, "and drawing after the pop is unclipped again");
    }

    #[test]
    fn a_clip_pushed_and_popped_without_drawing_changes_nothing() {
        let mut list = DrawList::default();
        list.push_colored(colored(0.0));
        list.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        list.pop_clip();
        list.push_colored(colored(1.0));
        assert_eq!(list.batches.len(), 1, "nothing was drawn under the clip, so nothing was split");
        assert_eq!(list.batches[0].count, 2);
    }

    #[test]
    fn nested_clips_intersect() {
        let mut list = DrawList::default();
        list.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
        list.push_clip(Rect::new(50.0, 20.0, 100.0, 10.0));
        assert_eq!(list.current_clip().rect, Some([50.0, 20.0, 50.0, 10.0]));
        list.pop_clip();
        assert_eq!(list.current_clip().rect, Some([0.0, 0.0, 100.0, 100.0]));
    }

    #[test]
    fn clips_that_stop_overlapping_leave_nothing_drawable() {
        let mut list = DrawList::default();
        list.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        list.push_clip(Rect::new(50.0, 50.0, 10.0, 10.0));
        assert!(list.current_clip().empty);
        list.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(list.current_clip().empty, "an empty clip stays empty however it is narrowed");
        list.push_colored(colored(0.0));
        assert_eq!(scissor_rect(list.batches[0].clip, (0.0, 0.0, 1280.0, 720.0), LOGICAL, (1280, 720)), None);
    }

    #[test]
    fn popping_more_than_was_pushed_is_survivable() {
        let mut list = DrawList::default();
        list.push_clip(Rect::new(0.0, 0.0, 4.0, 4.0));
        list.pop_clip();
        assert_eq!(list.clip_depth(), 0);
        assert_eq!(list.current_clip(), ClipState::default());
    }

    #[test]
    fn clearing_a_frame_drops_the_clip_stack_with_it() {
        let mut list = DrawList::default();
        list.push_clip(Rect::new(0.0, 0.0, 4.0, 4.0));
        list.push_colored(colored(0.0));
        list.clear();
        assert_eq!(list.clip_depth(), 0);
        assert_eq!(list.quad_count(), 0);
        assert!(list.batches.is_empty());
        assert_eq!(list.current_clip(), ClipState::default());
    }

    #[test]
    fn an_unclipped_batch_scissors_the_whole_surface() {
        let scissor = scissor_rect(ClipState::default(), (0.0, 0.0, 1280.0, 720.0), LOGICAL, (1280, 720));
        assert_eq!(scissor, Some((0, 0, 1280, 720)));
    }

    #[test]
    fn a_clip_is_scaled_from_logical_pixels_to_the_surface() {
        let clip = ClipState { rect: Some([320.0, 180.0, 640.0, 360.0]), empty: false };
        let scissor = scissor_rect(clip, (0.0, 0.0, 2560.0, 1440.0), LOGICAL, (2560, 1440));
        assert_eq!(scissor, Some((640, 360, 1280, 720)), "a doubled surface doubles the clip");
    }

    /// A fitted window draws the screen into a centred rectangle, and a scissor is in surface
    /// pixels, so the clip has to be offset by where the screen actually landed.
    #[test]
    fn a_clip_follows_the_screen_into_a_letterboxed_viewport() {
        let clip = ClipState { rect: Some([0.0, 0.0, 640.0, 720.0]), empty: false };
        let scissor = scissor_rect(clip, (320.0, 0.0, 1920.0, 1080.0), LOGICAL, (2560, 1080));
        assert_eq!(scissor, Some((320, 0, 960, 1080)), "the left half of the screen inside the bars");
    }

    #[test]
    fn a_clip_reaching_past_the_surface_is_trimmed_rather_than_rejected() {
        let clip = ClipState { rect: Some([-100.0, -100.0, 5000.0, 5000.0]), empty: false };
        assert_eq!(scissor_rect(clip, (0.0, 0.0, 1280.0, 720.0), LOGICAL, (1280, 720)), Some((0, 0, 1280, 720)));
    }

    #[test]
    fn a_clip_that_rounds_away_to_nothing_skips_the_draw() {
        let clip = ClipState { rect: Some([10.0, 10.0, 0.2, 20.0]), empty: false };
        assert_eq!(scissor_rect(clip, (0.0, 0.0, 1280.0, 720.0), LOGICAL, (1280, 720)), None, "a sub-pixel clip has no pixels to scissor");
        let offscreen = ClipState { rect: Some([5000.0, 0.0, 10.0, 10.0]), empty: false };
        assert_eq!(scissor_rect(offscreen, (0.0, 0.0, 1280.0, 720.0), LOGICAL, (1280, 720)), None);
    }

    #[test]
    fn a_window_with_no_size_yet_has_nothing_to_scissor() {
        assert_eq!(scissor_rect(ClipState::default(), (0.0, 0.0, 0.0, 0.0), LOGICAL, (0, 0)), None);
    }

    /// The alignment trap: 300 pixels is 1200 bytes a row, which is not a multiple of 256.
    #[test]
    fn an_unaligned_row_is_padded_out_and_its_pixels_stay_put() {
        let (width, height) = (300u32, 3u32);
        let tight = width * BYTES_PER_PIXEL;
        let source: Vec<u8> = (0..tight * height).map(|i| (i % 251) as u8).collect();
        let (padded, stride) = pad_rows_to_alignment(&source, width, height);

        assert_eq!(tight, 1200);
        assert_eq!(stride, 1280, "1200 bytes rounds up to the next multiple of 256");
        assert_eq!(stride % COPY_BYTES_PER_ROW_ALIGNMENT, 0);
        assert_eq!(padded.len(), (stride * height) as usize);
        for row in 0..height as usize {
            let from = row * tight as usize;
            let to = row * stride as usize;
            assert_eq!(&padded[to..to + tight as usize], &source[from..from + tight as usize], "row {row} moved");
            assert!(padded[to + tight as usize..to + stride as usize].iter().all(|b| *b == 0), "row {row} padding is not zeroed");
        }
    }

    #[test]
    fn an_already_aligned_row_is_left_exactly_as_it_was() {
        let (width, height) = (64u32, 2u32);
        let source: Vec<u8> = (0..width * BYTES_PER_PIXEL * height).map(|i| i as u8).collect();
        let (out, stride) = pad_rows_to_alignment(&source, width, height);
        assert_eq!(stride, width * BYTES_PER_PIXEL);
        assert_eq!(out, source);
    }

    #[test]
    fn a_short_buffer_is_zero_filled_rather_than_read_past() {
        let (out, stride) = pad_rows_to_alignment(&[1, 2, 3, 4], 300, 2);
        assert_eq!(out.len(), (stride * 2) as usize);
        assert_eq!(&out[0..4], &[1, 2, 3, 4]);
        assert!(out[4..].iter().all(|b| *b == 0));

        let (exact, stride) = pad_rows_to_alignment(&[9; 4], 64, 2);
        assert_eq!(exact.len(), (stride * 2) as usize);
        assert!(exact[4..].iter().all(|b| *b == 0));
    }

    /// A background image comes back through here on every frame, changed or not, and the common
    /// power-of-two texture already has aligned rows: copying it into a fresh allocation each time
    /// is megabytes a frame for nothing. An aligned, exactly sized buffer is handed straight on.
    #[test]
    fn an_aligned_buffer_of_the_right_length_is_not_copied() {
        let pixels = vec![7u8; (64 * BYTES_PER_PIXEL * 32) as usize];
        let (out, stride) = pad_rows_to_alignment(&pixels, 64, 32);
        assert_eq!(stride, 64 * BYTES_PER_PIXEL, "sixty-four pixels a row is already on the alignment");
        assert!(matches!(out, Cow::Borrowed(_)), "an aligned buffer of the right length is handed on rather than copied");
        assert_eq!(out.as_ptr(), pixels.as_ptr(), "and it is the caller's own bytes");

        let (unaligned, _) = pad_rows_to_alignment(&pixels, 63, 32);
        assert!(matches!(unaligned, Cow::Owned(_)), "a row off the alignment still has to be padded");
    }

    #[test]
    fn an_instance_carries_the_quads_geometry_untouched() {
        let mut params = QuadParams::new(Rect::new(10.0, 20.0, 30.0, 40.0));
        params.src = UvRect::new(0.25, 0.5, 0.75, 1.0);
        params.tint = Color { r: 255, g: 0, b: 0, a: 128 };
        let instance = TexturedInstance::new(&params);
        assert_eq!(instance.rect, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(instance.uv, [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(instance.tint[0], 1.0);
        assert_eq!(instance.tint[1], 0.0);
        assert!((instance.tint[3] - 128.0 / 255.0).abs() < f32::EPSILON);
        assert_eq!(instance.rotation, [1.0, 0.0, 0.0, 0.0], "an unrotated quad carries the identity");
    }

    /// The sign that is easy to get backwards: a positive angle turns clockwise on screen, which
    /// with y increasing downwards means a point to the right of the centre moves down.
    #[test]
    fn a_quarter_turn_is_packed_as_a_clockwise_rotation() {
        let mut params = QuadParams::new(Rect::new(0.0, 0.0, 10.0, 10.0));
        params.angle_deg = 90.0;
        let [cos, sin, ..] = TexturedInstance::new(&params).rotation;
        assert!(cos.abs() < 1e-6, "cos 90 is zero, got {cos}");
        assert!((sin - 1.0).abs() < 1e-6, "sin 90 is one, got {sin}");
        let (dx, dy) = (1.0f32, 0.0f32);
        let turned = (dx * cos - dy * sin, dx * sin + dy * cos);
        assert!(turned.1 > 0.5, "a point to the right turned downwards, got {turned:?}");
    }
}
