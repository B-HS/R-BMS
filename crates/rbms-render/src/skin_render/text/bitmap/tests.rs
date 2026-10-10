//! Tests of a `text` object drawn in a bitmap font: where its glyphs land, what the document's
//! `size`, `align`, `overflow` and `wrapping` do to them, its shadow and tint, the characters the
//! font lacks, and which of the font's pages are ever loaded.
//!
//! The font is the one `tests/skin/fonts/mini.fnt` describes, whose glyphs are flat boxes so that a
//! pixel either is a glyph's ink or is not, each in a colour of its own: `A` white, six by eight;
//! `B` red, four by eight, one pixel right of its pen; `y` green, six by ten, starting two rows
//! below the capital line; and an ideograph in blue, eight by eight. `A` and `B` are on the font's
//! first page and the other two on its second. `A` advances eight pixels, `B` seven, and a `B`
//! after an `A` is pulled two pixels towards it.
//!
//! The first half draws single objects with the pages already answered for, the way the text tests
//! next door do. The second half goes through a whole screen read from a document, which is where
//! the pages are asked for and where they are held to the screen's texture budget.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rbms_model::Mode;
use rbms_skin::dst::{DestinationTrack, Keyframe, SkinColor, SkinRect};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, StretchKind, load_skin_with_host};
use rbms_skin::model::TextDef;
use rbms_skin::property::MapHost;
use rbms_skin::property::generated::STRING_TITLE;
use rbms_skin::timer::TimerState;

use super::{BitmapKind, PageTable, ScreenFace, SharedPages};
use crate::bitmap_font;
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::draw::{ImageSelect, draw_object};
use crate::skin_render::object::{Body, ImageBody, SkinObject, Sprite};
use crate::skin_render::text::{FontRef, Fonts, text_body};
use crate::skin_render::textures::{SkinTexturePool, rgba_bytes};
use crate::skin_render::{FrameData, SkinAssets, SkinFrame, SkinImage, SkinScreen, SkinViewport};
use crate::{BlendMode, Color, CpuCanvas, Renderer, Theme, locked};

/// The canvas every line is drawn on, which is also the size the document is authored at.
const STAGE: (u32, u32) = (200, 100);

/// What the canvas is cleared to.
const BACKDROP: Color = Color { r: 100, g: 100, b: 100, a: u8::MAX };

/// The colours the fixture's glyphs are inked in.
const WHITE: Color = Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };
const RED: Color = Color { r: u8::MAX, g: 0, b: 0, a: u8::MAX };
const GREEN: Color = Color { r: 0, g: u8::MAX, b: 0, a: u8::MAX };
const BLUE: Color = Color { r: 0, g: 0, b: u8::MAX, a: u8::MAX };

/// The id the document registers its font under.
const FONT_ID: &str = "0";

/// The size the fixture font was made at, which draws it one to one.
const FONT_SIZE: i32 = 10;

/// The anchor the test lines are placed on, and the row their destination's top edge lands on.
const ANCHOR_X: i32 = 60;
const TOP_ROW: i32 = 30;

/// The height of the test lines' destination, which a bitmap text's size does not depend on.
const BOX_HEIGHT: f32 = 10.0;

/// The room a line that is meant to fit is given.
const ROOMY: f32 = 100.0;

/// The fixture's advances, the ink of its glyphs and its line height.
const ADVANCE_A: i32 = 8;
const KERN_AB: i32 = 2;
const INK_A: (i32, i32) = (6, 8);
const INK_B: (i32, i32) = (4, 8);
const BEARING_B: i32 = 1;
const INK_Y: (i32, i32) = (6, 10);
const DROP_Y: i32 = 2;
const INK_IDEOGRAPH: (i32, i32) = (8, 8);
const LINE_HEIGHT: i32 = 16;

/// How far a bilinear filter spreads the edge of a glyph drawn at twice its size.
const EDGE: i32 = 1;

/// The ideograph the fixture has a glyph for.
const IDEOGRAPH: &str = "\u{6f22}";

/// The files of the fixture.
const FONT_FILE: &str = "mini.fnt";
const PAGE_FILES: [&str; 2] = ["mini_0.png", "mini_1.png"];

/// How many bytes one RGBA pixel takes.
const RGBA_BYTES: usize = 4;

/// How many bytes one RGB pixel takes.
const RGB_BYTES: usize = 3;

/// The skin's blend value for additive drawing.
const BLEND_ADD: i32 = 2;

pub(super) fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("skin").join("fonts")
}

/// Decodes one of the fixture's page images.
pub(super) fn decode_png(path: &Path) -> Option<SkinImage> {
    let bytes = std::fs::read(path).ok()?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer.chunks_exact(RGB_BYTES).flat_map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX]).collect(),
        _ => return None,
    };
    SkinImage::new(info.width, info.height, rgba)
}

/// A text engine holding only the bundled face, the font table a screen would have built from the
/// fixture font, and that font as one screen's face with both of its pages uploaded to `canvas`.
fn engine(canvas: &mut CpuCanvas) -> (TextContext, Vec<(String, FontRef)>, Arc<ScreenFace>, SharedPages) {
    let (families, face, pages) = face_of(BitmapKind::Standard);
    for file in PAGE_FILES {
        upload(canvas, &pages, file);
    }
    (TextContext::embedded_only(), families, face, pages)
}

/// The fixture font as one screen's face, with none of its pages answered for.
fn face_of(kind: BitmapKind) -> (Vec<(String, FontRef)>, Arc<ScreenFace>, SharedPages) {
    face_from(FONT_FILE, kind)
}

/// The font `file` of the fixture folder as one screen's face, with none of its pages answered for.
pub(super) fn face_from(file: &str, kind: BitmapKind) -> (Vec<(String, FontRef)>, Arc<ScreenFace>, SharedPages) {
    let path = fixture_dir().join(file);
    let bytes = std::fs::read(&path).expect("the fixture font is readable");
    let font = bitmap_font::load(&path, &fixture_dir(), &bytes).expect("the fixture font loads");
    let pages = SharedPages::default();
    let face = Arc::new(ScreenFace::new(font, kind, Arc::clone(&pages)));
    (vec![(FONT_ID.to_string(), FontRef::Bitmap(Arc::clone(&face)))], face, pages)
}

/// Uploads one of the fixture's pages to `canvas` and answers for it in `pages`.
pub(super) fn upload(canvas: &mut CpuCanvas, pages: &SharedPages, file: &str) {
    let path = fixture_dir().join(file);
    let image = decode_png(&path).expect("the fixture page decodes");
    let tex = canvas.register_texture(&format!("test.page.{file}.{:p}", Arc::as_ptr(pages)), &image.rgba, image.width, image.height);
    locked(pages).settle(&path, Some((tex, (image.width, image.height))));
}

/// A destination of `width` anchored on [`ANCHOR_X`] with its top edge on [`TOP_ROW`], tinted
/// `color`.
fn track(width: f32, color: SkinColor) -> DestinationTrack {
    let rect = SkinRect::new(ANCHOR_X as f32, STAGE.1 as f32 - TOP_ROW as f32 - BOX_HEIGHT, width, BOX_HEIGHT);
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

/// A text object showing `line` written out in the registered font, in a destination of `width`.
fn constant(line: &str, families: &Fonts, width: f32, change: impl FnOnce(&mut TextDef)) -> SkinObject {
    let mut def = TextDef { font: FONT_ID.to_string(), size: FONT_SIZE, constant_text: Some(line.to_string()), ..TextDef::default() };
    change(&mut def);
    let white = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX);
    SkinObject { track: track(width, white), stretch: StretchKind::Stretch, body: Body::Text(text_body(&def, families)) }
}

fn stage() -> CpuCanvas {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    canvas
}

/// Draws one object the way a frame does, onto a canvas `scale` times the size the document was
/// authored at, answering whether anything reached the screen.
pub(super) fn paint_scaled(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject, authored: (f32, f32)) -> bool {
    let host = MapHost::new();
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &host, lua: None, mouse: None, data: FrameData::default() };
    let (width, height) = canvas.size();
    let viewport = SkinViewport::new(authored, (width as f32, height as f32));
    let mut ctx = RenderCtx::new(Theme::default(), text);
    draw_object(&mut ctx, canvas, object, &viewport, &frame)
}

fn paint(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject) -> bool {
    paint_scaled(text, canvas, object, (STAGE.0 as f32, STAGE.1 as f32))
}

/// Draws one object onto a cleared canvas, which has to put something on it.
fn shot(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject) {
    canvas.clear(BACKDROP);
    assert!(paint(text, canvas, object), "the line is drawn");
}

/// The box around every pixel `wanted` picks out, as `(left, top, right, bottom)` with the far
/// edges exclusive.
fn bounds(canvas: &CpuCanvas, wanted: impl Fn(Color) -> bool) -> Option<(i32, i32, i32, i32)> {
    let (width, height) = canvas.size();
    let lit: Vec<(i32, i32)> =
        (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).filter(|(x, y)| wanted(canvas.pixel_at(*x, *y))).map(|(x, y)| (x as i32, y as i32)).collect();
    let (xs, ys) = (lit.iter().map(|at| at.0), lit.iter().map(|at| at.1));
    Some((xs.clone().min()?, ys.clone().min()?, xs.max()? + 1, ys.max()? + 1))
}

/// The box around every pixel of exactly `color`.
fn ink_of(canvas: &CpuCanvas, color: Color) -> Option<(i32, i32, i32, i32)> {
    bounds(canvas, |pixel| pixel == color)
}

/// The box around every pixel that is not the backdrop.
fn ink(canvas: &CpuCanvas) -> Option<(i32, i32, i32, i32)> {
    bounds(canvas, |pixel| pixel != BACKDROP)
}

/// The box `size` pixels large with its top left corner at `(left, top)`.
fn boxed(left: i32, top: i32, size: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
    Some((left, top, left + size.0, top + size.1))
}

/// What [`boxed`] comes to for an enlarged glyph when every pixel its filtered edge touches is
/// counted: one pixel more on each side.
fn soft_boxed(left: i32, top: i32, size: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
    boxed(left - EDGE, top - EDGE, (size.0 + 2 * EDGE, size.1 + 2 * EDGE))
}

/// What [`boxed`] comes to for an enlarged glyph when only the pixels of its full colour are
/// counted: one pixel less on each side.
fn firm_boxed(left: i32, top: i32, size: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
    boxed(left + EDGE, top + EDGE, (size.0 - 2 * EDGE, size.1 - 2 * EDGE))
}

#[test]
fn a_glyph_lands_where_its_offsets_its_advance_and_the_kerning_put_it() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);

    shot(&mut text, &mut canvas, &constant("AB", &families, ROOMY, |_| {}));
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A), "the first glyph's ink starts on the anchor and on the capital line");
    assert_eq!(
        ink_of(&canvas, RED),
        boxed(ANCHOR_X + ADVANCE_A - KERN_AB + BEARING_B, TOP_ROW, INK_B),
        "the second follows the first's advance, pulled in by the kerning pair"
    );

    shot(&mut text, &mut canvas, &constant("BA", &families, ROOMY, |_| {}));
    assert_eq!(ink_of(&canvas, RED), boxed(ANCHOR_X + BEARING_B, TOP_ROW, INK_B), "a line starts at its first glyph's origin, not at its ink");
}

#[test]
fn a_glyph_of_another_page_and_one_below_the_capital_line_land_as_their_lines_say() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);

    shot(&mut text, &mut canvas, &constant(&format!("y{IDEOGRAPH}"), &families, ROOMY, |_| {}));
    assert_eq!(ink_of(&canvas, GREEN), boxed(ANCHOR_X, TOP_ROW + DROP_Y, INK_Y));
    assert_eq!(ink_of(&canvas, BLUE), boxed(ANCHOR_X + ADVANCE_A, TOP_ROW, INK_IDEOGRAPH));
}

#[test]
fn the_size_scales_the_glyphs_and_the_destination_height_does_not() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);

    shot(&mut text, &mut canvas, &constant("A", &families, ROOMY, |def| def.size = 2 * FONT_SIZE));
    assert_eq!(ink(&canvas), soft_boxed(ANCHOR_X, TOP_ROW, (2 * INK_A.0, 2 * INK_A.1)), "twice the font's own size is twice the glyph");
    assert_eq!(
        ink_of(&canvas, WHITE),
        firm_boxed(ANCHOR_X, TOP_ROW, (2 * INK_A.0, 2 * INK_A.1)),
        "filtered bilinearly, so its edge is a pixel soft either way"
    );

    let mut tall = constant("A", &families, ROOMY, |_| {});
    tall.track.frames[0].rect = SkinRect::new(ANCHOR_X as f32, STAGE.1 as f32 - TOP_ROW as f32 - 4.0 * BOX_HEIGHT, ROOMY, 4.0 * BOX_HEIGHT);
    shot(&mut text, &mut canvas, &tall);
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A), "a taller destination moves nothing and enlarges nothing");
}

#[test]
fn a_screen_larger_than_the_skin_enlarges_the_glyphs_with_it() {
    let mut canvas = CpuCanvas::new(2 * STAGE.0, 2 * STAGE.1);
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let object = constant("A", &families, ROOMY, |_| {});

    canvas.clear(BACKDROP);
    assert!(paint_scaled(&mut text, &mut canvas, &object, (STAGE.0 as f32, STAGE.1 as f32)));
    assert_eq!(ink(&canvas), soft_boxed(2 * ANCHOR_X, 2 * TOP_ROW, (2 * INK_A.0, 2 * INK_A.1)), "the size is in the skin's own pixels");
}

#[test]
fn the_destination_x_is_where_a_line_starts_centres_or_ends() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let mut aligned = |align: i32| {
        shot(&mut text, &mut canvas, &constant("AA", &families, ROOMY, |def| def.align = align));
        ink_of(&canvas, WHITE).expect("the line has ink")
    };
    let advance = 2 * ADVANCE_A;
    let inked = ADVANCE_A + INK_A.0;

    assert_eq!(aligned(0).0, ANCHOR_X, "a left-aligned line starts on the anchor");
    assert_eq!(aligned(1).0, ANCHOR_X - advance / 2, "a centred line has the middle of its advances on the anchor");
    assert_eq!(aligned(2).0, ANCHOR_X - advance, "a right-aligned line ends its last advance on the anchor");
    assert_eq!(aligned(2).2, ANCHOR_X - advance + inked, "which leaves its last glyph's ink short of the anchor by the glyph's right bearing");
}

#[test]
fn a_line_too_long_is_squeezed_cut_or_left_to_run_on() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let narrow = 2.0 * ADVANCE_A as f32;
    let mut fitted = |overflow: i32| {
        shot(&mut text, &mut canvas, &constant("AAAA", &families, narrow, |def| def.overflow = overflow));
        ink_of(&canvas, WHITE).expect("the line has ink")
    };

    let running = fitted(0);
    assert_eq!((running.0, running.2), (ANCHOR_X, ANCHOR_X + 3 * ADVANCE_A + INK_A.0), "overflow 0 runs past its room");

    let squeezed = fitted(1);
    assert_eq!((squeezed.0, squeezed.2), (ANCHOR_X, ANCHOR_X + 3 * ADVANCE_A / 2 + INK_A.0 / 2), "overflow 1 squeezes the advances and the glyphs into it");
    assert_eq!((squeezed.1, squeezed.3), (TOP_ROW, TOP_ROW + INK_A.1), "and leaves the height alone");

    let cut = fitted(2);
    assert_eq!(
        (cut.0, cut.2),
        (ANCHOR_X, ANCHOR_X + ADVANCE_A + INK_A.0),
        "overflow 2 keeps the glyphs whose advance ends inside it: two, in room for two advances"
    );
}

#[test]
fn a_wrapped_line_continues_one_line_height_down() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let room = 2.5 * ADVANCE_A as f32;

    shot(&mut text, &mut canvas, &constant("AA BA", &families, room, |def| (def.wrapping, def.overflow) = (true, 2)));
    assert_eq!(
        ink_of(&canvas, WHITE),
        Some((ANCHOR_X, TOP_ROW, ANCHOR_X + ADVANCE_A + INK_A.0, TOP_ROW + LINE_HEIGHT + INK_A.1)),
        "wrapping wins over the overflow"
    );
    assert_eq!(ink_of(&canvas, RED), boxed(ANCHOR_X, TOP_ROW + LINE_HEIGHT, INK_B), "and the continuation starts at its first glyph's ink");
}

#[test]
fn a_shadow_is_the_line_at_half_brightness_moved_right_and_down() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let offset = (3, 2);

    shot(&mut text, &mut canvas, &constant("A", &families, ROOMY, |def| (def.shadow_offset_x, def.shadow_offset_y) = (offset.0 as f32, offset.1 as f32)));
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A), "the line itself is where it was, drawn over its shadow");
    let shade = Color { r: u8::MAX / 2, g: u8::MAX / 2, b: u8::MAX / 2, a: u8::MAX };
    let shadow = ink_of(&canvas, shade).expect("the shadow shows past the line");
    assert_eq!((shadow.2, shadow.3), (ANCHOR_X + offset.0 + INK_A.0, TOP_ROW + offset.1 + INK_A.1), "a positive offset puts it to the right and below");
    assert_eq!(ink(&canvas), Some((ANCHOR_X, TOP_ROW, shadow.2, shadow.3)));
}

#[test]
fn the_destination_colour_tints_the_glyphs_and_fades_them() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);

    let mut tinted = constant("A", &families, ROOMY, |_| {});
    tinted.track = track(ROOMY, SkinColor::rgba(u8::MAX, 0, u8::MAX, u8::MAX));
    shot(&mut text, &mut canvas, &tinted);
    assert_eq!(ink_of(&canvas, Color { r: u8::MAX, g: 0, b: u8::MAX, a: u8::MAX }), boxed(ANCHOR_X, TOP_ROW, INK_A));

    let mut faded = constant("A", &families, ROOMY, |_| {});
    faded.track = track(ROOMY, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 0));
    canvas.clear(BACKDROP);
    paint(&mut text, &mut canvas, &faded);
    assert_eq!(ink(&canvas), None, "a line with no opacity leaves nothing");
}

#[test]
fn a_bitmap_text_is_blended_the_way_the_object_before_it_was() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let tex = canvas.register_texture("dot", &[0, 0, 0, u8::MAX], 1, 1);
    let dot = |blend: i32| {
        let sprite = Sprite { tex, size: (1, 1), origin: (0, 0), cell: (1, 1), columns: 1, rows: 1, timer: None, cycle: 0 };
        let mut track = track(1.0, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX));
        track.blend = blend;
        SkinObject { track, stretch: StretchKind::Stretch, body: Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First }) }
    };
    let dim = |blend: i32| {
        let mut object = constant("A", &families, ROOMY, |_| {});
        object.track = track(ROOMY, SkinColor::rgba(50, 50, 50, u8::MAX));
        object.track.blend = blend;
        object
    };
    let brightest = |canvas: &CpuCanvas| (0..STAGE.1).flat_map(|y| (0..STAGE.0).map(move |x| canvas.pixel_at(x, y).r)).max().unwrap_or_default();
    let darkest = |canvas: &CpuCanvas| (0..STAGE.1).flat_map(|y| (0..STAGE.0).map(move |x| canvas.pixel_at(x, y).r)).min().unwrap_or_default();

    assert!(paint(&mut text, &mut canvas, &dot(BLEND_ADD)));
    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &dim(0)));
    assert_eq!((darkest(&canvas), brightest(&canvas)), (BACKDROP.r, BACKDROP.r + 50), "after an additive image the ink is added to what is under it");
    assert_eq!(text.inherited_blend(), BlendMode::Add, "and the text leaves the blend as it found it");

    assert!(paint(&mut text, &mut canvas, &dot(0)));
    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &dim(BLEND_ADD)));
    assert_eq!((darkest(&canvas), brightest(&canvas)), (50, BACKDROP.r), "after a plain image it covers, whatever blend its own destination asks for");
}

#[test]
fn a_character_the_font_lacks_is_drawn_from_another_font_on_the_same_line() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);

    shot(&mut text, &mut canvas, &constant("AB", &families, ROOMY, |_| {}));
    let close = ink_of(&canvas, RED).expect("the B has ink");

    shot(&mut text, &mut canvas, &constant("AXB", &families, ROOMY, |_| {}));
    let apart = ink_of(&canvas, RED).expect("the B has ink");
    let own = ANCHOR_X + INK_A.0;
    let between = bounds(&canvas, |pixel| pixel != BACKDROP && pixel != RED).expect("the A and the stand-in have ink");
    let stand_in = bounds(&canvas, |pixel| pixel == WHITE).filter(|_| between.2 > own + 1).expect("the stand-in has ink right of the A");

    assert!(apart.0 >= close.0 + KERN_AB + 3, "the B is pushed along by the stand-in's advance: {close:?} to {apart:?}");
    assert!(between.2 > own && between.2 <= apart.0, "the stand-in's ink sits between the A and the B: {between:?}, B at {apart:?}");
    assert!((stand_in.1 - TOP_ROW).abs() <= 1, "a capital from the other font stands on the same capital line: {stand_in:?}");
    assert_eq!((apart.1, apart.3), (close.1, close.3), "and the line stays on its row");
}

#[test]
fn a_line_made_of_nothing_but_missing_characters_is_still_drawn() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);

    shot(&mut text, &mut canvas, &constant("XX", &families, ROOMY, |def| (def.shadow_offset_x, def.shadow_offset_y) = (2.0, 2.0)));
    let drawn = ink(&canvas).expect("the stand-ins have ink");
    assert!(drawn.0 >= ANCHOR_X && drawn.0 <= ANCHOR_X + 2, "it starts on the anchor, give or take its bearing: {drawn:?}");
    assert!((drawn.1 - TOP_ROW).abs() <= 1, "on the capital line: {drawn:?}");
}

/// A text reading the chart's title in the registered font, in a destination of [`ROOMY`]: a text
/// whose string is not known until it is drawn.
fn titled(fonts: &Fonts) -> SkinObject {
    let def = TextDef { font: FONT_ID.to_string(), size: FONT_SIZE, reference: STRING_TITLE, ..TextDef::default() };
    let white = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX);
    SkinObject { track: track(ROOMY, white), stretch: StretchKind::Stretch, body: Body::Text(text_body(&def, fonts)) }
}

/// Draws `object` on the cleared canvas with the chart's title reading `title`, answering whether
/// anything reached the screen.
fn paint_titled(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject, title: &str) -> bool {
    let mut host = MapHost::new();
    host.texts.insert(STRING_TITLE, title.to_string());
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &host, lua: None, mouse: None, data: FrameData::default() };
    let viewport = SkinViewport::new((STAGE.0 as f32, STAGE.1 as f32), (STAGE.0 as f32, STAGE.1 as f32));
    canvas.clear(BACKDROP);
    let mut ctx = RenderCtx::new(Theme::default(), text);
    draw_object(&mut ctx, canvas, object, &viewport, &frame)
}

/// A page is asked for by the first line with a glyph on it, and a glyph is drawn from the frame its
/// page is in. The glyphs of the same line that are on other pages do not wait for it: a line whose
/// string has just changed shows what it can at once.
#[test]
fn a_glyph_waits_for_its_own_page_and_the_rest_of_its_line_does_not() {
    let mut canvas = stage();
    let (fonts, _face, pages) = face_of(BitmapKind::Standard);
    let mut text = TextContext::embedded_only();
    let first = fixture_dir().join(PAGE_FILES[0]);
    let second = fixture_dir().join(PAGE_FILES[1]);
    let title = titled(&fonts);
    assert!(locked(&pages).wanted().is_empty(), "a text whose string is read when it is drawn asks for nothing before it is");

    assert!(!paint_titled(&mut text, &mut canvas, &title, "AB"), "nothing is drawn before the page is in");
    assert_eq!(ink(&canvas), None);
    assert_eq!(locked(&pages).wanted(), std::slice::from_ref(&first), "the page its glyphs are on is asked for, and no other");

    upload(&mut canvas, &pages, PAGE_FILES[0]);
    assert!(paint_titled(&mut text, &mut canvas, &title, "AB"), "and the line is drawn once it is");
    assert!(locked(&pages).wanted().is_empty());

    let mixed = format!("A{IDEOGRAPH}");
    for _ in 0..2 {
        assert!(paint_titled(&mut text, &mut canvas, &title, &mixed), "a line with one page of two draws the glyphs of the one it has");
        assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A), "where they belong in the whole line");
        assert_eq!(ink_of(&canvas, BLUE), None, "and none of the page it is waiting for, on every frame it waits");
        assert_eq!(locked(&pages).wanted(), std::slice::from_ref(&second));
    }

    upload(&mut canvas, &pages, PAGE_FILES[1]);
    assert!(paint_titled(&mut text, &mut canvas, &title, &mixed));
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A));
    assert_eq!(ink_of(&canvas, BLUE), boxed(ANCHOR_X + ADVANCE_A, TOP_ROW, INK_IDEOGRAPH), "the glyph that waited joins the line where it was laid out");
}

/// A page that was refused is not waited for: the line is drawn without its glyphs for good.
#[test]
fn a_refused_page_leaves_its_glyphs_out_and_is_not_asked_for_again() {
    let mut canvas = stage();
    let (fonts, _face, pages) = face_of(BitmapKind::Standard);
    let mut text = TextContext::embedded_only();
    upload(&mut canvas, &pages, PAGE_FILES[0]);
    let title = titled(&fonts);
    let mixed = format!("A{IDEOGRAPH}");

    paint_titled(&mut text, &mut canvas, &title, &mixed);
    locked(&pages).settle(&fixture_dir().join(PAGE_FILES[1]), None);
    assert!(paint_titled(&mut text, &mut canvas, &title, &mixed));
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A), "the glyphs of the pages that are in are drawn");
    assert_eq!(ink_of(&canvas, BLUE), None, "and the ones of the refused page are not");
    assert!(locked(&pages).wanted().is_empty());
}

/// A string the document wrote out is known when the screen is built, so the pages its glyphs are on
/// are asked for then, before the object is ever drawn. A character the font lacks is on no page.
#[test]
fn a_written_out_string_asks_for_its_pages_when_the_object_is_built() {
    let (fonts, _face, pages) = face_of(BitmapKind::Standard);
    let first = fixture_dir().join(PAGE_FILES[0]);
    let second = fixture_dir().join(PAGE_FILES[1]);

    constant("X A", &fonts, ROOMY, |_| {});
    assert_eq!(locked(&pages).wanted(), std::slice::from_ref(&first), "the page of the glyphs it has, and nothing for the space or the stand-in");
    constant("A", &fonts, ROOMY, |def| def.reference = STRING_TITLE);
    constant("", &fonts, ROOMY, |_| {});
    assert_eq!(locked(&pages).wanted(), std::slice::from_ref(&first), "a string written beside a property is never shown, and asks for nothing");
    constant(&format!("y{IDEOGRAPH}"), &fonts, ROOMY, |_| {});
    assert_eq!(locked(&pages).wanted(), [first, second]);
}

/// The glyphs of a line are drawn a page at a time, as the reference's font cache draws them, and
/// in the line's own order within a page.
#[test]
fn a_lines_glyphs_are_drawn_page_by_page() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas);
    let object = constant(&format!("yA{IDEOGRAPH}B"), &families, ROOMY, |_| {});
    shot(&mut text, &mut canvas, &object);

    let Body::Text(body) = &object.body else {
        panic!("the object is a text");
    };
    let bitmap = body.bitmap.as_ref().expect("the text is in a bitmap font").borrow();
    let drawn: Vec<(usize, u32)> = bitmap.laid.as_ref().expect("the line was laid out").quads.iter().map(|quad| (quad.page, quad.src.0)).collect();
    assert_eq!(drawn, [(0, 0), (0, 10), (1, 0), (1, 10)], "the A and the B of the first page, then the y and the ideograph of the second");
}

#[test]
fn the_font_type_says_how_the_pages_are_read() {
    assert_eq!(BitmapKind::of(0), BitmapKind::Standard);
    assert_eq!(BitmapKind::of(1), BitmapKind::DistanceField);
    assert_eq!(BitmapKind::of(2), BitmapKind::DistanceField);
    assert_eq!(BitmapKind::of(3), BitmapKind::Standard);
    assert_eq!(BitmapKind::of(-1), BitmapKind::Standard);

    let (families, face, _pages) = face_of(BitmapKind::DistanceField);
    let def = TextDef { font: FONT_ID.to_string(), size: FONT_SIZE, ..TextDef::default() };
    let body = text_body(&def, &families);
    assert!(body.family.is_none(), "a text in a bitmap font has no TrueType family");
    let bitmap = body.bitmap.as_ref().expect("it has the face its font id loaded as");
    assert!(Arc::ptr_eq(bitmap.borrow().face(), &face));
    assert_eq!(bitmap.borrow().face().kind(), BitmapKind::DistanceField);

    let unknown = text_body(&TextDef { font: "no such font".to_string(), ..def }, &families);
    assert!(unknown.bitmap.is_none() && unknown.family.is_none());
}

#[test]
fn a_page_table_forgets_everything_when_it_is_cleared() {
    let mut table = PageTable::default();
    let path = fixture_dir().join(PAGE_FILES[0]);
    let mut canvas = stage();
    let tex = canvas.register_texture("page", &[0; RGBA_BYTES], 1, 1);

    assert!(!table.is_wanted(&path));
    table.settle(&path, Some((tex, (1, 1))));
    assert_eq!((table.loaded(), table.wanted().len()), (1, 0));
    table.clear();
    assert_eq!((table.loaded(), table.wanted().len()), (0, 0));
}

/// A skin folder of the test's own holding a copy of the fixture font, removed when the test is
/// done with it.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-bitmap-font-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        for file in PAGE_FILES.into_iter().chain([FONT_FILE]) {
            std::fs::copy(fixture_dir().join(file), root.join(file)).expect("the fixture is copied");
        }
        Scratch { root }
    }

    /// Writes a decide-screen document with `fonts` and one text reading the chart's title in
    /// font 0, and reads it.
    fn load(&self, fonts: &str) -> LoadedSkin {
        let (width, height) = STAGE;
        let bottom = height as i32 - TOP_ROW - BOX_HEIGHT as i32;
        let document = format!(
            r#"{{
                "type": 6, "name": "bitmap font", "w": {width}, "h": {height},
                "font": [{fonts}],
                "text": [{{ "id": "title", "font": "{FONT_ID}", "size": {FONT_SIZE}, "ref": {STRING_TITLE} }}],
                "destination": [{{ "id": "title", "dst": [{{ "time": 0, "x": {ANCHOR_X}, "y": {bottom}, "w": {ROOMY}, "h": {BOX_HEIGHT} }}] }}]
            }}"#
        );
        let path = self.root.join("skin.json");
        std::fs::write(&path, document).expect("the scratch document is writable");
        let user = SkinUserConfig::default();
        let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&self.root, &user, Mode::BEAT_7K) };
        load_skin_with_host(&path, options, &MapHost::new()).expect("the scratch document loads")
    }
}

/// The file `name` beside the font of `skin`, as the font names its pages.
fn beside_font(skin: &LoadedSkin, name: &str) -> PathBuf {
    skin.fonts[FONT_ID].parent().expect("the font is in a folder").join(name)
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A host that reads what it is asked for off the disk and remembers every image it was asked for.
#[derive(Default)]
struct Disk {
    asked: Vec<String>,
}

impl SkinAssets for Disk {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        self.asked.push(path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned());
        decode_png(path)
    }
}

/// Draws one whole frame of `screen` with the chart's title reading `title`, answering how many
/// objects put something on the cleared canvas.
fn frame(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, title: &str) -> usize {
    let mut host = MapHost::new();
    host.texts.insert(STRING_TITLE, title.to_string());
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &host, lua: None, mouse: None, data: FrameData::default() };
    canvas.clear(BACKDROP);
    let mut ctx = RenderCtx::new(Theme::default(), text);
    screen.draw(&mut ctx, canvas, &frame)
}

#[test]
fn a_screen_loads_only_the_pages_its_text_has_come_to_need() {
    let scratch = Scratch::new("pages");
    let skin = scratch.load(&format!(r#"{{ "id": "{FONT_ID}", "path": "{FONT_FILE}" }}"#));
    let mut canvas = stage();
    let mut text = TextContext::embedded_only();
    let mut assets = Disk::default();
    let mut screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut assets);

    assert!(screen.warnings().iter().all(|warning| warning.contains("font \"0\": line")), "only the fixture's broken lines are said: {:?}", screen.warnings());
    assert_eq!(screen.warnings().len(), 3);
    assert_eq!((screen.bitmap_font_count(), screen.families().len()), (1, 0), "a bitmap font is no family of the text engine's");
    assert!(assets.asked.is_empty(), "building the screen decodes no page");
    assert!(screen.wanted_font_pages().is_empty());

    assert_eq!(frame(&screen, &mut text, &mut canvas, "AB"), 0, "the first frame has no page to draw from");
    assert_eq!(screen.wanted_font_pages(), [beside_font(&skin, PAGE_FILES[0])]);
    assert_eq!(screen.load_font_pages(&mut canvas, &mut assets, None), 1);
    assert_eq!(assets.asked, [PAGE_FILES[0]], "the page the title's glyphs are on is decoded, and no other");
    assert_eq!((screen.font_page_count(), screen.texture_stats().count), (1, 1));

    assert_eq!(frame(&screen, &mut text, &mut canvas, "AB"), 1);
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A));
    assert_eq!(ink_of(&canvas, RED), boxed(ANCHOR_X + ADVANCE_A - KERN_AB + BEARING_B, TOP_ROW, INK_B));
    assert!(screen.wanted_font_pages().is_empty());
    assert_eq!(screen.load_font_pages(&mut canvas, &mut assets, None), 0);

    assert_eq!(frame(&screen, &mut text, &mut canvas, IDEOGRAPH), 0, "a title on a page not yet in waits for it");
    assert_eq!(screen.wanted_font_pages(), [beside_font(&skin, PAGE_FILES[1])]);
    assert_eq!(screen.load_font_pages(&mut canvas, &mut assets, None), 1);
    assert_eq!(assets.asked, PAGE_FILES);
    assert_eq!(frame(&screen, &mut text, &mut canvas, IDEOGRAPH), 1);
    assert_eq!(ink_of(&canvas, BLUE), boxed(ANCHOR_X, TOP_ROW, INK_IDEOGRAPH));
    assert_eq!(screen.texture_stats().bytes, 2 * rgba_bytes((32, 16)), "a page counts against the screen's textures like any source");

    screen.release(&mut canvas);
    assert_eq!((screen.font_page_count(), screen.texture_stats().count), (0, 0));
}

#[test]
fn a_page_is_shared_through_the_pool_and_held_to_its_budget() {
    let scratch = Scratch::new("pool");
    let skin = scratch.load(&format!(r#"{{ "id": "{FONT_ID}", "path": "{FONT_FILE}" }}"#));
    let mut canvas = stage();
    let mut text = TextContext::embedded_only();
    let mut pool = SkinTexturePool::with_budget(rgba_bytes((32, 16)));
    let first_page = beside_font(&skin, PAGE_FILES[0]);
    let second_page = beside_font(&skin, PAGE_FILES[1]);

    let mut assets = Disk::default();
    let mut screen = SkinScreen::build_shared(&mut canvas, &mut text, &skin, &mut assets, &mut pool);
    frame(&screen, &mut text, &mut canvas, "A");
    assert!(screen.settle_font_page(&mut canvas, &first_page, &mut assets, Some(&mut pool)));
    assert_eq!(pool.holders(&first_page), 1, "the page is the pool's, held by the screen");

    let mut other_assets = Disk::default();
    let mut other = SkinScreen::build_shared(&mut canvas, &mut text, &skin, &mut other_assets, &mut pool);
    frame(&other, &mut text, &mut canvas, "A");
    struct Nothing;
    impl SkinAssets for Nothing {
        fn image(&mut self, _path: &Path) -> Option<SkinImage> {
            None
        }
    }
    assert!(other.settle_font_page(&mut canvas, &first_page, &mut Nothing, Some(&mut pool)), "a second screen draws from the texture that is there");
    assert_eq!(pool.holders(&first_page), 2);
    assert_eq!(frame(&other, &mut text, &mut canvas, "A"), 1);

    assert!(!screen.settle_font_page(&mut canvas, &second_page, &mut assets, Some(&mut pool)), "a page nobody asked for is not taken");
    frame(&screen, &mut text, &mut canvas, IDEOGRAPH);
    let warned = screen.warnings().len();
    assert!(!screen.settle_font_page(&mut canvas, &second_page, &mut assets, Some(&mut pool)), "a page past the budget is refused");
    assert_eq!(screen.warnings().len(), warned + 1, "{:?}", screen.warnings());
    assert!(screen.warnings().last().is_some_and(|warning| warning.contains("font page") && warning.contains("MiB")), "{:?}", screen.warnings());
    assert!(screen.wanted_font_pages().is_empty(), "and is not asked for again");
    assert_eq!(frame(&screen, &mut text, &mut canvas, IDEOGRAPH), 0, "its glyphs draw nothing");
    assert_eq!(frame(&screen, &mut text, &mut canvas, "A"), 1, "and the glyphs of the page that is in still do");

    screen.release_shared(&mut canvas, &mut pool);
    assert_eq!(pool.holders(&first_page), 1);
    other.release_shared(&mut canvas, &mut pool);
    assert_eq!((pool.holders(&first_page), pool.sweep(&mut canvas)), (0, 1), "the page goes when the last screen holding it has");
}

#[test]
fn a_font_file_that_is_no_font_leaves_a_warning_and_the_stand_in_face() {
    let scratch = Scratch::new("broken");
    std::fs::write(scratch.root.join("broken.fnt"), "this is not a font\n").expect("the broken font is writable");
    let skin = scratch.load(&format!(r#"{{ "id": "{FONT_ID}", "path": "broken.fnt" }}"#));
    let mut canvas = stage();
    let mut text = TextContext::embedded_only();
    let mut assets = Disk::default();
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut assets);

    assert_eq!(screen.bitmap_font_count(), 0);
    assert!(screen.warnings().iter().any(|warning| warning.contains("could not be loaded") && warning.contains("common line")), "{:?}", screen.warnings());
    assert_eq!(frame(&screen, &mut text, &mut canvas, "AB"), 1, "the text is drawn in the default face, as for any font that did not load");
    assert!(screen.wanted_font_pages().is_empty());
}

/// A font file is a stranger's, and the numbers in it are whatever it says. A glyph that advances
/// the pen by the largest number a line can give puts what follows it further off than any screen
/// reaches -- its neighbour in the font, and the stand-in for a character the font lacks, which is
/// composed into a texture from where it lands. Nothing of that is more than a line drawn far away.
#[test]
fn a_font_with_an_advance_past_any_screen_is_still_only_a_line_of_text() {
    let scratch = Scratch::new("far-advance");
    let far = i32::MAX;
    let font = format!(
        "info size={FONT_SIZE} padding=0,0,0,0\ncommon lineHeight={LINE_HEIGHT} base=12 scaleW=32 scaleH=16 pages=1\npage id=0 file=\"{}\"\nchar id=65 x=0 y=0 width=10 height=12 xoffset=0 yoffset=2 xadvance={far} page=0\nchar id=66 x=10 y=0 width=8 height=12 xoffset=0 yoffset=2 xadvance={far} page=0\nkerning first=65 second=66 amount=1\nkerning first=66 second=65 amount=-1\n",
        PAGE_FILES[0]
    );
    std::fs::write(scratch.root.join("far.fnt"), font).expect("the made-up font is writable");
    let skin = scratch.load(&format!(r#"{{ "id": "{FONT_ID}", "path": "far.fnt" }}"#));
    let mut canvas = stage();
    let mut text = TextContext::embedded_only();
    let mut assets = Disk::default();
    let mut screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut assets);
    assert!(screen.warnings().is_empty(), "{:?}", screen.warnings());

    let lines = ["AB", "AXB", "XAX", "ABABAB XX", "A\nBX"];
    for line in lines {
        frame(&screen, &mut text, &mut canvas, line);
    }
    assert_eq!(screen.load_font_pages(&mut canvas, &mut assets, None), 1);
    for line in lines {
        assert_eq!(frame(&screen, &mut text, &mut canvas, line), 1, "{line:?} is drawn, its first glyph on the anchor");
    }
}

/// A distance field font is a bitmap font like any other to the screen: it is not warned about,
/// its pages are asked for the same way, and its glyphs sit where a plain font's do. The fixture's
/// pages are all or nothing, which read as distances is a glyph with a hard edge.
#[test]
fn a_distance_field_font_is_laid_out_and_loaded_like_a_plain_one() {
    let scratch = Scratch::new("distance-field");
    let skin = scratch.load(&format!(r#"{{ "id": "{FONT_ID}", "path": "{FONT_FILE}", "type": 1 }}"#));
    let mut canvas = stage();
    let mut text = TextContext::embedded_only();
    let mut assets = Disk::default();
    let mut screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut assets);

    assert!(!screen.warnings().iter().any(|warning| warning.contains("distance field")), "{:?}", screen.warnings());
    frame(&screen, &mut text, &mut canvas, "A");
    assert_eq!(screen.load_font_pages(&mut canvas, &mut assets, None), 1);
    assert_eq!(frame(&screen, &mut text, &mut canvas, "A"), 1);
    assert_eq!(ink_of(&canvas, WHITE), boxed(ANCHOR_X, TOP_ROW, INK_A), "its glyphs sit where a plain font's do");
}
