//! Tests of the `text` object as it reaches the screen: where its destination puts a line, how the
//! document's `size`, `overflow` and `wrapping` shape it, its shadow, which string it shows, the
//! blend it inherits and the texture it keeps.
//!
//! Every line is drawn onto a [`CpuCanvas`] through the dispatch a real frame goes through, in the
//! bundled face loaded the way a skin loads its own font, so what is asserted is the pixels a
//! screen would have shown.

use rbms_skin::dst::{DestinationTrack, Keyframe, SkinColor, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::TextDef;
use rbms_skin::property::generated::STRING_TITLE;
use rbms_skin::property::{MapHost, NameSpace, reference_implements};
use rbms_skin::timer::TimerState;

use super::{FontRef, Fonts, TEXT_PIXELS_PER_SCALE, text_body};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::draw::{ImageSelect, draw_object};
use crate::skin_render::object::{Body, ImageBody, SkinObject, Sprite};
use crate::skin_render::{FrameData, SkinFrame, SkinViewport};
use crate::{Color, CpuCanvas, Renderer, Theme};

/// The canvas every line is drawn on, which is also the size the document is authored at.
const STAGE: (u32, u32) = (640, 240);

/// What the canvas is cleared to: dark enough to tell ink from, bright enough to be darkened.
const BACKDROP: Color = Color { r: 100, g: 100, b: 100, a: u8::MAX };

/// The id the document registers its font under.
const FONT_ID: &str = "0";

/// The em size the test lines are drawn at.
const EM: i32 = 40;

/// The anchor the test lines are placed on.
const ANCHOR_X: f32 = 320.0;

/// How far above the document's foot the test lines' destination starts.
const FOOT: f32 = 100.0;

/// The room a line that is meant to fit is given.
const ROOMY: f32 = 300.0;

/// The room a line that is meant not to fit is given.
const NARROW: f32 = 120.0;

/// How far a measured edge may sit from the computed one: glyphs land on whole pixels.
const PIXEL: i32 = 1;

/// A line a good deal longer than [`NARROW`] at [`EM`].
const LONG_LINE: &str = "OVERFLOWING LINE OF TEXT";

/// A string id the reference has no property under.
const NOT_A_STRING_ID: i32 = 9_999;

/// The skin's blend value for additive drawing.
const BLEND_ADD: i32 = 2;

/// An engine holding only the bundled face, loaded once more the way a skin loads its own font,
/// and the font table a screen would have built from it.
fn engine() -> (TextContext, Vec<(String, FontRef)>) {
    let mut text = TextContext::embedded_only();
    let family = text.load_font(include_bytes!("../../../../../assets/fonts/Inter-Regular.ttf").to_vec()).expect("the bundled font loads as a skin font");
    (text, vec![(FONT_ID.to_string(), FontRef::Family(family))])
}

/// A text record in the registered font, generated at [`EM`].
fn record() -> TextDef {
    TextDef { font: FONT_ID.to_string(), size: EM, ..TextDef::default() }
}

/// A destination of `width` anchored on [`ANCHOR_X`], [`EM`] high, tinted `color`.
fn track(width: f32, color: SkinColor) -> DestinationTrack {
    let rect = SkinRect::new(ANCHOR_X, FOOT, width, EM as f32);
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

/// The text object `def` declares, in a destination of `width`, in white.
fn text_object(def: &TextDef, families: &Fonts, width: f32) -> SkinObject {
    let white = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX);
    SkinObject { track: track(width, white), stretch: StretchKind::Stretch, body: Body::Text(text_body(def, families)) }
}

/// A text object showing `line` written out.
fn constant(line: &str, families: &Fonts, width: f32, change: impl FnOnce(&mut TextDef)) -> SkinObject {
    let mut def = TextDef { constant_text: Some(line.to_string()), ..record() };
    change(&mut def);
    text_object(&def, families, width)
}

fn stage() -> CpuCanvas {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    canvas
}

/// Draws one object the way a frame does, answering whether anything reached the screen.
fn paint(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost) -> bool {
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: host, lua: None, mouse: None, data: FrameData::default() };
    let viewport = SkinViewport::new((STAGE.0 as f32, STAGE.1 as f32), (STAGE.0 as f32, STAGE.1 as f32));
    let mut ctx = RenderCtx::new(Theme::default(), text);
    draw_object(&mut ctx, canvas, object, &viewport, &frame)
}

/// Draws one object onto a fresh canvas and hands the canvas back.
fn shot(text: &mut TextContext, object: &SkinObject, host: &MapHost) -> CpuCanvas {
    let mut canvas = stage();
    assert!(paint(text, &mut canvas, object, host), "the line is drawn");
    canvas
}

/// The box around every pixel that is not the backdrop, as `(left, top, right, bottom)` with the
/// far edges exclusive.
fn ink(canvas: &CpuCanvas) -> Option<(i32, i32, i32, i32)> {
    let lit: Vec<(i32, i32)> = (0..STAGE.1)
        .flat_map(|y| (0..STAGE.0).map(move |x| (x, y)))
        .filter(|(x, y)| canvas.pixel_at(*x, *y) != BACKDROP)
        .map(|(x, y)| (x as i32, y as i32))
        .collect();
    let (xs, ys) = (lit.iter().map(|at| at.0), lit.iter().map(|at| at.1));
    Some((xs.clone().min()?, ys.clone().min()?, xs.max()? + 1, ys.max()? + 1))
}

/// The row a destination's top edge lands on.
fn top_row() -> i32 {
    STAGE.1 as i32 - (FOOT as i32 + EM)
}

#[test]
fn the_destination_x_is_where_a_line_starts_centres_or_ends() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let aligned = |text: &mut TextContext, align: i32| {
        let object = constant("ALIGN", &families, ROOMY, |def| def.align = align);
        ink(&shot(text, &object, &host)).expect("the word has ink")
    };

    let left = aligned(&mut text, 0);
    let centre = aligned(&mut text, 1);
    let right = aligned(&mut text, 2);
    let anchor = ANCHOR_X as i32;

    assert_eq!(left.0, anchor, "a left-aligned line starts on x itself, its first glyph's bearing taken off");
    assert!(((centre.0 + centre.2) / 2 - anchor).abs() <= PIXEL, "a centred line has x in its middle: {centre:?}");
    assert!((right.2 - anchor).abs() <= PIXEL, "a right-aligned line ends on x: {right:?}");
    assert_eq!(left.2 - left.0, right.2 - right.0, "and is the same line wherever it is put");
}

#[test]
fn the_destination_top_is_the_capital_line() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let capital = ink(&shot(&mut text, &constant("M", &families, ROOMY, |_| {}), &host)).expect("a capital has ink");
    let descending = ink(&shot(&mut text, &constant("Mg", &families, ROOMY, |_| {}), &host)).expect("a descender has ink");

    assert!((capital.1 - top_row()).abs() <= PIXEL, "the top of a capital sits on the destination's top edge: {capital:?}");
    assert!(capital.3 - capital.1 < EM, "and its foot, the baseline, well inside the box: {capital:?}");
    assert_eq!(descending.1, capital.1, "whatever else is on the line");
    assert!(descending.3 > capital.3, "a descender falls below the baseline: {descending:?}");
}

#[test]
fn the_destination_height_sizes_the_glyphs_and_the_documents_size_only_generates_them() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let sized = |text: &mut TextContext, size: i32| {
        let object = constant("M", &families, ROOMY, |def| def.size = size);
        ink(&shot(text, &object, &host)).expect("a capital has ink")
    };

    let native = sized(&mut text, EM);
    let halved = sized(&mut text, EM / 2);
    assert!(((halved.3 - halved.1) - (native.3 - native.1)).abs() <= PIXEL, "a font generated at half the size is scaled back up to the height: {halved:?}");
    assert!((halved.1 - native.1).abs() <= PIXEL, "and hangs from the same line");

    let mut canvas = stage();
    let sizeless = constant("M", &families, ROOMY, |def| def.size = 0);
    assert!(!paint(&mut text, &mut canvas, &sizeless, &host), "a font of no size cannot be generated, so its text is not drawn");
    assert_eq!(ink(&canvas), None);
}

#[test]
fn overflow_lets_a_long_line_run_on_squeezes_it_or_cuts_it() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let fitted = |text: &mut TextContext, overflow: i32, align: i32| {
        let object = constant(LONG_LINE, &families, NARROW, |def| {
            def.overflow = overflow;
            def.align = align;
        });
        ink(&shot(text, &object, &host)).expect("the line has ink")
    };
    let (anchor, room) = (ANCHOR_X as i32, NARROW as i32);

    let overflowing = fitted(&mut text, 0, 0);
    assert!(overflowing.2 - overflowing.0 > room, "overflow 0 runs past the destination: {overflowing:?}");

    let shrunk = fitted(&mut text, 1, 0);
    assert!((shrunk.0 - anchor).abs() <= PIXEL && (shrunk.2 - (anchor + room)).abs() <= PIXEL, "overflow 1 fills the destination exactly: {shrunk:?}");
    assert_eq!((shrunk.1, shrunk.3), (overflowing.1, overflowing.3), "and squeezes sideways only");

    let shrunk_right = fitted(&mut text, 1, 2);
    assert!(
        (shrunk_right.2 - anchor).abs() <= PIXEL && (shrunk_right.0 - (anchor - room)).abs() <= PIXEL,
        "to the left of x when right-aligned: {shrunk_right:?}"
    );

    let cut = fitted(&mut text, 2, 0);
    assert_eq!(cut.0, anchor);
    assert!(cut.2 - cut.0 <= room, "overflow 2 stops inside the destination: {cut:?}");
    assert!(cut.2 - cut.0 > room / 2, "after as many glyphs as fit");
}

#[test]
fn wrapping_breaks_a_long_line_and_wins_over_overflow() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let single = ink(&shot(&mut text, &constant("ONE", &families, NARROW, |_| {}), &host)).expect("one word has ink");
    let wrapped = constant("ONE TWO THREE FOUR", &families, NARROW, |def| {
        def.wrapping = true;
        def.overflow = 1;
    });
    let block = ink(&shot(&mut text, &wrapped, &host)).expect("the wrapped lines have ink");

    assert!((block.1 - top_row()).abs() <= PIXEL, "the first line still hangs from the destination's top: {block:?}");
    assert!(block.3 - block.1 > 2 * (single.3 - single.1), "the rest stack below it: {block:?}");
    assert!(block.2 - block.0 <= NARROW as i32, "no line is longer than the destination");
    assert!(block.2 - block.0 > single.2 - single.0, "and the lines are not squeezed, whatever overflow says");
}

#[test]
fn a_shadow_is_the_line_at_half_brightness_moved_right_and_down() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let plain = ink(&shot(&mut text, &constant("SHADE", &families, ROOMY, |_| {}), &host)).expect("the word has ink");
    let shaded = |text: &mut TextContext, offset: (f32, f32)| {
        let mut object = constant("SHADE", &families, ROOMY, |def| (def.shadow_offset_x, def.shadow_offset_y) = offset);
        object.track = track(ROOMY, SkinColor::rgba(240, 120, 60, u8::MAX));
        shot(text, &object, &host)
    };

    let below = shaded(&mut text, (6.0, 4.0));
    let bounds = ink(&below).expect("the shaded word has ink");
    assert_eq!((bounds.0, bounds.1), (plain.0, plain.1), "the line itself has not moved");
    assert_eq!((bounds.2, bounds.3), (plain.2 + 6, plain.3 + 4), "a positive offset puts the shadow to the right and below");
    let pixels: Vec<Color> = (0..STAGE.1).flat_map(|y| (0..STAGE.0).map(move |x| (x, y))).map(|(x, y)| below.pixel_at(x, y)).collect();
    assert!(pixels.contains(&Color { r: 240, g: 120, b: 60, a: u8::MAX }), "the line is drawn in its own colour");
    assert!(pixels.contains(&Color { r: 120, g: 60, b: 30, a: u8::MAX }), "and its shadow in half of each channel, as opaque as the line");

    let above = ink(&shaded(&mut text, (-6.0, -4.0))).expect("the shaded word has ink");
    assert_eq!((above.0, above.1), (plain.0 - 6, plain.1 - 4), "a negative offset puts it to the left and above");
    assert_eq!((above.2, above.3), (plain.2, plain.3));
}

#[test]
fn a_property_the_reference_reads_silences_the_written_out_text() {
    let (mut text, families) = engine();
    assert!(reference_implements(NameSpace::Text, STRING_TITLE) && !reference_implements(NameSpace::Text, NOT_A_STRING_ID));
    let mut host = MapHost::new();
    host.texts.insert(STRING_TITLE, "TITLE".to_string());
    host.texts.insert(NOT_A_STRING_ID, "UNREAD".to_string());
    let written = shot(&mut text, &constant("WRITTEN OUT", &families, ROOMY, |_| {}), &host);
    let read = shot(&mut text, &text_object(&TextDef { reference: STRING_TITLE, ..record() }, &families, ROOMY), &host);
    assert!(written.pixels() != read.pixels(), "the two strings draw differently");

    let both = constant("WRITTEN OUT", &families, ROOMY, |def| def.reference = STRING_TITLE);
    assert!(shot(&mut text, &both, &host).pixels() == read.pixels(), "with a property to read, the written out text is ignored");

    let silent = MapHost::new();
    let mut canvas = stage();
    assert!(!paint(&mut text, &mut canvas, &both, &silent), "even when the property reads empty: nothing is drawn rather than the written out text");

    let unknown = constant("WRITTEN OUT", &families, ROOMY, |def| def.reference = NOT_A_STRING_ID);
    assert!(shot(&mut text, &unknown, &host).pixels() == written.pixels(), "an id the reference has no string under is no property, so the text stands");
}

#[test]
fn a_text_is_blended_the_way_the_object_before_it_was() {
    let (mut text, families) = engine();
    let host = MapHost::new();
    let mut canvas = stage();
    let tex = canvas.register_texture("dot", &[0, 0, 0, u8::MAX], 1, 1);
    let dot = |blend: i32| {
        let sprite = Sprite { tex, size: (1, 1), origin: (0, 0), cell: (1, 1), columns: 1, rows: 1, timer: None, cycle: 0 };
        let mut track = track(1.0, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX));
        track.blend = blend;
        SkinObject { track, stretch: StretchKind::Stretch, body: Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First }) }
    };
    let dim = |blend: i32| {
        let mut object = constant("BLEND", &families, ROOMY, |_| {});
        object.track = track(ROOMY, SkinColor::rgba(50, 50, 50, u8::MAX));
        object.track.blend = blend;
        object
    };
    let brightest = |canvas: &CpuCanvas| (0..STAGE.1).flat_map(|y| (0..STAGE.0).map(move |x| canvas.pixel_at(x, y).r)).max().unwrap_or_default();
    let darkest = |canvas: &CpuCanvas| (0..STAGE.1).flat_map(|y| (0..STAGE.0).map(move |x| canvas.pixel_at(x, y).r)).min().unwrap_or_default();

    assert!(paint(&mut text, &mut canvas, &dot(BLEND_ADD), &host));
    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &dim(0), &host));
    assert_eq!((darkest(&canvas), brightest(&canvas)), (BACKDROP.r, BACKDROP.r + 50), "after an additive image the ink is added to what is under it");

    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &dim(0), &host));
    assert_eq!(brightest(&canvas), BACKDROP.r + 50, "a text sets nothing, so the next text is added as well");

    assert!(paint(&mut text, &mut canvas, &dot(0), &host));
    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &dim(BLEND_ADD), &host));
    assert_eq!((darkest(&canvas), brightest(&canvas)), (50, BACKDROP.r), "after a plain image it covers, whatever blend its own destination asks for");
}

#[test]
fn a_text_keeps_one_texture_and_hands_it_back() {
    let (mut text, families) = engine();
    let mut host = MapHost::new();
    host.texts.insert(STRING_TITLE, "FIRST".to_string());
    let object = text_object(&TextDef { reference: STRING_TITLE, ..record() }, &families, ROOMY);
    let mut canvas = stage();

    assert!(paint(&mut text, &mut canvas, &object, &host));
    assert!(paint(&mut text, &mut canvas, &object, &host));
    assert_eq!(canvas.live_texture_count(), 1, "the same line drawn again reuses its texture");
    let first = ink(&canvas);

    host.texts.insert(STRING_TITLE, "A SECOND, LONGER LINE".to_string());
    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &object, &host));
    assert_eq!(canvas.live_texture_count(), 1, "a new string replaces what the texture holds");
    assert_ne!(ink(&canvas), first, "and is what is drawn");

    object.release(&mut canvas);
    assert_eq!(canvas.live_texture_count(), 0, "releasing the object hands the texture back");
    canvas.clear(BACKDROP);
    assert!(paint(&mut text, &mut canvas, &object, &host), "and the object composes its line again if it is drawn after that");
    assert_eq!(canvas.live_texture_count(), 1);
}

#[test]
fn a_text_whose_font_did_not_load_keeps_its_stand_in() {
    let (mut text, _) = engine();
    let host = MapHost::new();
    let object = constant("STAND IN", &[], ROOMY, |def| def.shadow_offset_x = 4.0);
    let drawn = shot(&mut text, &object, &host);

    let mut expected = stage();
    text.reset_family();
    text.draw_text(&mut expected, ANCHOR_X, top_row() as f32, EM as f32 / TEXT_PIXELS_PER_SCALE, Color::rgb(u8::MAX, u8::MAX, u8::MAX), "STAND IN");
    assert_eq!(ink(&drawn), ink(&expected), "the default face at the destination's height, from its top left corner");
    assert!(drawn.pixels() == expected.pixels(), "pixel for pixel, with no shadow");
    assert_eq!(drawn.live_texture_count(), 0, "drawn as fills, with no texture of its own");
}
