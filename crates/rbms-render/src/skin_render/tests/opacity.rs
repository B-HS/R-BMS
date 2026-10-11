//! How the basic objects take the alpha a player's offset leaves them: an image, a reference image,
//! a whole number, a fractional one, a slider's handle and a graph's bar.
//!
//! Each is drawn three times through the dispatch a real frame goes through, by a destination that
//! names one offset: with the offset at nothing, where the object is opaque; with it taking half
//! of the alpha away, where the canvas shows through by half; and with it taking all of it, where
//! the object is left out of the frame and the canvas is as it was.

use rbms_skin::dst::SkinOffset;

use super::*;

/// The offset every object of these tests names.
const PART: i32 = 9_001;

/// The alpha an opaque object is left with once [`HALF_SHIFT`] is added to it.
const HALF_ALPHA: u8 = 127;

/// The shift that takes half of an opaque object's alpha away, and the one that takes all of it.
const HALF_SHIFT: f32 = -128.0;
const WHOLE_SHIFT: f32 = -255.0;

/// A shift past the whole range either way, which is held at its end.
const PAST_THE_RANGE: f32 = 1_000.0;

/// `object` with its destination naming the offset.
fn part(mut object: SkinObject) -> SkinObject {
    object.track.offsets.push(PART);
    object
}

/// `host` answering the offset with an alpha shift of `shift`.
fn shifted(mut host: MapHost, shift: f32) -> MapHost {
    host.offsets.insert(PART, SkinOffset { a: shift, ..SkinOffset::default() });
    host
}

/// What the canvas shows where `ink` is drawn over the backdrop at `alpha`, by the canvas's own
/// source-over arithmetic.
fn seen_through(ink: (u8, u8, u8), alpha: u8) -> (u8, u8, u8) {
    let mix = |over: u8, under: u8| ((u32::from(over) * u32::from(alpha) + u32::from(under) * u32::from(u8::MAX - alpha)) / u32::from(u8::MAX)) as u8;
    (mix(ink.0, BACKDROP.0), mix(ink.1, BACKDROP.1), mix(ink.2, BACKDROP.2))
}

/// Draws `object` at the three shifts and holds what is at `at`, where it draws `ink`, against
/// what each should leave there.
fn assert_fades(canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost, at: (u32, u32), ink: (u8, u8, u8), what: &str) {
    wipe(canvas);
    assert!(paint_plain(canvas, object, &shifted(host.clone(), 0.0)), "{what}: nothing was drawn with the offset at nothing");
    assert_eq!(rgb(canvas, at.0, at.1), ink, "{what}: an offset of nothing leaves it opaque");

    wipe(canvas);
    assert!(paint_plain(canvas, object, &shifted(host.clone(), HALF_SHIFT)), "{what}: nothing was drawn at half its alpha");
    assert_eq!(rgb(canvas, at.0, at.1), seen_through(ink, HALF_ALPHA), "{what}: half the alpha shows the canvas through by half");

    for gone in [WHOLE_SHIFT, -PAST_THE_RANGE] {
        wipe(canvas);
        assert!(!paint_plain(canvas, object, &shifted(host.clone(), gone)), "{what}: an object with no alpha left is drawn (shift {gone})");
        assert_eq!(left_edge(canvas, at.1), None, "{what}: and something of it reached the canvas (shift {gone})");
    }

    wipe(canvas);
    assert!(paint_plain(canvas, object, &shifted(host.clone(), PAST_THE_RANGE)), "{what}: nothing was drawn with the alpha pushed past opaque");
    assert_eq!(rgb(canvas, at.0, at.1), ink, "{what}: an alpha pushed past opaque is opaque and no more");
}

#[test]
fn an_image_fades_with_the_offset_its_destination_names() {
    let mut canvas = stage();
    let sprite = swatch(&mut canvas, "plain", 1, 1, &[RED]);
    let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First });
    let image = part(object_at(SkinRect::new(24.0, 8.0, 16.0, 8.0), body));
    assert_fades(&mut canvas, &image, &MapHost::new(), (30, 36), RED, "image");
}

/// An image whose own keyframe is already half transparent is shifted from there: the offset is
/// added to the alpha the document wrote, not multiplied into it.
#[test]
fn an_offset_is_added_to_the_alpha_the_document_wrote() {
    const WRITTEN: u8 = 150;
    const SHIFT: f32 = -50.0;
    let mut canvas = stage();
    let sprite = swatch(&mut canvas, "plain", 1, 1, &[RED]);
    let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First });
    let mut image = part(object_at(SkinRect::new(24.0, 8.0, 16.0, 8.0), body));
    image.track = DestinationTrack { offsets: vec![PART], ..held(SkinRect::new(24.0, 8.0, 16.0, 8.0), SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, WRITTEN)) };

    assert!(paint_plain(&mut canvas, &image, &shifted(MapHost::new(), SHIFT)));
    assert_eq!(rgb(&canvas, 30, 36), seen_through(RED, WRITTEN - 50), "150 less 50 is an alpha of 100");

    wipe(&mut canvas);
    assert!(!paint_plain(&mut canvas, &image, &shifted(MapHost::new(), -f32::from(WRITTEN))), "an offset as large as the alpha takes all of it");
    assert_eq!(left_edge(&canvas, 36), None);
}

/// An object that names no offset is not touched by one, whatever the host answers under any id.
#[test]
fn an_object_that_names_no_offset_is_left_as_it_is() {
    let mut canvas = stage();
    let sprite = swatch(&mut canvas, "plain", 1, 1, &[RED]);
    let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First });
    let image = object_at(SkinRect::new(24.0, 8.0, 16.0, 8.0), body);
    assert!(paint_plain(&mut canvas, &image, &shifted(MapHost::new(), WHOLE_SHIFT)));
    assert_eq!(rgb(&canvas, 30, 36), RED);
}

#[test]
fn a_reference_image_fades_with_the_offset_its_destination_names() {
    let mut canvas = stage();
    let tex = swatch_texture(&mut canvas, "stagefile", 1, 1, &[GREEN]);
    let images = || ReferenceImages { stagefile: Some(tex), ..ReferenceImages::default() };
    let picture = part(reference_at(SkinRect::new(24.0, 8.0, 16.0, 8.0), "-100"));
    let paint_at = |canvas: &mut CpuCanvas, shift: f32| paint(canvas, &picture, &shifted(MapHost::new(), shift), STAGE, images());

    assert!(paint_at(&mut canvas, 0.0), "the chart's picture is not drawn at all");
    assert_eq!(rgb(&canvas, 30, 36), GREEN);
    wipe(&mut canvas);
    assert!(paint_at(&mut canvas, HALF_SHIFT));
    assert_eq!(rgb(&canvas, 30, 36), seen_through(GREEN, HALF_ALPHA));
    wipe(&mut canvas);
    assert!(!paint_at(&mut canvas, WHOLE_SHIFT));
    assert_eq!(left_edge(&canvas, 36), None);
}

#[test]
fn a_number_fades_with_the_offset_its_destination_names() {
    let mut canvas = stage();
    let mut host = MapHost::new();
    host.integers.insert(NUMBER_PLAYLEVEL, 7);
    let number = part(object_at(SkinRect::new(8.0, 8.0, 8.0, 8.0), Body::Number(level_number(&mut canvas, 1, 0, ValueSource::Id(NUMBER_PLAYLEVEL)))));
    assert_fades(&mut canvas, &number, &host, (12, 36), glyph(7), "number");
}

#[test]
fn a_fractional_number_fades_with_the_offset_its_destination_names() {
    let mut canvas = stage();
    let mut host = MapHost::new();
    host.floats.insert(FLOAT_HISPEED, 3.0);
    let fraction = part(object_at(SkinRect::new(8.0, 8.0, 8.0, 8.0), Body::Float(speed_fraction(&mut canvas, 1, 0, ValueSource::Id(FLOAT_HISPEED)))));
    assert_fades(&mut canvas, &fraction, &host, (12, 36), glyph(3), "float");
}

#[test]
fn a_sliders_handle_fades_with_the_offset_its_destination_names() {
    let mut canvas = stage();
    let mut host = MapHost::new();
    host.rates.insert(RATE_MUSICSELECT_POSITION, 0.0);
    let slider = part(object_at(SkinRect::new(20.0, 8.0, 4.0, 8.0), Body::Slider(handle(&mut canvas, 1, 10.0, None))));
    assert_fades(&mut canvas, &slider, &host, (21, 36), RED, "slider");
}

#[test]
fn a_graphs_bar_fades_with_the_offset_its_destination_names() {
    let mut canvas = stage();
    let mut host = MapHost::new();
    host.rates.insert(RATE_MUSICSELECT_POSITION, 1.0);
    let graph = part(object_at(SkinRect::new(8.0, 8.0, 16.0, 8.0), Body::Graph(bar(&mut canvas, 0))));
    assert_fades(&mut canvas, &graph, &host, (8, 36), RED, "graph");
}

/// A fade whose keyframes carry different alphas is shifted on every frame of it, so an offset that
/// takes everything away leaves nothing of it between the keyframes either.
#[test]
fn an_image_that_fades_out_is_shifted_between_its_keyframes() {
    const FADE_MS: i64 = 1_000;
    let rect = SkinRect::new(24.0, 8.0, 16.0, 8.0);
    let opaque = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX);
    let clear = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 0);
    let mut canvas = stage();
    let sprite = swatch(&mut canvas, "plain", 1, 1, &[RED]);
    let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First });
    let mut image = part(object_at(rect, body));
    image.track.frames = vec![
        Keyframe { time_ms: 0, rect, clip: None, color: opaque, angle_deg: 0.0 },
        Keyframe { time_ms: FADE_MS, rect, clip: None, color: clear, angle_deg: 0.0 },
    ];

    let mut at = |shift: f32, now_ms: i64| {
        let mut host = shifted(MapHost::new(), shift);
        host.now_us = now_ms * MICROS_PER_MILLI;
        wipe(&mut canvas);
        (paint_plain(&mut canvas, &image, &host), rgb(&canvas, 30, 36))
    };
    assert_eq!(at(0.0, FADE_MS / 2), (true, seen_through(RED, 128)), "halfway through its own fade it is at half");
    assert_eq!(at(HALF_SHIFT, FADE_MS / 2), (false, BACKDROP), "128 less 128 is nothing: the shift is applied halfway through the fade");
    assert_eq!(at(HALF_SHIFT, FADE_MS / 4), (true, seen_through(RED, 63)), "and a quarter of the way through, where 191 less 128 is left");
    for now_ms in [0, 1, FADE_MS / 3, FADE_MS / 2, FADE_MS - 1] {
        assert_eq!(at(WHOLE_SHIFT, now_ms), (false, BACKDROP), "{now_ms} ms into the fade");
    }
}
