//! Shelf-packed glyph atlas: rasterized glyphs gathered into one texture so a line of text can be
//! drawn as textured quads instead of one small fill per run of lit pixels.
//!
//! The atlas is a single page, at most [`ATLAS_MAX_DIM`] square. It starts short and doubles in
//! height as glyphs arrive, which keeps the first upload small; the width never changes, so a
//! glyph already packed keeps its pixel position across a growth and only its normalised
//! coordinates move. When a page cannot grow any further and the next glyph still does not fit,
//! the page is emptied and packing restarts — glyphs are cheap to rasterize again, and a text
//! engine that had to refuse to draw would be worse.
//!
//! Every mutation bumps [`GlyphAtlas::revision`], which is how a [`GlyphAtlasBinding`] knows the
//! page a backend holds is stale. A growth or an emptying bumps [`GlyphAtlas::layout_revision`] as
//! well, because those are the two changes that move glyphs already packed: after one of them, the
//! normalised coordinates handed out earlier name different pixels. A backend that draws
//! immediately never notices, but one that records a frame and submits it at the end would sample
//! the new page with the old coordinates, so the binding gives every layout its own registration
//! key and keeps the previous page alive until the frame that used it is over.

use std::collections::HashMap;
use std::hash::Hash;

use cosmic_text::{CacheKey, Color as CtColor, FontSystem, SwashCache};

use crate::{BYTES_PER_PIXEL, Color, Renderer, TextureId, UvRect};

/// Largest page edge, in pixels. Well inside the 2D texture limit of every backend this targets.
pub const ATLAS_MAX_DIM: u32 = 2048;

/// Height the page starts at, so the first upload moves a fraction of a full page.
pub const ATLAS_INITIAL_HEIGHT: u32 = 256;

/// Transparent pixels kept between packed glyphs, so a filtered sample near a glyph edge cannot
/// pick up its neighbour.
const GLYPH_PADDING: u32 = 1;

/// Registration key prefix the page is uploaded under. One key per layout: packing a glyph into
/// free space re-uploads under the same key, and only a growth or an emptying takes a new one.
pub const ATLAS_TEXTURE_KEY: &str = "rbms.render.glyph-atlas";

/// Where one glyph sits in the page, and where it sits relative to the pen it was rasterized for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasEntry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Horizontal offset from the pen position to the glyph's left edge.
    pub left: i32,
    /// Vertical offset from the pen position to the glyph's top edge.
    pub top: i32,
}

impl AtlasEntry {
    /// The source rectangle to sample this glyph from a `page_w` x `page_h` page.
    pub fn uv(&self, page_w: u32, page_h: u32) -> UvRect {
        UvRect::from_pixels(self.x, self.y, self.width, self.height, page_w, page_h)
    }

    /// A glyph with no lit pixels, such as a space, which is packed but never drawn.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// One row of the shelf packer.
struct Shelf {
    y: u32,
    height: u32,
    used_width: u32,
}

/// A page of packed glyphs, keyed by whatever identifies a rasterization to the caller.
pub struct GlyphAtlas<K> {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    shelves: Vec<Shelf>,
    entries: HashMap<K, AtlasEntry>,
    revision: u64,
    layout_revision: u64,
}

impl<K: Eq + Hash> Default for GlyphAtlas<K> {
    fn default() -> Self {
        GlyphAtlas::new()
    }
}

impl<K: Eq + Hash> GlyphAtlas<K> {
    pub fn new() -> GlyphAtlas<K> {
        let (width, height) = (ATLAS_MAX_DIM, ATLAS_INITIAL_HEIGHT);
        GlyphAtlas {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * BYTES_PER_PIXEL],
            shelves: Vec::new(),
            entries: HashMap::new(),
            revision: 0,
            layout_revision: 0,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The page's pixels, RGBA8 and not premultiplied.
    pub fn page(&self) -> &[u8] {
        &self.rgba
    }

    /// Bumped by every change to the page, so a holder of an uploaded copy can tell it is stale.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Bumped only when glyphs already packed change where they are read from: the page growing
    /// taller, or the page being emptied and repacked.
    ///
    /// Coordinates handed out under one layout revision stay correct for as long as it stands, so
    /// this is what decides whether a backend needs a second texture rather than a fresh upload.
    pub fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<AtlasEntry> {
        self.entries.get(key).copied()
    }

    /// Forget every glyph and start the page over at its initial height.
    pub fn clear(&mut self) {
        self.height = ATLAS_INITIAL_HEIGHT;
        self.rgba.clear();
        self.rgba.resize(self.width as usize * self.height as usize * BYTES_PER_PIXEL, 0);
        self.shelves.clear();
        self.entries.clear();
        self.revision += 1;
        self.layout_revision += 1;
    }

    /// Pack `rgba` (a `width` x `height` glyph bitmap) under `key`, returning where it landed.
    ///
    /// A key already present is returned untouched, so callers can insert unconditionally. `None`
    /// comes back only when the glyph is wider or taller than a full page.
    pub fn insert(&mut self, key: K, width: u32, height: u32, left: i32, top: i32, rgba: &[u8]) -> Option<AtlasEntry> {
        if let Some(entry) = self.entries.get(&key) {
            return Some(*entry);
        }
        if width == 0 || height == 0 {
            let entry = AtlasEntry { x: 0, y: 0, width: 0, height: 0, left, top };
            self.entries.insert(key, entry);
            return Some(entry);
        }
        if width > self.width || height > ATLAS_MAX_DIM {
            return None;
        }
        let (x, y) = match self.reserve(width, height) {
            Some(spot) => spot,
            None => {
                self.clear();
                self.reserve(width, height)?
            }
        };
        for row in 0..height {
            let from = (row as usize * width as usize) * BYTES_PER_PIXEL;
            let to = ((y + row) as usize * self.width as usize + x as usize) * BYTES_PER_PIXEL;
            let span = width as usize * BYTES_PER_PIXEL;
            self.rgba[to..to + span].copy_from_slice(&rgba[from..from + span]);
        }
        let entry = AtlasEntry { x, y, width, height, left, top };
        self.entries.insert(key, entry);
        self.revision += 1;
        Some(entry)
    }

    /// Find room for a `width` x `height` glyph, growing the page if that is what it takes.
    fn reserve(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        let need = width + GLYPH_PADDING;
        for shelf in self.shelves.iter_mut() {
            if shelf.height >= height && shelf.used_width + need <= self.width {
                let x = shelf.used_width;
                shelf.used_width += need;
                return Some((x, shelf.y));
            }
        }
        let top = self.shelves.last().map(|s| s.y + s.height + GLYPH_PADDING).unwrap_or(0);
        while top + height > self.height {
            if self.height >= ATLAS_MAX_DIM {
                return None;
            }
            self.height = (self.height * 2).min(ATLAS_MAX_DIM);
            self.rgba.resize(self.width as usize * self.height as usize * BYTES_PER_PIXEL, 0);
            self.revision += 1;
            self.layout_revision += 1;
        }
        self.shelves.push(Shelf { y: top, height, used_width: need });
        Some((0, top))
    }
}

/// One backend's copy of an atlas page: the handle it was registered under and the revisions that
/// copy holds. Keep one per renderer — a handle from one backend means nothing to another.
///
/// A page that only gained glyphs is re-uploaded in place, because nothing already drawn moved. A
/// page that grew or was emptied gets a handle of its own instead, and the handle it replaces is
/// kept until [`GlyphAtlasBinding::end_frame`], so quads a deferred backend has already recorded
/// still read the pixels they were measured against.
#[derive(Debug, Default)]
pub struct GlyphAtlasBinding {
    tex: Option<TextureId>,
    revision: Option<u64>,
    layout_revision: Option<u64>,
    retired: Vec<TextureId>,
}

impl GlyphAtlasBinding {
    pub fn new() -> GlyphAtlasBinding {
        GlyphAtlasBinding { tex: None, revision: None, layout_revision: None, retired: Vec::new() }
    }

    /// The handle this binding last registered, if any.
    pub fn texture(&self) -> Option<TextureId> {
        self.tex
    }

    /// How many pages this binding is still holding for a frame that has not ended.
    pub fn retired_count(&self) -> usize {
        self.retired.len()
    }

    /// Make sure `renderer` holds the atlas as it stands, uploading only when the page has moved
    /// on since the last call.
    pub fn sync<K: Eq + Hash, R: Renderer>(&mut self, renderer: &mut R, atlas: &GlyphAtlas<K>) -> TextureId {
        let (width, height) = atlas.size();
        let layout = atlas.layout_revision();
        if self.layout_revision == Some(layout) {
            if self.revision == Some(atlas.revision())
                && let Some(tex) = self.tex
            {
                return tex;
            }
            if let Some(tex) = self.tex {
                renderer.register_texture(&page_key(layout), atlas.page(), width, height);
                self.revision = Some(atlas.revision());
                return tex;
            }
        }
        if let Some(previous) = self.tex.take() {
            self.retired.push(previous);
        }
        let tex = renderer.register_texture(&page_key(layout), atlas.page(), width, height);
        self.tex = Some(tex);
        self.revision = Some(atlas.revision());
        self.layout_revision = Some(layout);
        tex
    }

    /// Hands back the pages this frame stopped drawing from.
    ///
    /// Call it once the frame has been submitted. Until then the older pages stay uploaded, which
    /// is the whole point: a backend that batches a frame reads them when it finally draws.
    pub fn end_frame<R: Renderer>(&mut self, renderer: &mut R) {
        for tex in self.retired.drain(..) {
            renderer.release_texture(tex);
        }
    }
}

/// The registration key one layout of the page is uploaded under.
fn page_key(layout_revision: u64) -> String {
    format!("{ATLAS_TEXTURE_KEY}.{layout_revision}")
}

/// One glyph rasterized into a tight bitmap, with the offset from the pen to its top left corner.
pub struct GlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub rgba: Vec<u8>,
}

/// The colour a glyph rasterization is cached under, so the two text paths key the same glyph the
/// same way.
pub fn packed_rgb(color: Color) -> u32 {
    ((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32
}

pub fn glyph_bitmap(fs: &mut FontSystem, swash: &mut SwashCache, ck: CacheKey, base: CtColor) -> GlyphBitmap {
    let mut pixels: Vec<(i32, i32, u8, u8, u8, u8)> = Vec::new();
    swash.with_pixels(fs, ck, base, |dx, dy, col| {
        if col.a() != 0 {
            pixels.push((dx, dy, col.r(), col.g(), col.b(), col.a()));
        }
    });
    let Some(&(first_x, first_y, ..)) = pixels.first() else {
        return GlyphBitmap { width: 0, height: 0, left: 0, top: 0, rgba: Vec::new() };
    };
    let (mut left, mut top, mut right, mut bottom) = (first_x, first_y, first_x + 1, first_y + 1);
    for &(dx, dy, ..) in &pixels {
        left = left.min(dx);
        top = top.min(dy);
        right = right.max(dx + 1);
        bottom = bottom.max(dy + 1);
    }
    let (width, height) = ((right - left) as u32, (bottom - top) as u32);
    let mut rgba = vec![0u8; width as usize * height as usize * BYTES_PER_PIXEL];
    for (dx, dy, r, g, b, a) in pixels {
        let i = ((dy - top) as usize * width as usize + (dx - left) as usize) * BYTES_PER_PIXEL;
        rgba[i..i + BYTES_PER_PIXEL].copy_from_slice(&[r, g, b, a]);
    }
    GlyphBitmap { width, height, left, top, rgba }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CpuCanvas;

    fn glyph(width: u32, height: u32, value: u8) -> Vec<u8> {
        [value, value, value, value].repeat((width * height) as usize)
    }

    #[test]
    fn a_new_atlas_is_empty_and_starts_short() {
        let atlas: GlyphAtlas<u32> = GlyphAtlas::new();
        assert_eq!(atlas.size(), (ATLAS_MAX_DIM, ATLAS_INITIAL_HEIGHT));
        assert!(atlas.is_empty());
        assert_eq!(atlas.revision(), 0);
        assert_eq!(atlas.page().len(), (ATLAS_MAX_DIM * ATLAS_INITIAL_HEIGHT) as usize * BYTES_PER_PIXEL);
    }

    #[test]
    fn an_inserted_glyph_is_found_again_and_carries_its_pen_offsets() {
        let mut atlas = GlyphAtlas::new();
        let entry = atlas.insert(1u32, 4, 6, -2, -8, &glyph(4, 6, 200)).expect("fits");
        assert_eq!((entry.width, entry.height), (4, 6));
        assert_eq!((entry.left, entry.top), (-2, -8));
        assert_eq!(atlas.get(&1), Some(entry));
        assert_eq!(atlas.len(), 1);
        assert!(atlas.revision() > 0, "packing a glyph moves the page on");
    }

    #[test]
    fn inserting_the_same_key_twice_keeps_the_first_placement() {
        let mut atlas = GlyphAtlas::new();
        let first = atlas.insert(7u32, 3, 3, 0, 0, &glyph(3, 3, 100)).expect("fits");
        let revision = atlas.revision();
        let second = atlas.insert(7u32, 3, 3, 0, 0, &glyph(3, 3, 250)).expect("known key");
        assert_eq!(first, second);
        assert_eq!(atlas.revision(), revision, "a known key does not touch the page");
        assert_eq!(atlas.len(), 1);
    }

    #[test]
    fn the_glyphs_pixels_land_where_the_entry_says() {
        let mut atlas = GlyphAtlas::new();
        atlas.insert(1u32, 2, 2, 0, 0, &glyph(2, 2, 111)).expect("fits");
        let entry = atlas.insert(2u32, 2, 2, 0, 0, &glyph(2, 2, 222)).expect("fits");
        let (page_w, _) = atlas.size();
        let at = |x: u32, y: u32| atlas.page()[(y as usize * page_w as usize + x as usize) * BYTES_PER_PIXEL];
        assert_eq!(at(entry.x, entry.y), 222, "the second glyph is where its entry points");
        assert_eq!(at(0, 0), 111, "and the first glyph is still where it was");
        assert!(entry.x > 0, "the second glyph shares the first shelf rather than starting a new one");
    }

    #[test]
    fn glyphs_are_separated_by_padding_so_a_filtered_sample_cannot_bleed() {
        let mut atlas = GlyphAtlas::new();
        let first = atlas.insert(1u32, 2, 2, 0, 0, &glyph(2, 2, 255)).expect("fits");
        let second = atlas.insert(2u32, 2, 2, 0, 0, &glyph(2, 2, 255)).expect("fits");
        assert!(second.x >= first.x + first.width + GLYPH_PADDING, "{second:?} crowds {first:?}");
    }

    #[test]
    fn an_empty_glyph_is_recorded_without_taking_room() {
        let mut atlas = GlyphAtlas::new();
        let entry = atlas.insert(1u32, 0, 0, 3, -4, &[]).expect("an empty glyph still gets an entry");
        assert!(entry.is_empty());
        assert_eq!((entry.left, entry.top), (3, -4));
        assert_eq!(atlas.revision(), 0, "nothing was written to the page");
    }

    #[test]
    fn a_tall_run_of_glyphs_grows_the_page_instead_of_failing() {
        let mut atlas = GlyphAtlas::new();
        let tall = ATLAS_INITIAL_HEIGHT / 2;
        for key in 0..4u32 {
            let entry = atlas.insert(key, ATLAS_MAX_DIM, tall, 0, 0, &glyph(ATLAS_MAX_DIM, tall, 30)).expect("grows to fit");
            assert_eq!(entry.width, ATLAS_MAX_DIM);
        }
        assert!(atlas.size().1 > ATLAS_INITIAL_HEIGHT, "the page grew");
        assert!(atlas.size().1 <= ATLAS_MAX_DIM, "and stayed inside the limit");
        assert_eq!(atlas.len(), 4);
    }

    #[test]
    fn a_full_page_is_emptied_and_restarted_rather_than_refusing_the_glyph() {
        let mut atlas = GlyphAtlas::new();
        let band = ATLAS_MAX_DIM / 4;
        let mut packed = 0;
        for key in 0..8u32 {
            if atlas.insert(key, ATLAS_MAX_DIM, band, 0, 0, &glyph(ATLAS_MAX_DIM, band, 30)).is_some() {
                packed += 1;
            }
        }
        assert_eq!(packed, 8, "every glyph was placed");
        assert!(atlas.len() < 8, "which means the page was restarted part way");
        assert_eq!(atlas.size().1, ATLAS_MAX_DIM, "and it had grown to the limit first");
    }

    #[test]
    fn a_glyph_larger_than_a_page_is_refused_without_wiping_what_is_packed() {
        let mut atlas = GlyphAtlas::new();
        atlas.insert(1u32, 4, 4, 0, 0, &glyph(4, 4, 90)).expect("fits");
        assert_eq!(atlas.insert(2u32, ATLAS_MAX_DIM + 1, 4, 0, 0, &[]), None, "wider than a page");
        assert_eq!(atlas.insert(3u32, 4, ATLAS_MAX_DIM + 1, 0, 0, &[]), None, "taller than a page");
        assert_eq!(atlas.len(), 1, "the packed glyph survived");
    }

    #[test]
    fn clearing_returns_the_page_to_its_starting_state() {
        let mut atlas = GlyphAtlas::new();
        atlas.insert(1u32, 8, 8, 0, 0, &glyph(8, 8, 255)).expect("fits");
        let revision = atlas.revision();
        atlas.clear();
        assert!(atlas.is_empty());
        assert_eq!(atlas.size(), (ATLAS_MAX_DIM, ATLAS_INITIAL_HEIGHT));
        assert!(atlas.page().iter().all(|b| *b == 0), "the page was wiped");
        assert!(atlas.revision() > revision, "clearing counts as a change");
    }

    #[test]
    fn an_entrys_uv_covers_exactly_its_pixels() {
        let entry = AtlasEntry { x: 4, y: 8, width: 2, height: 4, left: 0, top: 0 };
        let uv = entry.uv(16, 16);
        assert_eq!(uv, UvRect::new(4.0 / 16.0, 8.0 / 16.0, 6.0 / 16.0, 12.0 / 16.0));
    }

    #[test]
    fn a_binding_uploads_once_and_again_only_after_the_page_moves_on() {
        let mut canvas = CpuCanvas::new(4, 4);
        let mut atlas = GlyphAtlas::new();
        let mut binding = GlyphAtlasBinding::new();
        atlas.insert(1u32, 2, 2, 0, 0, &glyph(2, 2, 10)).expect("fits");

        let first = binding.sync(&mut canvas, &atlas);
        assert_eq!(canvas.live_texture_count(), 1);
        assert_eq!(binding.sync(&mut canvas, &atlas), first, "an unchanged page is not re-registered");
        assert_eq!(canvas.live_texture_count(), 1);

        atlas.insert(2u32, 2, 2, 0, 0, &glyph(2, 2, 20)).expect("fits");
        assert_eq!(binding.sync(&mut canvas, &atlas), first, "the page keeps one handle across revisions");
        assert_eq!(canvas.live_texture_count(), 1, "and re-uploading does not leak a second texture");
        assert_eq!(canvas.texture_size(first), Some(atlas.size()));
    }
}
