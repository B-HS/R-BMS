//! Tests of a `text` object drawn in a distance field font: that it goes through the reference's
//! distance field shader, and that what the document says of its outline and its shadow reaches the
//! shader the way `SkinTextBitmap.setDistanceFieldUniforms` hands it over.
//!
//! The font is `tests/skin/fonts/field.fnt`, made at 32 pixels with one page of 128 by 64. The page
//! is cyan throughout -- no test draws in cyan, so a path that showed the page's own colour would
//! show -- and its alpha is the distance to each glyph's outline: half on the outline, everything
//! four texels inside it and nothing four texels outside. An `O` is a disc of twelve texels' radius
//! whose middle is sixteen right of the pen and sixteen below the destination's top edge; an `I` is
//! an upright bar eight texels wide and twenty-four tall. The shader's ramp of a sixteenth either
//! side of the outline is half a texel either side of it.

use std::path::Path;
use std::sync::Arc;

use rbms_model::Mode;
use rbms_skin::dst::{DestinationTrack, Keyframe, SkinColor, SkinRect};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, StretchKind, load_skin_with_host};
use rbms_skin::model::TextDef;
use rbms_skin::property::MapHost;
use rbms_skin::timer::TimerState;

use super::tests::{decode_png, face_from, fixture_dir, paint_scaled, upload};
use super::{BitmapKind, FieldInk, ScreenFace, SharedPages, UNREADABLE_COLOR, written_color};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::object::{Body, SkinObject};
use crate::skin_render::text::{FontRef, Fonts, text_body};
use crate::skin_render::{FrameData, SkinAssets, SkinFrame, SkinImage, SkinScreen};
use crate::{BlendMode, Color, CpuCanvas, DISTANCE_FIELD_EDGE, DistanceFieldStyle, Renderer, Theme};

/// The canvas the lines are drawn on, which is also the size the documents are authored at.
const STAGE: (u32, u32) = (240, 160);

const BACKDROP: Color = Color { r: 100, g: 100, b: 100, a: u8::MAX };
const WHITE: Color = Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };
const BLACK: Color = Color { r: 0, g: 0, b: 0, a: u8::MAX };
const RED: Color = Color { r: u8::MAX, g: 0, b: 0, a: u8::MAX };
const GREEN: Color = Color { r: 0, g: u8::MAX, b: 0, a: u8::MAX };
const BLUE: Color = Color { r: 0, g: 0, b: u8::MAX, a: u8::MAX };

/// The colour of the fixture's page, which only a path that is not the shader's shows.
const PAGE_CYAN: Color = Color { r: 0, g: 128, b: u8::MAX, a: u8::MAX };

/// The files of the fixture and the size of its page.
const FONT_FILE: &str = "field.fnt";
const PAGE_FILE: &str = "field_0.png";
const PAGE_SIZE: (u32, u32) = (128, 64);

/// The id the font is registered under, and the size it was made at.
const FONT_ID: &str = "0";
const FONT_SIZE: i32 = 32;

/// The anchor of the test lines and the row their destination's top edge is on, far enough in for
/// the disc at twice its size and its shadow.
const ANCHOR_X: i32 = 40;
const TOP_ROW: i32 = 20;

/// The room a line is given and the height of its destination, which its size does not depend on.
const ROOM: f32 = 160.0;
const BOX_HEIGHT: f32 = 10.0;

/// How far right of the pen and below the top edge the middle of the fixture's `O` is, and the
/// radius of its disc, at the size the font was made at.
const DISC_MIDDLE: i32 = 16;
const DISC_RADIUS: f32 = 12.0;

/// How many texels of the fixture one unit of the field's range is: four either side of an outline.
const FIELD_RANGE_TEXELS: f32 = 8.0;

/// How far from the middle of the `O` its quad reaches, in texels: nothing of the glyph, its outline
/// or its shadow is drawn past that.
const CELL_REACH: i32 = 20;

/// The fixture font as one screen's distance field face, with its page uploaded to `canvas`.
fn engine(canvas: &mut CpuCanvas, kind: BitmapKind) -> (TextContext, Vec<(String, FontRef)>, Arc<ScreenFace>, SharedPages) {
    let (families, face, pages) = face_from(FONT_FILE, kind);
    upload(canvas, &pages, PAGE_FILE);
    (TextContext::embedded_only(), families, face, pages)
}

fn stage() -> CpuCanvas {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    canvas
}

/// A text object showing `line` in the registered font at the size it was made at, coloured `ink`.
fn written(line: &str, families: &Fonts, ink: Color, change: impl FnOnce(&mut TextDef)) -> SkinObject {
    let mut def = TextDef { font: FONT_ID.to_string(), size: FONT_SIZE, constant_text: Some(line.to_string()), ..TextDef::default() };
    change(&mut def);
    let rect = SkinRect::new(ANCHOR_X as f32, STAGE.1 as f32 - TOP_ROW as f32 - BOX_HEIGHT, ROOM, BOX_HEIGHT);
    let color = SkinColor::rgba(ink.r, ink.g, ink.b, ink.a);
    let track = DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color, angle_deg: 0.0 }], ..DestinationTrack::default() };
    SkinObject { track, stretch: StretchKind::Stretch, body: Body::Text(text_body(&def, families)) }
}

/// Draws one object onto a cleared canvas `scale` times the size the document is authored at.
fn shot(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject) {
    canvas.clear(BACKDROP);
    assert!(paint_scaled(text, canvas, object, (STAGE.0 as f32, STAGE.1 as f32)), "the line is drawn");
}

/// The pixel `(right, down)` pixels from the one whose corner is the middle of the first `O` of a
/// line drawn `scale` times the size the font was made at.
fn from_middle(canvas: &CpuCanvas, scale: i32, right: i32, down: i32) -> Color {
    canvas.pixel_at((scale * (ANCHOR_X + DISC_MIDDLE) + right) as u32, (scale * (TOP_ROW + DISC_MIDDLE) + down) as u32)
}

/// How far the centre of that pixel is from a point `moved` pixels from the middle of the disc, in
/// texels of the fixture.
fn texels_from(moved: (i32, i32), scale: i32, right: i32, down: i32) -> f32 {
    ((right - moved.0) as f32 + 0.5).hypot((down - moved.1) as f32 + 0.5) / scale as f32
}

/// Holds every pixel within `reach` pixels of the middle of the disc to what `expected` says of it,
/// given how far it is from the middle in texels; `None` is a pixel on an edge, which is not held
/// to anything.
fn assert_rings(canvas: &CpuCanvas, scale: i32, reach: i32, what: &str, expected: impl Fn(f32, (i32, i32)) -> Option<Color>) {
    for down in -reach..reach {
        for right in -reach..reach {
            let out = texels_from((0, 0), scale, right, down);
            if let Some(color) = expected(out, (right, down)) {
                assert_eq!(from_middle(canvas, scale, right, down), color, "{what}: ({right}, {down}), {out} texels out");
            }
        }
    }
}

/// The fixture page is the field the tests reason about: cyan, with the distance to a disc and to a
/// bar in its alpha.
#[test]
fn the_fixture_page_holds_the_field_of_a_disc_and_of_a_bar() {
    let page = decode_png(&fixture_dir().join(PAGE_FILE)).expect("the fixture page decodes");
    assert_eq!((page.width, page.height), PAGE_SIZE);
    let held = |x: u32, y: u32| {
        let at = ((y * PAGE_SIZE.0 + x) * 4) as usize;
        assert_eq!(&page.rgba[at..at + 3], &[PAGE_CYAN.r, PAGE_CYAN.g, PAGE_CYAN.b], "the page is cyan at ({x}, {y})");
        page.rgba[at + 3]
    };
    let disc = |x: u32, y: u32| {
        let out = (x as f32 + 0.5 - 20.0).hypot(y as f32 + 0.5 - 20.0);
        ((DISTANCE_FIELD_EDGE + (DISC_RADIUS - out) / FIELD_RANGE_TEXELS).clamp(0.0, 1.0) * 255.0).round() as u8
    };
    for (x, y) in [(20, 20), (8, 20), (7, 20), (31, 20), (32, 20), (20, 4), (28, 28), (3, 3), (39, 39)] {
        assert_eq!(held(x, y), disc(x, y), "the disc's field at ({x}, {y})");
    }
    assert_eq!(
        (held(52, 20), held(47, 20), held(44, 20), held(52, 5)),
        (239, 112, 16, 48),
        "the bar: three and a half texels inside it, and half a texel, three and a half and two and a half outside"
    );
    assert_eq!((held(100, 20), held(20, 50)), (0, 0), "and nothing anywhere else");
}

/// The glyph is the text's colour inside its outline and nothing outside it: the page's own colour
/// is never read, only the distance in its alpha.
#[test]
fn a_distance_field_text_is_its_own_colour_inside_the_outline_and_nothing_outside() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);

    shot(&mut text, &mut canvas, &written("O", &families, GREEN, |_| {}));
    assert_rings(&canvas, 1, CELL_REACH, "a green O", |out, _| match out {
        out if out <= DISC_RADIUS - 1.0 => Some(GREEN),
        out if out >= DISC_RADIUS + 1.0 => Some(BACKDROP),
        _ => None,
    });

    shot(&mut text, &mut canvas, &written("I", &families, RED, |_| {}));
    let bar = (ANCHOR_X as u32 + 4, TOP_ROW as u32 + 4);
    assert_eq!(canvas.pixel_at(bar.0 + 1, bar.1 + 1), RED, "a texel inside the bar's corner");
    assert_eq!(canvas.pixel_at(bar.0 + 6, bar.1 + 22), RED, "and inside the corner opposite");
    assert_eq!(canvas.pixel_at(bar.0 - 2, bar.1 + 12), BACKDROP, "two texels left of it");
    assert_eq!(canvas.pixel_at(bar.0 + 9, bar.1 + 12), BACKDROP, "and two right of it");
}

/// Types 1 and 2 are drawn through the one path, and type 0 is not: a plain font shows its page's
/// coverage in the page's own colour.
#[test]
fn both_distance_field_types_are_drawn_alike_and_a_plain_font_is_not() {
    assert_eq!(BitmapKind::of(1), BitmapKind::of(2));
    let drawn = |font_type: i32| {
        let mut canvas = stage();
        let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::of(font_type));
        let object = written("OI", &families, WHITE, |def| {
            def.outline_width = 0.5;
            def.outline_color = "ff0000ff".to_string();
            (def.shadow_offset_x, def.shadow_offset_y) = (3.0, 3.0);
            def.shadow_color = "0000ffff".to_string();
        });
        shot(&mut text, &mut canvas, &object);
        canvas
    };
    let (plain, field, coloured) = (drawn(0), drawn(1), drawn(2));
    assert_eq!(field.pixels(), coloured.pixels(), "types 1 and 2 are the same pixels");
    assert_eq!(from_middle(&field, 1, 0, 0), WHITE);
    assert_eq!(from_middle(&plain, 1, 0, 0), PAGE_CYAN, "a plain font multiplies the page's colour by the text's");
    assert_ne!(field.pixels(), plain.pixels());
}

/// `outlineWidth` is halved and taken off the outline's own distance, so half a unit reaches a
/// quarter of the field's range -- two texels -- past the glyph, in `outlineColor`.
#[test]
fn an_outline_reaches_half_its_width_of_the_fields_range_past_the_glyph() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);
    let outlined = |width: f32, color: &str| {
        written("O", &families, BLACK, |def| {
            def.outline_width = width;
            def.outline_color = color.to_string();
        })
    };

    shot(&mut text, &mut canvas, &outlined(0.5, "ff0000ff"));
    assert_rings(&canvas, 1, CELL_REACH, "an outline half a unit wide", |out, _| match out {
        out if out <= DISC_RADIUS - 1.0 => Some(BLACK),
        out if (DISC_RADIUS + 0.75..=DISC_RADIUS + 1.25).contains(&out) => Some(RED),
        out if out >= DISC_RADIUS + 2.75 => Some(BACKDROP),
        _ => None,
    });

    shot(&mut text, &mut canvas, &outlined(0.5, "ffffff00"));
    assert_rings(&canvas, 1, CELL_REACH, "an outline of the colour a text has when it names none", |out, _| (out >= DISC_RADIUS + 1.0).then_some(BACKDROP));
}

/// However wide an outline is asked for, the distance it is drawn out to stops at a tenth of the
/// field's range: `Math.max(0.1f, 0.5f - outlineWidth / 2f)`.
#[test]
fn an_outline_is_never_drawn_out_past_a_tenth_of_the_fields_range() {
    let style = |width: f32| FieldInk::of(&TextDef { outline_width: width, ..TextDef::default() }).style((0.0, 0.0), (1.0, 1.0));
    assert_eq!(style(0.0).outline_distance, 0.5);
    assert_eq!(style(0.5).outline_distance, 0.25);
    assert_eq!(style(0.8).outline_distance, 0.1);
    assert_eq!(style(1.0).outline_distance, 0.1, "a whole unit would be the foot of the field");
    assert_eq!(style(40.0).outline_distance, 0.1);
    assert_eq!(style(-0.5).outline_distance, 0.75, "and a negative width draws the glyph in, which nothing stops");

    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);
    let mut drawn = |width: f32| {
        shot(
            &mut text,
            &mut canvas,
            &written("O", &families, BLACK, |def| {
                def.outline_width = width;
                def.outline_color = "ff0000ff".to_string();
            }),
        );
        canvas.pixels().to_vec()
    };
    let (wide, wider, moderate) = (drawn(1.0), drawn(40.0), drawn(0.5));
    assert_eq!(wide, wider);
    assert_ne!(wide, moderate);
}

/// What a text is handed to the shader with when it says nothing: no outline, an outline and a
/// shadow of transparent white, and a shadow that is not moved or smoothed.
#[test]
fn a_text_that_names_no_outline_and_no_shadow_hands_over_the_defaults() {
    assert_eq!(FieldInk::of(&TextDef::default()).style((0.0, 0.0), (128.0, 64.0)), DistanceFieldStyle::PLAIN);
}

/// The colours are read the way the reference's libGDX reads them: six digits are opaque, eight
/// carry an alpha, a `#` is skipped, any other length takes its first six digits and is opaque, and
/// text that cannot be read is opaque white -- not the transparent white a text that wrote nothing
/// has.
#[test]
fn an_outline_or_shadow_colour_is_read_the_way_the_reference_reads_it() {
    assert_eq!(written_color("ffffff00"), Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: 0 });
    assert_eq!(written_color("ff8000"), Color { r: u8::MAX, g: 128, b: 0, a: u8::MAX });
    assert_eq!(written_color("#0a141e28"), Color { r: 10, g: 20, b: 30, a: 40 });
    assert_eq!(written_color("0A141E28"), Color { r: 10, g: 20, b: 30, a: 40 });
    assert_eq!(written_color("0a141e2"), Color { r: 10, g: 20, b: 30, a: u8::MAX }, "seven digits are not eight");
    assert_eq!(written_color("0a141e2832"), Color { r: 10, g: 20, b: 30, a: u8::MAX }, "nor are ten");
    for unreadable in ["", "#", "ff80", "red", "gg0000ff", "ff0000zz", "\u{ff}\u{ff}\u{ff}\u{ff}"] {
        assert_eq!(written_color(unreadable), UNREADABLE_COLOR, "{unreadable:?}");
    }

    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);
    let object = written("O", &families, BLACK, |def| {
        def.outline_width = 0.5;
        def.outline_color = "no colour".to_string();
    });
    shot(&mut text, &mut canvas, &object);
    assert_eq!(from_middle(&canvas, 1, 9, 8), WHITE, "an outline whose colour cannot be read is white, and shows");
}

/// The shadow is drawn by the shader, in `shadowColor`, moved by `shadowOffsetX` and
/// `shadowOffsetY` right and down. There is no copy of the line at half its brightness, which is
/// what a plain font draws for a shadow. The offset is in texels of the page, so a text at twice
/// the font's size has its shadow twice as far.
#[test]
fn a_shadow_is_the_shadow_colour_moved_by_the_offset_in_the_pages_texels() {
    const MOVED: (i32, i32) = (6, 4);
    const CRESCENT_PIXELS: usize = 100;
    let shadowed = |families: &Fonts| {
        written("O", families, WHITE, |def| {
            (def.shadow_offset_x, def.shadow_offset_y) = (MOVED.0 as f32, MOVED.1 as f32);
            def.shadow_color = "0000ffff".to_string();
        })
    };
    for scale in [1, 2] {
        let mut canvas = CpuCanvas::new(STAGE.0 * scale as u32, STAGE.1 * scale as u32);
        let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);
        shot(&mut text, &mut canvas, &shadowed(&families));

        let moved = (MOVED.0 * scale, MOVED.1 * scale);
        let reach = CELL_REACH * scale;
        assert_rings(&canvas, scale, reach, "a shadowed O", |glyph, at| {
            let shadow = texels_from(moved, scale, at.0, at.1);
            match (glyph, shadow) {
                (glyph, _) if glyph <= DISC_RADIUS - 1.0 => Some(WHITE),
                (glyph, shadow) if glyph >= DISC_RADIUS + 1.0 && shadow <= DISC_RADIUS - 0.5 => Some(BLUE),
                (glyph, shadow) if glyph >= DISC_RADIUS + 1.0 && shadow >= DISC_RADIUS + 0.5 => Some(BACKDROP),
                _ => None,
            }
        });
        let shadow_only = (-reach..reach).flat_map(|down| (-reach..reach).map(move |right| (right, down)));
        let shadow_only = shadow_only.filter(|(right, down)| from_middle(&canvas, scale, *right, *down) == BLUE).count();
        assert!(
            shadow_only > CRESCENT_PIXELS * (scale * scale) as usize,
            "scale {scale}: a crescent of the shadow shows beside the glyph: {shadow_only} pixels"
        );
    }
}

/// `shadowSmoothness` is halved on its way to the shader, so a half is a quarter of the field's
/// range either side of the outline: two texels of the fixture.
#[test]
fn a_shadows_smoothness_is_halved_into_the_width_of_its_edge() {
    const SHADOW_DROP: i32 = 5;
    let style = |smoothness: f32| FieldInk::of(&TextDef { shadow_smoothness: smoothness, ..TextDef::default() }).style((6.0, -4.0), (128.0, 64.0));
    assert_eq!(style(0.5).shadow_smoothing, 0.25);
    assert_eq!(style(0.0).shadow_smoothing, 0.0);
    assert_eq!(style(0.0).shadow_offset, (6.0 / 128.0, -4.0 / 64.0), "the offset is a share of the page the font file states");

    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);
    let mut edge = |smoothness: f32| {
        let object = written("O", &families, WHITE, |def| {
            def.shadow_offset_y = SHADOW_DROP as f32;
            def.shadow_color = "0000ffff".to_string();
            def.shadow_smoothness = smoothness;
        });
        shot(&mut text, &mut canvas, &object);
        assert_eq!(from_middle(&canvas, 1, 0, DISC_RADIUS as i32 + 1), BLUE, "just below the glyph the shadow is whole");
        (DISC_RADIUS as i32 + 1..CELL_REACH).filter(|down| !matches!(from_middle(&canvas, 1, 0, *down), pixel if pixel == BLUE || pixel == BACKDROP)).count()
    };
    assert_eq!(edge(0.0), 0, "a shadow that is not smoothed has no pixel part way");
    assert_eq!(edge(0.5), 4, "one smoothed by a half fades over two texels either side of its outline");
}

/// A distance field text is blended the way the object before it was, like every text, and its
/// colour's alpha fades it -- squared, as the shader has it.
#[test]
fn a_distance_field_text_takes_the_blend_it_inherits_and_its_alpha_squared() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);

    text.leave_blend(BlendMode::Add);
    shot(&mut text, &mut canvas, &written("O", &families, Color { r: 50, g: 60, b: 70, a: u8::MAX }, |_| {}));
    assert_eq!(from_middle(&canvas, 1, 0, 0), Color { r: 150, g: 160, b: 170, a: u8::MAX }, "after an additive object the glyph adds to the backdrop");

    text.leave_blend(BlendMode::Alpha);
    shot(&mut text, &mut canvas, &written("O", &families, Color { a: 128, ..BLACK }, |_| {}));
    assert_eq!(
        from_middle(&canvas, 1, 0, 0),
        Color { r: 106, g: 106, b: 106, a: u8::MAX },
        "half transparent black comes out a quarter opaque and half white: (127 * 64 + 100 * 191) / 255 = 106"
    );
}

/// A character the font lacks is drawn from the text engine's fonts beside the glyphs, plainly: in
/// the text's colour, with no outline and no shadow of its own.
#[test]
fn a_character_the_font_lacks_is_drawn_plainly_beside_the_distance_field_glyphs() {
    let mut canvas = stage();
    let (mut text, families, _face, _pages) = engine(&mut canvas, BitmapKind::DistanceField);
    let object = written("OX", &families, WHITE, |def| {
        def.outline_width = 0.5;
        def.outline_color = "ff0000ff".to_string();
        (def.shadow_offset_x, def.shadow_offset_y) = (0.0, 3.0);
        def.shadow_color = "0000ffff".to_string();
    });
    shot(&mut text, &mut canvas, &object);

    let past_the_disc = (ANCHOR_X + 2 * DISC_MIDDLE + 4) as u32;
    let beside: Vec<Color> = (0..STAGE.1).flat_map(|y| (past_the_disc..STAGE.0).map(move |x| (x, y))).map(|(x, y)| canvas.pixel_at(x, y)).collect();
    assert!(beside.contains(&WHITE), "the stand-in has ink right of the disc");
    assert!(beside.iter().all(|pixel| pixel.r == pixel.g && pixel.g == pixel.b), "and it is grey all through: no red outline and no blue shadow");
    assert_eq!(from_middle(&canvas, 1, 0, 0), WHITE, "while the disc beside it is drawn as before");
    assert_eq!(from_middle(&canvas, 1, 9, 8), RED);
}

/// A host that reads what it is asked for off the disk.
struct Disk;

impl SkinAssets for Disk {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        decode_png(path)
    }
}

/// The whole way from a document: a font of type 1 and a text with an outline and a shadow are read
/// by the loader, compiled into a screen that warns of nothing, and drawn through the shader.
#[test]
fn a_document_with_a_distance_field_font_is_drawn_with_its_outline_and_its_shadow() {
    let root = std::env::temp_dir().join(format!("rbms-skin-field-font-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the scratch folder is writable");
    for file in [FONT_FILE, PAGE_FILE] {
        std::fs::copy(fixture_dir().join(file), root.join(file)).expect("the fixture is copied");
    }
    let (width, height) = STAGE;
    let bottom = height as i32 - TOP_ROW - BOX_HEIGHT as i32;
    let document = format!(
        r#"{{
            "type": 6, "name": "distance field font", "w": {width}, "h": {height},
            "font": [{{ "id": "{FONT_ID}", "path": "{FONT_FILE}", "type": 1 }}],
            "text": [{{
                "id": "title", "font": "{FONT_ID}", "size": {FONT_SIZE}, "constantText": "O",
                "outlineColor": "ff0000ff", "outlineWidth": 0.5,
                "shadowColor": "0000ffff", "shadowOffsetX": 6, "shadowOffsetY": 4, "shadowSmoothness": 0
            }}],
            "destination": [{{ "id": "title", "dst": [{{ "time": 0, "x": {ANCHOR_X}, "y": {bottom}, "w": {ROOM}, "h": {BOX_HEIGHT}, "r": 0, "g": 255, "b": 0 }}] }}]
        }}"#
    );
    let path = root.join("skin.json");
    std::fs::write(&path, document).expect("the scratch document is writable");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, Mode::BEAT_7K) };
    let skin = load_skin_with_host(&path, options, &MapHost::new()).expect("the scratch document loads");

    let mut canvas = stage();
    let mut text = TextContext::embedded_only();
    let mut screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut Disk);
    assert!(screen.warnings().is_empty(), "{:?}", screen.warnings());
    let mut draw = |screen: &SkinScreen, canvas: &mut CpuCanvas| {
        let (host, timers) = (MapHost::new(), TimerState::new());
        let frame = SkinFrame { now_us: 0, timers: &timers, state: &host, lua: None, mouse: None, data: FrameData::default() };
        canvas.clear(BACKDROP);
        screen.draw(&mut RenderCtx::new(Theme::default(), &mut text), canvas, &frame)
    };
    assert_eq!(draw(&screen, &mut canvas), 0, "the first frame waits for the page");
    assert_eq!(screen.load_font_pages(&mut canvas, &mut Disk, None), 1);
    assert_eq!(draw(&screen, &mut canvas), 1);

    assert_eq!(from_middle(&canvas, 1, 0, 0), GREEN, "the glyph in the destination's colour");
    assert_eq!(from_middle(&canvas, 1, 9, 8), RED, "the outline around it");
    assert_eq!(from_middle(&canvas, 1, 16, 4), BLUE, "the shadow right of and below it");
    assert_eq!(from_middle(&canvas, 1, -17, 4), BACKDROP, "and nothing on the other side");

    screen.release(&mut canvas);
    let _ = std::fs::remove_dir_all(&root);
}
