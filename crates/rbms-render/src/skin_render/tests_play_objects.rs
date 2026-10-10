//! Unit tests for the play screen's own document objects: the gauge, the judgement pop-up and the
//! lane covers. The note field's are beside the note field, in `notes/tests.rs`.
//!
//! Each one is drawn onto a [`CpuCanvas`] through the very dispatch a real frame goes through, so
//! what is asserted is the pixel a screen would have shown. The sources are one-pixel textures of a
//! colour per part, which is what lets a test name the cell that reached the screen rather than
//! recognising a picture.

use std::borrow::Cow;
use std::cell::RefCell;

use rbms_skin::dst::{DestinationTrack, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinOffset, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::property::generated::FLOAT_GROOVEGAUGE_1P;
use rbms_skin::property::{SkinHost, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState};

use super::draw::draw_object;
use super::gauge::{GAUGE_TYPES, GaugeAnimation, GaugeBody, GaugeMotion, GaugeScale, SLOT_BELOW_BORDER, SLOT_LEADING, SLOT_LIT, SLOT_UNLIT, SLOTS_PER_GAUGE};
use super::object::{Body, SkinObject, Sprite};
use super::{FrameData, GaugeFrame, SkinFrame, SkinViewport};
use crate::ctx::with_render_ctx;
use crate::{Color, CpuCanvas, Renderer, TextureId};

/// The canvas every test draws on, which is also the size the documents are authored at so the
/// viewport maps one to one and a document pixel is a screen pixel.
const CANVAS: (u32, u32) = (1280, 720);

/// The gauge the DoD names: not quite three quarters full.
const TEST_GAUGE: f32 = 0.74;

/// The percent of a full gauge the gauges of these tests clear at.
const TEST_GAUGE_BORDER: f32 = 80.0;

/// How many parts that gauge is cut into.
const TEST_PARTS: i32 = 50;

/// A colour that cannot be confused with a background or another part.
fn shade(value: u8) -> Color {
    Color { r: value, g: 0, b: 0, a: 255 }
}

/// A one-cell sprite of one flat colour.
fn solid(canvas: &mut CpuCanvas, key: &str, color: Color) -> Sprite {
    let tex = canvas.register_texture(key, &[color.r, color.g, color.b, color.a], 1, 1);
    strip(tex, 1)
}

/// A sprite over a texture already registered, cut into `cells` columns of one row.
fn strip(tex: TextureId, cells: u32) -> Sprite {
    Sprite { tex, size: (cells, 1), origin: (0, 0), cell: (1, 1), columns: cells, rows: 1, timer: None, cycle: 0 }
}

/// A strip of ten digit cells, each a colour of its own so a drawn place names its digit.
fn digits(canvas: &mut CpuCanvas) -> Sprite {
    let pixels: Vec<u8> = (0..10).flat_map(|digit| [digit_shade(digit), 0, 0, 255]).collect();
    let tex = canvas.register_texture("digits", &pixels, 10, 1);
    strip(tex, 10)
}

/// The red channel the digit `value` is drawn in.
fn digit_shade(value: u8) -> u8 {
    20 + value * 20
}

/// A destination that holds one rectangle still, in one colour.
fn held(rect: SkinRect, color: SkinColor) -> DestinationTrack {
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

/// A destination that holds one rectangle still, fully opaque.
fn still(rect: SkinRect) -> DestinationTrack {
    held(rect, SkinColor::rgba(255, 255, 255, 255))
}

/// One draw-list entry over a body and the rectangle its destination holds.
fn object(rect: SkinRect, body: Body) -> SkinObject {
    SkinObject { track: still(rect), stretch: StretchKind::from_id(-1), body }
}

/// The game state the play objects read from a host: the share of a full gauge, and nothing else.
#[derive(Debug, Default, Clone, Copy)]
struct PlayState {
    gauge: f32,
}

impl OffsetSource for PlayState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for PlayState {
    fn boolean(&self, _id: i32) -> Option<bool> {
        Some(false)
    }
}

impl SkinHost for PlayState {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, id: i32) -> f32 {
        if id == FLOAT_GROOVEGAUGE_1P { self.gauge } else { UNMAPPED_FLOAT }
    }

    fn text(&self, _id: i32) -> Cow<'_, str> {
        Cow::Borrowed(UNMAPPED_STRING)
    }

    fn timer_us(&self, _id: i32) -> i64 {
        TIMER_OFF
    }

    fn now_us(&self) -> i64 {
        0
    }
}

/// What a play frame carries for a gauge object: the gauge the run is played on.
fn playing(gauge_kind: usize) -> FrameData<'static> {
    FrameData { gauge: Some(GaugeFrame::of_kind(gauge_kind, TEST_GAUGE_BORDER)), ..FrameData::default() }
}

/// Draws one object on a canvas that already holds its textures.
fn draw_on(canvas: &mut CpuCanvas, object: &SkinObject, state: &PlayState, data: FrameData<'_>) -> bool {
    draw_at(canvas, object, state, data, 0)
}

/// The same, `now_ms` milliseconds into the scene.
fn draw_at(canvas: &mut CpuCanvas, object: &SkinObject, state: &PlayState, data: FrameData<'_>, now_ms: i64) -> bool {
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers: &timers, state, lua: None, mouse: None, data };
    let viewport = SkinViewport::new((CANVAS.0 as f32, CANVAS.1 as f32), (CANVAS.0 as f32, CANVAS.1 as f32));
    with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
}

/// The first row of `column` holding `color`, scanning down from the top of the canvas.
fn first_row_of(canvas: &CpuCanvas, column: u32, color: Color) -> Option<u32> {
    (0..CANVAS.1).find(|row| canvas.pixel_at(column, *row) == color)
}

/// The last such row.
fn last_row_of(canvas: &CpuCanvas, column: u32, color: Color) -> Option<u32> {
    (0..CANVAS.1).rev().find(|row| canvas.pixel_at(column, *row) == color)
}

/// The seven-key groove gauge of the reference: never below two, full at a hundred, cleared at
/// eighty.
const GROOVE: GaugeScale = GaugeScale::new(2.0, 100.0, 80.0);

/// How many parts the tests that read every part of a gauge cut it into.
const FEW_PARTS: i32 = 10;

/// How long the flickering test gauge takes over one fade up and down, in milliseconds.
const FLICKER_CYCLE_MS: i32 = 100;

/// The red a flickering test gauge's lit, unlit and leading cells are drawn in, far enough apart
/// that a fade between two of them cannot be taken for either.
const FLICKER_LIT: u8 = 40;
const FLICKER_UNLIT: u8 = 10;
const FLICKER_LEADING: u8 = 240;

/// A gauge body over `nodes` and the table that spreads them, with the record's default times.
fn gauge_of(nodes: Vec<Sprite>, slots: [Option<u8>; 36], parts: i32, animation: GaugeAnimation) -> GaugeBody {
    GaugeBody { nodes, slots, animation: Some(animation), range: 0, cycle: 0, starttime: 0, endtime: 500, motion: RefCell::new(GaugeMotion::new(parts)) }
}

/// What a frame carries when the screen knows the gauge outright: its type, its value and its
/// limits, the same for every type.
fn gauged(gauge: GaugeFrame) -> FrameData<'static> {
    FrameData { gauge: Some(gauge), ..FrameData::default() }
}

/// A gauge over a table of thirty-six distinct nodes, so a drawn part names the cell it read.
fn distinct_gauge(canvas: &mut CpuCanvas) -> GaugeBody {
    let nodes: Vec<Sprite> = (0..36).map(|slot| solid(canvas, &format!("node{slot}"), shade(slot as u8 + 1))).collect();
    let mut slots = [None; 36];
    for (slot, entry) in slots.iter_mut().enumerate() {
        *entry = Some(slot as u8);
    }
    gauge_of(nodes, slots, TEST_PARTS, GaugeAnimation::Random)
}

/// The part of a gauge drawn `part` places along a bar of `rect`, as a canvas column.
fn part_column(rect: SkinRect, part: i32) -> u32 {
    (rect.x + rect.w * (part as f32 - 0.5) / TEST_PARTS as f32) as u32
}

#[test]
fn a_gauge_lights_one_part_for_every_part_of_its_value() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let bar = object(rect, Body::Gauge(distinct_gauge(&mut canvas)));
    let state = PlayState { gauge: TEST_GAUGE };
    assert!(draw_on(&mut canvas, &bar, &state, playing(0)), "the gauge drew nothing at all");

    let row = (CANVAS.1 as f32 - (rect.y + rect.h / 2.0)) as u32;
    let cell = |part: i32| usize::from(canvas.pixel_at(part_column(rect, part), row).r).saturating_sub(1) % SLOTS_PER_GAUGE;
    let unlit = [SLOT_UNLIT, SLOT_UNLIT + SLOT_BELOW_BORDER];
    let lit = (1..=TEST_PARTS).filter(|part| !unlit.contains(&cell(*part))).count();
    assert_eq!(lit, 37, "a gauge at {TEST_GAUGE} of fifty parts lights thirty-seven of them");
    let leading = [SLOT_LEADING, SLOT_LEADING + SLOT_BELOW_BORDER];
    assert!(leading.contains(&cell(37)), "the thirty-seventh part is the leading one");
}

#[test]
fn a_gauge_reads_a_column_of_its_own_for_every_kind_of_gauge() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let row = (CANVAS.1 as f32 - (rect.y + rect.h / 2.0)) as u32;
    let state = PlayState { gauge: TEST_GAUGE };

    let mut seen = Vec::new();
    for kind in 0..6 {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let bar = object(rect, Body::Gauge(distinct_gauge(&mut canvas)));
        draw_on(&mut canvas, &bar, &state, playing(kind));
        seen.push(canvas.pixel_at(part_column(rect, 1), row).r);
    }
    seen.dedup();
    assert_eq!(seen.len(), 6, "each kind of gauge should read a column of the table of its own");
}

/// The cell each part of a ten-part gauge read, by the red of the pixel in the middle of it.
fn cells_read(canvas: &CpuCanvas, rect: SkinRect) -> Vec<usize> {
    let row = (CANVAS.1 as f32 - (rect.y + rect.h / 2.0)) as u32;
    (1..=FEW_PARTS).map(|part| usize::from(canvas.pixel_at((rect.x + rect.w * (part as f32 - 0.5) / FEW_PARTS as f32) as u32, row).r).wrapping_sub(1)).collect()
}

/// Every gauge type reads the six cells the reference gives it, and every part of it the cell its
/// place asks for: lit, leading or unlit, in the shade for the side of the clear line it is on.
#[test]
fn a_gauge_reads_the_cell_the_reference_names_for_every_gauge_type_and_state() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let columns = [0, 1, 2, 3, 4, 5, 3, 4, 5];
    let below = SLOT_BELOW_BORDER;
    for (gauge_type, column) in columns.into_iter().enumerate() {
        let first = column * SLOTS_PER_GAUGE;
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let mut body = distinct_gauge(&mut canvas);
        body.motion = RefCell::new(GaugeMotion::new(FEW_PARTS));
        let bar = object(rect, Body::Gauge(body));

        let half = GaugeFrame::playing(gauge_type, 50.0, [GROOVE; GAUGE_TYPES]);
        assert!(draw_on(&mut canvas, &bar, &PlayState::default(), gauged(half)), "gauge type {gauge_type} drew nothing");
        let lit = first + SLOT_LIT + below;
        let dark = first + SLOT_UNLIT;
        let expected = vec![lit, lit, lit, lit, first + SLOT_LEADING + below, dark + below, dark + below, dark, dark, dark];
        assert_eq!(cells_read(&canvas, rect), expected, "gauge type {gauge_type} at half");

        canvas.clear(Color::BLACK);
        let full = GaugeFrame::playing(gauge_type, 100.0, [GROOVE; GAUGE_TYPES]);
        draw_on(&mut canvas, &bar, &PlayState::default(), gauged(full));
        let bright = first + SLOT_LIT;
        let expected = vec![lit, lit, lit, lit, lit, lit, lit, bright, bright, first + SLOT_LEADING];
        assert_eq!(cells_read(&canvas, rect), expected, "gauge type {gauge_type} full");
    }
}

/// How many parts of a fifty-part gauge are lit on the canvas.
fn lit_count(canvas: &CpuCanvas, rect: SkinRect) -> usize {
    let row = (CANVAS.1 as f32 - (rect.y + rect.h / 2.0)) as u32;
    let unlit = [SLOT_UNLIT, SLOT_UNLIT + SLOT_BELOW_BORDER];
    (1..=TEST_PARTS).filter(|part| !unlit.contains(&(usize::from(canvas.pixel_at(part_column(rect, *part), row).r).wrapping_sub(1) % SLOTS_PER_GAUGE))).count()
}

/// On a score screen the gauge fills from its least value to the one the run ended on, at the pace
/// that would fill the whole bar between the record's two times.
#[test]
fn a_score_screens_gauge_fills_up_to_the_runs_last_value_as_the_scene_opens() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let bar = object(rect, Body::Gauge(distinct_gauge(&mut canvas)));
    let ended = gauged(GaugeFrame::finished(2, 90.0, [GROOVE; GAUGE_TYPES]));

    for (now_ms, lit) in [(0, 1), (125, 12), (250, 25), (449, 44), (450, 45), (500, 45), (5_000, 45)] {
        canvas.clear(Color::BLACK);
        assert!(draw_at(&mut canvas, &bar, &PlayState::default(), ended, now_ms));
        assert_eq!(lit_count(&canvas, rect), lit, "{now_ms} ms into the scene");
    }

    canvas.clear(Color::BLACK);
    let running = gauged(GaugeFrame::playing(2, 90.0, [GROOVE; GAUGE_TYPES]));
    draw_at(&mut canvas, &bar, &PlayState::default(), running, 0);
    assert_eq!(lit_count(&canvas, rect), 45, "a run in progress shows its gauge as it is");
}

/// A record can put the fill later in the scene, and until then the gauge stands at its least.
#[test]
fn a_score_screens_gauge_waits_for_its_start_time() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let mut body = distinct_gauge(&mut canvas);
    (body.starttime, body.endtime) = (1_000, 3_000);
    let bar = object(rect, Body::Gauge(body));
    let ended = gauged(GaugeFrame::finished(3, 90.0, [GROOVE; GAUGE_TYPES]));

    for (now_ms, lit) in [(0, 1), (999, 1), (2_000, 25), (3_000, 45)] {
        canvas.clear(Color::BLACK);
        draw_at(&mut canvas, &bar, &PlayState::default(), ended, now_ms);
        assert_eq!(lit_count(&canvas, rect), lit, "{now_ms} ms into the scene");
    }
}

/// The flickering gauge darkens nothing behind its leading part; it draws that part lit and fades
/// the leading cell in and out over it.
#[test]
fn a_flickering_gauge_fades_its_leading_cell_over_the_lit_one() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let reds = [FLICKER_LIT, FLICKER_LIT, FLICKER_UNLIT, FLICKER_UNLIT, FLICKER_LEADING, FLICKER_LEADING];
    let nodes: Vec<Sprite> = reds.iter().enumerate().map(|(state, red)| solid(&mut canvas, &format!("flicker{state}"), shade(*red))).collect();
    let mut slots = [None; 36];
    for (slot, entry) in slots.iter_mut().enumerate() {
        *entry = Some((slot % SLOTS_PER_GAUGE) as u8);
    }
    let mut body = gauge_of(nodes, slots, FEW_PARTS, GaugeAnimation::Flickering);
    (body.range, body.cycle) = (3, FLICKER_CYCLE_MS);
    let bar = object(rect, Body::Gauge(body));
    let half = gauged(GaugeFrame::playing(2, 50.0, [GROOVE; GAUGE_TYPES]));
    let row = (CANVAS.1 as f32 - (rect.y + rect.h / 2.0)) as u32;
    let red_of = |canvas: &CpuCanvas, part: i32| canvas.pixel_at((rect.x + rect.w * (part as f32 - 0.5) / FEW_PARTS as f32) as u32, row).r;

    draw_at(&mut canvas, &bar, &PlayState::default(), half, 0);
    assert_eq!(
        (1..=FEW_PARTS).map(|part| red_of(&canvas, part)).collect::<Vec<_>>(),
        [vec![FLICKER_LIT; 5], vec![FLICKER_UNLIT; 5]].concat(),
        "at the start of a cycle"
    );

    canvas.clear(Color::BLACK);
    draw_at(&mut canvas, &bar, &PlayState::default(), half, 49);
    assert_eq!(red_of(&canvas, 5), FLICKER_LEADING, "half way through, the leading cell is fully over the lit one");
    assert_eq!((red_of(&canvas, 4), red_of(&canvas, 6)), (FLICKER_LIT, FLICKER_UNLIT), "and its neighbours are untouched");

    canvas.clear(Color::BLACK);
    draw_at(&mut canvas, &bar, &PlayState::default(), half, 1_025);
    let faded = red_of(&canvas, 5);
    assert!((130..=150).contains(&faded), "a quarter of the way through, the leading cell is about half over the lit one: {faded}");

    canvas.clear(Color::BLACK);
    draw_at(&mut canvas, &bar, &PlayState::default(), half, 99);
    assert_eq!(red_of(&canvas, 5), FLICKER_LIT, "and at the end of the cycle it is gone again");
}

/// A record whose `type` is none of the four the reference lists is never drawn, and neither is a
/// gauge on a frame that carries none.
#[test]
fn a_gauge_with_no_animation_the_reference_knows_or_no_gauge_to_show_draws_nothing() {
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let shown = gauged(GaugeFrame::playing(2, 50.0, [GROOVE; GAUGE_TYPES]));

    let mut unlisted = distinct_gauge(&mut canvas);
    unlisted.animation = None;
    assert!(!draw_on(&mut canvas, &object(rect, Body::Gauge(unlisted)), &PlayState::default(), shown));

    let bar = object(rect, Body::Gauge(distinct_gauge(&mut canvas)));
    assert!(!draw_on(&mut canvas, &bar, &PlayState::default(), FrameData::default()));
    let past_the_last = gauged(GaugeFrame::playing(GAUGE_TYPES, 50.0, [GROOVE; GAUGE_TYPES]));
    assert!(!draw_on(&mut canvas, &bar, &PlayState::default(), past_the_last), "a type that names no gauge");
    assert!(draw_on(&mut canvas, &bar, &PlayState::default(), shown));
}

/// The judgement pop-up and the lane covers: each drawn from a body made by hand, so one rule can be
/// looked at alone, and once from a document, so the loader and the screen builder are seen to
/// arrange them the same way.
mod pop_ups_and_covers {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use rbms_model::Mode;
    use rbms_skin::dst::{DestinationTrack, DrawCondition, Keyframe, LuaDrawEval, LuaFnId, SkinOffset, SkinRect, TimerRef};
    use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, StretchKind, load_skin};
    use rbms_skin::model::{Animation, Destination};
    use rbms_skin::property::MapHost;
    use rbms_skin::timer::{MICROS_PER_MILLI, TimerId, TimerState};

    use super::super::covers::{CoverBody, attach_offsets};
    use super::super::draw::{ImageSelect, draw_object};
    use super::super::frame::{JUDGE_REGIONS, JudgeFrame, JudgeHit};
    use super::super::gauge::GAUGE_TYPES;
    use super::super::judge::{JudgeBody, JudgeCount, pull_count_left};
    use super::super::object::{Body, ImageBody, SkinObject, Sprite};
    use super::super::{FrameData, GaugeFrame, SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen, SkinViewport};
    use super::{CANVAS, GROOVE, digit_shade, digits, first_row_of, last_row_of, object, shade, solid, still, strip};
    use crate::ctx::{RenderCtx, with_render_ctx};
    use crate::font::TextContext;
    use crate::{Color, CpuCanvas, Renderer};

    /// Draws one object against a host made of maps, with the timers and the frame data given.
    fn show(canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost, timers: &TimerState, data: FrameData<'_>, now_ms: i64) -> bool {
        let frame = SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers, state: host, lua: None, mouse: None, data };
        let viewport = SkinViewport::new((CANVAS.0 as f32, CANVAS.1 as f32), (CANVAS.0 as f32, CANVAS.1 as f32));
        with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
    }

    /// The same, as the scene opens and with no timer on.
    fn show_still(canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost, data: FrameData<'_>) -> bool {
        show(canvas, object, host, &TimerState::new(), data, 0)
    }

    /// The screen row a document height falls on, for a pixel that sits just above it.
    fn row_above(document_y: f32) -> u32 {
        (CANVAS.1 as f32 - document_y) as u32 - 1
    }

    /// The first and last screen rows of `column` holding `color`.
    fn rows_of(canvas: &CpuCanvas, column: u32, color: Color) -> Option<(u32, u32)> {
        first_row_of(canvas, column, color).zip(last_row_of(canvas, column, color))
    }

    /// The screen rows a document span from `bottom` up to `top` covers.
    fn rows_between(bottom: f32, top: f32) -> Option<(u32, u32)> {
        Some(((CANVAS.1 as f32 - top) as u32, row_above(bottom)))
    }

    /// The first and last screen columns of `row` holding `color`.
    fn columns_of(canvas: &CpuCanvas, row: u32, color: Color) -> Option<(u32, u32)> {
        let holds = |column: &u32| canvas.pixel_at(*column, row) == color;
        (0..CANVAS.0).find(holds).zip((0..CANVAS.0).rev().find(holds))
    }

    /// The offset the lift is published under.
    const LIFT_OFFSET: i32 = 3;

    /// The offset the hidden cover is published under.
    const HIDDEN_OFFSET: i32 = 5;

    /// The line the test covers are cropped at, which is where the test field's judgement line is.
    const COVER_LINE: f32 = 100.0;

    /// Where a test cover sits before any offset: wholly below its line, with its top edge on it.
    const COVER_RECT: SkinRect = SkinRect::new(200.0, -400.0, 300.0, 500.0);

    /// How far the test lift raises the judgement line, in document pixels.
    const COVER_LIFT: f32 = 40.0;

    /// How far the test hidden modifier reaches up from the judgement line.
    const COVER_HIDDEN: f32 = 120.0;

    /// The alpha the hidden cover's offset carries while the modifier is off.
    const HIDDEN_OFF_ALPHA: f32 = -255.0;

    /// A column inside every test cover.
    const COVER_COLUMN: u32 = 350;

    /// A host publishing the lift and the hidden cover the way a play screen does: the lift as a
    /// height, and the hidden cover as a height while it is on and as a fade to nothing while it is off.
    fn covering(lift: f32, hidden: Option<f32>) -> MapHost {
        let mut host = MapHost::new();
        host.offsets.insert(LIFT_OFFSET, SkinOffset { y: lift, ..SkinOffset::default() });
        let hidden = match hidden {
            Some(height) => SkinOffset { y: height, ..SkinOffset::default() },
            None => SkinOffset { a: HIDDEN_OFF_ALPHA, ..SkinOffset::default() },
        };
        host.offsets.insert(HIDDEN_OFFSET, hidden);
        host
    }

    /// A cover over `rect`, given the offsets the loader gives one of its kind.
    fn cover_over(rect: SkinRect, body: Body) -> SkinObject {
        let mut track = still(rect);
        attach_offsets(&body, &mut track);
        SkinObject { track, stretch: StretchKind::from_id(-1), body }
    }

    /// A cover body of one flat colour.
    fn cover_body(canvas: &mut CpuCanvas, color: Color, disappear_line: f32, follows_lift: bool) -> CoverBody {
        CoverBody { sprite: solid(canvas, "cover", color), disappear_line, follows_lift }
    }

    #[test]
    fn the_loader_gives_a_hidden_cover_the_lift_and_hidden_offsets_and_a_lift_cover_the_lift() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let declared = DestinationTrack { offsets: vec![40, 0], ..still(COVER_RECT) };

        let mut hidden = declared.clone();
        attach_offsets(&Body::HiddenCover(cover_body(&mut canvas, shade(210), COVER_LINE, true)), &mut hidden);
        assert_eq!(hidden.offsets, vec![40, 0, LIFT_OFFSET, HIDDEN_OFFSET], "after whatever the document's own destination names");

        let mut lift = declared.clone();
        attach_offsets(&Body::LiftCover(cover_body(&mut canvas, shade(210), COVER_LINE, false)), &mut lift);
        assert_eq!(lift.offsets, vec![40, 0, LIFT_OFFSET]);

        let mut named = DestinationTrack { offsets: vec![LIFT_OFFSET], ..still(COVER_RECT) };
        attach_offsets(&Body::HiddenCover(cover_body(&mut canvas, shade(210), COVER_LINE, true)), &mut named);
        assert_eq!(named.offsets, vec![LIFT_OFFSET, HIDDEN_OFFSET], "an offset the document already names is applied once, not twice");

        let mut other = declared.clone();
        let image = Body::Image(ImageBody { variants: vec![Some((solid(&mut canvas, "plain", shade(210)), 0, 1))], select: ImageSelect::First });
        attach_offsets(&image, &mut other);
        assert_eq!(other.offsets, declared.offsets, "no other object is given an offset it did not name");
    }

    /// A hidden cover's band stands on the judgement line the lift raised and reaches up by what the
    /// hidden modifier hides. Everything of the image below that line is cropped.
    #[test]
    fn a_hidden_cover_rises_from_the_lifted_judgement_line_by_what_the_modifier_hides() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let color = shade(210);
        let cover = cover_over(COVER_RECT, Body::HiddenCover(cover_body(&mut canvas, color, COVER_LINE, true)));

        assert!(show_still(&mut canvas, &cover, &covering(COVER_LIFT, Some(COVER_HIDDEN)), FrameData::default()), "the cover drew nothing at all");
        let foot = COVER_LINE + COVER_LIFT;
        assert_eq!(
            rows_of(&canvas, COVER_COLUMN, color),
            rows_between(foot, foot + COVER_HIDDEN),
            "from the raised line up to the lift and the hidden height above it"
        );
        assert_eq!(
            columns_of(&canvas, row_above(foot), color),
            Some((COVER_RECT.x as u32, (COVER_RECT.x + COVER_RECT.w) as u32 - 1)),
            "across the width the document gave it"
        );

        canvas.clear(Color::BLACK);
        assert!(show_still(&mut canvas, &cover, &covering(0.0, Some(COVER_HIDDEN)), FrameData::default()));
        assert_eq!(
            rows_of(&canvas, COVER_COLUMN, color),
            rows_between(COVER_LINE, COVER_LINE + COVER_HIDDEN),
            "with no lift it stands on the line the document wrote"
        );
    }

    /// The hidden cover's offset is the only thing that keeps a cover the player has not asked for off
    /// the screen: while the modifier is off it fades the cover to nothing.
    #[test]
    fn a_hidden_cover_is_faded_to_nothing_while_the_modifier_is_off() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let color = shade(210);
        let on_screen = SkinRect::new(200.0, 100.0, 300.0, 200.0);
        let cover = cover_over(on_screen, Body::HiddenCover(cover_body(&mut canvas, color, -1.0, true)));

        assert!(!show_still(&mut canvas, &cover, &covering(0.0, None), FrameData::default()), "a cover faded to nothing is not drawn");
        assert_eq!(rows_of(&canvas, COVER_COLUMN, color), None);
        assert!(show_still(&mut canvas, &cover, &covering(0.0, Some(0.0)), FrameData::default()), "the same cover with the modifier on is");
    }

    /// A lift cover is moved by the lift alone and its line stays where the document put it, so what
    /// shows is exactly the band between where the judgement line was and where the lift put it.
    #[test]
    fn a_lift_cover_shows_the_band_the_lift_opened_below_the_judgement_line() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let color = shade(190);
        let cover = cover_over(COVER_RECT, Body::LiftCover(cover_body(&mut canvas, color, COVER_LINE, false)));

        assert!(
            !show_still(&mut canvas, &cover, &covering(0.0, Some(COVER_HIDDEN)), FrameData::default()),
            "with no lift the cover does not reach above its line"
        );
        assert!(show_still(&mut canvas, &cover, &covering(COVER_LIFT, Some(COVER_HIDDEN)), FrameData::default()), "the lift cover drew nothing at all");
        assert_eq!(rows_of(&canvas, COVER_COLUMN, color), rows_between(COVER_LINE, COVER_LINE + COVER_LIFT), "the hidden modifier does not move it");
    }

    /// The line crops the image; it does not squeeze it into what is left. A cover whose image is one
    /// colour above another shows only as much of the upper colour as rose above the line.
    #[test]
    fn a_cover_is_cropped_at_its_line_rather_than_resized() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let (upper, lower) = (shade(220), shade(110));
        let pixels = [upper.r, 0, 0, 255, lower.r, 0, 0, 255];
        let tex = canvas.register_texture("two-tone", &pixels, 1, 2);
        let sprite = Sprite { tex, size: (1, 2), origin: (0, 0), cell: (1, 2), columns: 1, rows: 1, timer: None, cycle: 0 };
        let cover = cover_over(COVER_RECT, Body::HiddenCover(CoverBody { sprite, disappear_line: COVER_LINE, follows_lift: true }));

        assert!(show_still(&mut canvas, &cover, &covering(0.0, Some(COVER_HIDDEN)), FrameData::default()));
        assert_eq!(rows_of(&canvas, COVER_COLUMN, upper), rows_between(COVER_LINE, COVER_LINE + COVER_HIDDEN), "the band shows the top of the image");
        assert_eq!(rows_of(&canvas, COVER_COLUMN, lower), None, "and none of the half that is still below the line");
    }

    /// A cover with no line is an image moved by its offsets and nothing more, and one that has risen
    /// clear of its line is drawn whole.
    #[test]
    fn a_cover_with_no_line_or_wholly_above_it_is_drawn_whole() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let color = shade(210);
        let on_screen = SkinRect::new(200.0, 100.0, 300.0, 200.0);
        let whole = rows_between(on_screen.y + COVER_LIFT, on_screen.y + on_screen.h + COVER_LIFT);

        let unlined = cover_over(on_screen, Body::LiftCover(cover_body(&mut canvas, color, -1.0, false)));
        assert!(show_still(&mut canvas, &unlined, &covering(COVER_LIFT, None), FrameData::default()));
        assert_eq!(rows_of(&canvas, COVER_COLUMN, color), whole, "no line crops nothing");

        canvas.clear(Color::BLACK);
        let above = cover_over(on_screen, Body::LiftCover(cover_body(&mut canvas, color, COVER_LINE, false)));
        assert!(show_still(&mut canvas, &above, &covering(COVER_LIFT, None), FrameData::default()));
        assert_eq!(rows_of(&canvas, COVER_COLUMN, color), whole, "a line below the cover crops nothing either");
    }

    /// Where a pop-up's word sits, in the document's own space.
    const WORD_RECT: SkinRect = SkinRect::new(100.0, 200.0, 40.0, 20.0);

    /// How far to the right of the word's left edge the document centres the count.
    const COUNT_ANCHOR: i32 = 60;

    /// One place of the count.
    const COUNT_PLACE: (i32, i32) = (8, 12);

    /// The screen row the word and the count both cross.
    const POP_UP_ROW: u32 = CANVAS.1 - 205;

    /// How far apart the two pop-ups of a double field stand.
    const SECOND_FIELD: f32 = 600.0;

    /// The colour each of a pop-up's seven words is drawn in, by slot: green, where every digit is red.
    fn word_shade(slot: usize) -> Color {
        Color { r: 0, g: 100 + 20 * slot as u8, b: 0, a: 255 }
    }

    /// The red the alternate zero of an eleven-glyph strip is drawn in.
    const ALTERNATE_ZERO_SHADE: u8 = 240;

    /// One part of a pop-up over `track`: the strip it is cut from, held whole.
    fn judge_part(track: DestinationTrack, sprite: Sprite) -> SkinObject {
        let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, sprite.cells()))], select: ImageSelect::First });
        SkinObject { track, stretch: StretchKind::from_id(-1), body }
    }

    /// The destination a document writes for a count: one keyframe, a place wide, centred
    /// [`COUNT_ANCHOR`] to the right of the word and level with its foot.
    fn count_destination() -> Destination {
        let frame = Animation { time: Some(0), x: Some(COUNT_ANCHOR), y: Some(0), w: Some(COUNT_PLACE.0), h: Some(COUNT_PLACE.1), ..Animation::default() };
        Destination { dst: vec![frame], ..Destination::default() }
    }

    /// The track the loader assembles for that destination and then pulls left for a count of `places`.
    fn count_track(places: i32) -> DestinationTrack {
        let rect = SkinRect::new(COUNT_ANCHOR as f32, 0.0, COUNT_PLACE.0 as f32, COUNT_PLACE.1 as f32);
        let mut track = DestinationTrack { relative: true, ..still(rect) };
        pull_count_left(&mut track, &count_destination(), places);
        track
    }

    /// A count of `places` cut from a strip of ten digits.
    fn judge_count(ten_digits: Sprite, places: i32) -> JudgeCount {
        JudgeCount { part: judge_part(count_track(places), ten_digits), glyphs: 10, sets: 1, places: places as u32, space: 0.0, offsets: Vec::new() }
    }

    /// A pop-up of `slots` words, each a colour of its own, with a count of `places` beside every one.
    fn pop_up(canvas: &mut CpuCanvas, slots: usize, places: i32, shift: bool) -> JudgeBody {
        let words = (0..slots).map(|slot| Some(judge_part(still(WORD_RECT), solid(canvas, &format!("word{slot}"), word_shade(slot))))).collect();
        let ten_digits = digits(canvas);
        let counts = (0..slots).map(|_| Some(judge_count(ten_digits, places))).collect();
        JudgeBody { words, counts, shift, region: 0 }
    }

    /// The draw-list entry a pop-up is: an object that places itself and leaves everything to its parts.
    fn popped(body: JudgeBody) -> SkinObject {
        object(SkinRect::new(0.0, 0.0, 0.0, 0.0), Body::Judge(body))
    }

    /// A frame in which region zero last took `judgement` with the run's combo at `combo`.
    fn judged(judgement: usize, combo: i32) -> FrameData<'static> {
        FrameData { judge: JudgeFrame::default().with_region(0, JudgeHit { judgement, combo, at_us: 0 }), ..FrameData::default() }
    }

    /// The same frame on a gauge that is full.
    fn judged_on_a_full_gauge(judgement: usize, combo: i32) -> FrameData<'static> {
        FrameData { gauge: Some(GaugeFrame::playing(2, GROOVE.max, [GROOVE; GAUGE_TYPES])), ..judged(judgement, combo) }
    }

    /// The word on show in the pop-up's row, wherever a shift has slid it.
    fn word_on_show(canvas: &CpuCanvas) -> Option<Color> {
        (0..7).map(word_shade).find(|color| columns_of(canvas, POP_UP_ROW, *color).is_some())
    }

    /// The digits on show in the pop-up's row, left to right, read from the colour of each run of
    /// places, with the screen column the first one starts at.
    fn count_on_show(canvas: &CpuCanvas, from: u32) -> (Option<u32>, Vec<u8>) {
        let digit_of = |column: u32| {
            let pixel = canvas.pixel_at(column, POP_UP_ROW);
            (0..10u8).find(|digit| pixel == shade(digit_shade(*digit))).or((pixel == shade(ALTERNATE_ZERO_SHADE)).then_some(u8::MAX))
        };
        let first = (from..CANVAS.0).find(|column| digit_of(*column).is_some());
        let read = first.map(|first| (first..CANVAS.0).step_by(COUNT_PLACE.0 as usize).map_while(digit_of).collect()).unwrap_or_default();
        (first, read)
    }

    /// The screen column the document centres a count on, for a word that has not been slid.
    const COUNT_CENTRE: u32 = (WORD_RECT.x as i32 + COUNT_ANCHOR) as u32;

    #[test]
    fn a_pop_up_shows_the_word_of_the_judgement_its_region_took() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let pop_up = popped(pop_up(&mut canvas, 6, 4, false));
        let host = MapHost::new();
        for judgement in 0..6 {
            canvas.clear(Color::BLACK);
            assert!(show_still(&mut canvas, &pop_up, &host, judged(judgement, 7)), "judgement {judgement} drew nothing");
            assert_eq!(word_on_show(&canvas), Some(word_shade(judgement)), "judgement {judgement} shows the word at its own place in the list");
            let rows = rows_of(&canvas, WORD_RECT.x as u32 + 1, word_shade(judgement));
            assert_eq!(rows, rows_between(WORD_RECT.y, WORD_RECT.y + WORD_RECT.h), "where its own destination put it");
        }

        canvas.clear(Color::BLACK);
        assert!(!show_still(&mut canvas, &pop_up, &host, FrameData::default()), "a region that has judged nothing shows no pop-up");
        assert!(!show_still(&mut canvas, &pop_up, &host, judged(6, 7)), "and there is no seventh judgement to show a word for");
    }

    /// A perfect great on a full gauge reads the seventh slot, word and count apart, and the first slot
    /// where the document has no seventh.
    #[test]
    fn a_perfect_great_on_a_full_gauge_shows_the_seventh_word_or_else_the_first() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let host = MapHost::new();
        let mut seven = pop_up(&mut canvas, 7, 4, false);
        seven.counts[6] = None;
        let seven = popped(seven);

        assert!(show_still(&mut canvas, &seven, &host, judged_on_a_full_gauge(0, 12)));
        assert_eq!(word_on_show(&canvas), Some(word_shade(6)), "the seventh word is the full gauge's");
        assert_eq!(count_on_show(&canvas, COUNT_CENTRE - 20).1, vec![1, 2], "and with no seventh count the first one rides beside it");

        canvas.clear(Color::BLACK);
        show_still(&mut canvas, &seven, &host, judged(0, 12));
        assert_eq!(word_on_show(&canvas), Some(word_shade(0)), "a gauge that is not full shows the ordinary word");

        canvas.clear(Color::BLACK);
        show_still(&mut canvas, &seven, &host, judged_on_a_full_gauge(1, 12));
        assert_eq!(word_on_show(&canvas), Some(word_shade(1)), "and so does every judgement but the best");

        canvas.clear(Color::BLACK);
        let six = popped(pop_up(&mut canvas, 6, 4, false));
        show_still(&mut canvas, &six, &host, judged_on_a_full_gauge(0, 12));
        assert_eq!(word_on_show(&canvas), Some(word_shade(0)), "a document with six words shows the first on a full gauge too");
    }

    /// The loader pulls a count left by half the places it reserves and the count centres its digits
    /// over those places, so whatever the count reserves and however many digits are on show, the run
    /// stays centred on the one place the document named.
    #[test]
    fn a_count_stays_centred_on_the_place_the_document_gave_it() {
        let host = MapHost::new();
        let combos: [(i32, &[u8]); 4] = [(7, &[7]), (42, &[4, 2]), (305, &[3, 0, 5]), (1984, &[1, 9, 8, 4])];
        for places in 1..=4usize {
            for (combo, expected) in combos.iter().take(places) {
                let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
                let pop_up = popped(pop_up(&mut canvas, 6, places as i32, false));
                assert!(show_still(&mut canvas, &pop_up, &host, judged(1, *combo)));

                let (first, read) = count_on_show(&canvas, WORD_RECT.x as u32 + WORD_RECT.w as u32);
                let half = COUNT_PLACE.0 as u32 * expected.len() as u32 / 2;
                assert_eq!(read, expected.to_vec(), "{places} places showing {combo}");
                assert_eq!(first, Some(COUNT_CENTRE - half), "{places} places showing {combo}: the run starts half its own width left of the anchor");
                assert_eq!(
                    columns_of(&canvas, POP_UP_ROW, word_shade(1)).map(|(left, _)| left),
                    Some(WORD_RECT.x as u32),
                    "and the word stays where it was put"
                );
            }
        }
    }

    /// With `shift` the word slides left by half the width of the digits on show. The count is placed
    /// against the word as it was before the slide, so the pair is centred as one.
    #[test]
    fn a_shifting_word_slides_left_by_half_the_digits_on_show() {
        let host = MapHost::new();
        for (combo, digits_shown) in [(7, 1u32), (42, 2), (305, 3), (1984, 4)] {
            let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
            let pop_up = popped(pop_up(&mut canvas, 6, 4, true));
            assert!(show_still(&mut canvas, &pop_up, &host, judged(0, combo)));

            let half = COUNT_PLACE.0 as u32 * digits_shown / 2;
            let word = columns_of(&canvas, POP_UP_ROW, word_shade(0));
            assert_eq!(word, Some((WORD_RECT.x as u32 - half, (WORD_RECT.x + WORD_RECT.w) as u32 - half - 1)), "a combo of {combo}");
            let (first, _) = count_on_show(&canvas, WORD_RECT.x as u32 + WORD_RECT.w as u32);
            assert_eq!(first, Some(COUNT_CENTRE - half), "the count of {combo} does not move with the word");
        }
    }

    /// A bad, a poor and a miss break the combo: no count is drawn beside them and, with nothing to make
    /// room for, the word does not slide.
    #[test]
    fn a_judgement_that_breaks_the_combo_draws_no_count_and_does_not_slide() {
        let host = MapHost::new();
        for judgement in 3..6 {
            let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
            let pop_up = popped(pop_up(&mut canvas, 6, 4, true));
            assert!(show_still(&mut canvas, &pop_up, &host, judged(judgement, 1984)));
            assert_eq!(count_on_show(&canvas, 0), (None, Vec::new()), "judgement {judgement} shows no digits");
            assert_eq!(columns_of(&canvas, POP_UP_ROW, word_shade(judgement)).map(|(left, _)| left), Some(WORD_RECT.x as u32), "and its word stays put");
        }
    }

    /// A double field has a pop-up for each half, and each shows the judgement and the combo of its own
    /// region. A pop-up that names a region the frame has nothing for is not drawn.
    #[test]
    fn two_regions_keep_a_judgement_and_a_combo_each() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let left = popped(pop_up(&mut canvas, 6, 4, false));
        let mut right = pop_up(&mut canvas, 6, 4, false);
        right.region = 1;
        for word in right.words.iter_mut().flatten() {
            word.track.frames[0].rect.x += SECOND_FIELD;
        }
        let right = popped(right);
        let host = MapHost::new();

        let both = JudgeFrame::default()
            .with_region(0, JudgeHit { judgement: 0, combo: 12, at_us: 0 })
            .with_region(1, JudgeHit { judgement: 2, combo: 345, at_us: 0 });
        let data = FrameData { judge: both, ..FrameData::default() };
        assert!(show_still(&mut canvas, &left, &host, data));
        assert!(show_still(&mut canvas, &right, &host, data));
        let second_field = SECOND_FIELD as u32;
        assert_eq!(canvas.pixel_at(WORD_RECT.x as u32 + 1, POP_UP_ROW), word_shade(0), "the left field shows its own judgement");
        assert_eq!(canvas.pixel_at(WORD_RECT.x as u32 + second_field + 1, POP_UP_ROW), word_shade(2), "and the right field its own");
        assert_eq!(count_on_show(&canvas, WORD_RECT.x as u32 + WORD_RECT.w as u32).1, vec![1, 2], "the left count is the left region's combo");
        assert_eq!(count_on_show(&canvas, second_field + WORD_RECT.x as u32 + WORD_RECT.w as u32).1, vec![3, 4, 5], "and the right count the right region's");

        canvas.clear(Color::BLACK);
        assert!(!show_still(&mut canvas, &right, &host, judged(0, 12)), "a region that has judged nothing shows nothing, whatever the other one took");

        for region in [-1, 2, 3, 7] {
            let mut stray = pop_up(&mut canvas, 6, 4, false);
            stray.region = region;
            assert!(!show_still(&mut canvas, &popped(stray), &host, data), "region {region} is not folded onto one the frame has");
        }
    }

    /// The timer a region's judgements switch on.
    const JUDGE_TIMER: TimerId = TimerId(46);

    /// How long the test pop-up stays on show, in milliseconds.
    const POP_UP_SHOWN_MS: i64 = 500;

    /// When the test judgement lands, in milliseconds.
    const JUDGED_AT_MS: i64 = 1_000;

    /// A destination that follows the judge timer and plays once, as a document writes one for a
    /// pop-up's parts.
    fn played_once(mut track: DestinationTrack) -> DestinationTrack {
        let last = Keyframe { time_ms: POP_UP_SHOWN_MS, ..track.frames[0] };
        track.frames.push(last);
        track.timer = Some(TimerRef::Id(JUDGE_TIMER));
        track.loop_ms = -1;
        track
    }

    /// Nothing clears a region's judgement. The pop-up comes and goes with the timer its parts follow.
    #[test]
    fn a_pop_up_comes_and_goes_with_the_timer_its_parts_follow() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let mut body = pop_up(&mut canvas, 6, 4, false);
        for word in body.words.iter_mut().flatten() {
            word.track = played_once(word.track.clone());
        }
        for count in body.counts.iter_mut().flatten() {
            count.part.track = played_once(count.part.track.clone());
        }
        let pop_up = popped(body);
        let host = MapHost::new();
        let data = judged(1, 42);

        let off = TimerState::new();
        assert!(!show(&mut canvas, &pop_up, &host, &off, data, JUDGED_AT_MS), "with the judge timer off there is no pop-up");

        let mut on = TimerState::new();
        on.set_on(JUDGE_TIMER, JUDGED_AT_MS * MICROS_PER_MILLI);
        for now_ms in [JUDGED_AT_MS, JUDGED_AT_MS + 250, JUDGED_AT_MS + POP_UP_SHOWN_MS] {
            canvas.clear(Color::BLACK);
            assert!(show(&mut canvas, &pop_up, &host, &on, data, now_ms), "{now_ms} ms");
            assert_eq!(word_on_show(&canvas), Some(word_shade(1)), "{now_ms} ms");
            assert_eq!(count_on_show(&canvas, WORD_RECT.x as u32 + WORD_RECT.w as u32).1, vec![4, 2], "{now_ms} ms");
        }
        canvas.clear(Color::BLACK);
        assert!(!show(&mut canvas, &pop_up, &host, &on, data, JUDGED_AT_MS + POP_UP_SHOWN_MS + 1), "once it has played it is gone until the next judgement");

        on.set_on(JUDGE_TIMER, (JUDGED_AT_MS + 2_000) * MICROS_PER_MILLI);
        assert!(show(&mut canvas, &pop_up, &host, &on, data, JUDGED_AT_MS + 2_100), "and the next judgement starts it again");
    }

    /// The count is placed against the word's corner, so whatever moves the word moves the count with
    /// it. The count's own offsets resize its places without moving them.
    #[test]
    fn a_count_rides_the_word_and_takes_only_a_size_from_its_own_offsets() {
        const USER_OFFSET: i32 = 32;
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let mut body = pop_up(&mut canvas, 6, 2, false);
        for word in body.words.iter_mut().flatten() {
            word.track.offsets = vec![LIFT_OFFSET];
        }
        for count in body.counts.iter_mut().flatten() {
            count.part.track.offsets = vec![USER_OFFSET];
        }
        let pop_up = popped(body);
        let mut host = covering(COVER_LIFT, None);
        host.offsets.insert(USER_OFFSET, SkinOffset { x: 300.0, y: 300.0, w: 4.0, h: 6.0, ..SkinOffset::default() });

        assert!(show_still(&mut canvas, &pop_up, &host, judged(0, 42)));
        let lifted = WORD_RECT.y + COVER_LIFT;
        assert_eq!(rows_of(&canvas, WORD_RECT.x as u32 + 1, word_shade(0)), rows_between(lifted, lifted + WORD_RECT.h), "the lift raises the word");

        let place = COUNT_PLACE.0 as f32 + 4.0;
        let first = WORD_RECT.x + COUNT_ANCHOR as f32 - COUNT_PLACE.0 as f32;
        let four = shade(digit_shade(4));
        let two = shade(digit_shade(2));
        let row = row_above(lifted);
        assert_eq!(columns_of(&canvas, row, four), Some((first as u32, (first + place) as u32 - 1)), "the first place is where it was, and wider");
        assert_eq!(columns_of(&canvas, row, two), Some(((first + place) as u32, (first + place * 2.0) as u32 - 1)), "the next follows at the wider step");
        assert_eq!(
            rows_of(&canvas, first as u32, four),
            rows_between(lifted, lifted + COUNT_PLACE.1 as f32 + 6.0),
            "and it rose with the word and grew taller"
        );
    }

    /// A strip of eleven glyphs keeps an alternate zero, which fills every place the combo leaves empty.
    /// The count is then as wide as the places it reserves, and that is what a shifting word slides by.
    #[test]
    fn a_strip_with_an_alternate_zero_fills_the_places_a_combo_leaves_empty() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let pixels: Vec<u8> = (0..10).map(digit_shade).chain([ALTERNATE_ZERO_SHADE]).flat_map(|red| [red, 0, 0, 255]).collect();
        let eleven = strip(canvas.register_texture("eleven", &pixels, 11, 1), 11);
        let mut body = pop_up(&mut canvas, 6, 4, true);
        for count in body.counts.iter_mut().flatten() {
            *count = JudgeCount { part: judge_part(count.part.track.clone(), eleven), glyphs: 11, sets: 1, places: 4, space: 0.0, offsets: Vec::new() };
        }
        assert!(show_still(&mut canvas, &popped(body), &MapHost::new(), judged(0, 12)));

        let (first, read) = count_on_show(&canvas, WORD_RECT.x as u32 + WORD_RECT.w as u32);
        assert_eq!(read, vec![u8::MAX, u8::MAX, 1, 2], "two alternate zeroes lead the combo");
        assert_eq!(first, Some(COUNT_CENTRE - COUNT_PLACE.0 as u32 * 2), "over all four places");
        let slid = WORD_RECT.x as u32 - COUNT_PLACE.0 as u32 * 2;
        assert_eq!(columns_of(&canvas, POP_UP_ROW, word_shade(0)).map(|(left, _)| left), Some(slid), "and the word slides by half of all four");
    }

    /// A strip of more than one set steps through its sets by its own cycle, on the frame clock when it
    /// names no timer.
    #[test]
    fn a_count_steps_through_the_sets_of_its_strip() {
        const SET_CYCLE_MS: i32 = 100;
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let pixels: Vec<u8> = (0..20u8).flat_map(|cell| [digit_shade(cell % 10), if cell < 10 { 0 } else { 200 }, 0, 255]).collect();
        let tex = canvas.register_texture("two-sets", &pixels, 10, 2);
        let two_sets = Sprite { tex, size: (10, 2), origin: (0, 0), cell: (1, 1), columns: 10, rows: 2, timer: None, cycle: SET_CYCLE_MS };
        let mut body = pop_up(&mut canvas, 6, 1, false);
        for count in body.counts.iter_mut().flatten() {
            *count = JudgeCount { part: judge_part(count.part.track.clone(), two_sets), glyphs: 10, sets: 2, places: 1, space: 0.0, offsets: Vec::new() };
        }
        let pop_up = popped(body);
        let host = MapHost::new();
        let column = COUNT_CENTRE - COUNT_PLACE.0 as u32 / 2 + 1;

        show(&mut canvas, &pop_up, &host, &TimerState::new(), judged(0, 7), 0);
        assert_eq!(canvas.pixel_at(column, POP_UP_ROW), shade(digit_shade(7)), "the first set as the cycle starts");
        show(&mut canvas, &pop_up, &host, &TimerState::new(), judged(0, 7), i64::from(SET_CYCLE_MS) / 2);
        assert_eq!(canvas.pixel_at(column, POP_UP_ROW), Color { r: digit_shade(7), g: 200, b: 0, a: 255 }, "the second half way through it");
    }

    /// The whole-number arithmetic the loader pulls a count left with, on the keyframes as the document
    /// wrote them.
    #[test]
    fn the_loader_pulls_a_count_left_with_the_references_own_arithmetic() {
        let keyframe = |time: i64, x: Option<i32>, w: Option<i32>| Animation { time: Some(time), x, w, ..Animation::default() };
        let pulled = |frames: &[Animation], digit: i32| {
            let declared = Destination { dst: frames.to_vec(), ..Destination::default() };
            let blank = still(SkinRect::default()).frames[0];
            let mut filled: Vec<Keyframe> = frames.iter().map(|frame| Keyframe { time_ms: frame.time.unwrap_or(0), ..blank }).collect();
            filled.sort_by_key(|frame| frame.time_ms);
            let mut track = DestinationTrack { frames: filled, ..DestinationTrack::default() };
            pull_count_left(&mut track, &declared, digit);
            track.frames.iter().map(|frame| frame.rect.x).collect::<Vec<f32>>()
        };

        assert_eq!(pulled(&[keyframe(0, Some(237), Some(55))], 6), vec![72.0], "half of six places of fifty-five");
        assert_eq!(pulled(&[keyframe(0, Some(60), Some(7))], 3), vec![50.0], "ten and a half is ten: the division drops the half");
        assert_eq!(
            pulled(&[keyframe(0, Some(237), Some(55)), keyframe(500, None, None)], 6),
            vec![72.0, 72.0],
            "a keyframe that names neither inherits the one before"
        );
        assert_eq!(
            pulled(&[keyframe(0, Some(237), Some(55)), keyframe(500, Some(300), None)], 6),
            vec![72.0, 300.0],
            "one that names an x and no width is not moved at all, as the reference leaves it"
        );
        assert_eq!(
            pulled(&[keyframe(500, Some(100), Some(10)), keyframe(0, Some(40), Some(10))], 2),
            vec![30.0, 90.0],
            "written out of order, they land on the keyframes they belong to"
        );

        let mut unassembled = DestinationTrack::default();
        pull_count_left(&mut unassembled, &Destination { dst: vec![keyframe(0, Some(237), Some(55))], ..Destination::default() }, 6);
        assert!(unassembled.frames.is_empty(), "a slot the loader assembled no keyframe for is left alone");
    }

    /// The function a test pop-up's own destination is gated on.
    const POP_UP_GATE: LuaFnId = LuaFnId(1);

    /// The function its words are gated on.
    const WORD_GATE: LuaFnId = LuaFnId(2);

    /// An evaluator that keeps which conditions it was asked, in order, and answers each as told.
    struct Asked {
        pop_up_holds: bool,
        calls: RefCell<Vec<LuaFnId>>,
    }

    impl LuaDrawEval for Asked {
        fn call_boolean(&self, function: LuaFnId) -> bool {
            self.calls.borrow_mut().push(function);
            function != POP_UP_GATE || self.pop_up_holds
        }
    }

    /// The reference asks whether the region has judged anything before it asks the pop-up's own
    /// conditions, and prepares the pop-up's parts even when those conditions have left it out. A skin
    /// sees both, because a condition written as a function is called exactly then.
    #[test]
    fn a_pop_up_asks_the_skin_in_the_order_the_reference_does() {
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let mut body = pop_up(&mut canvas, 6, 4, false);
        for word in body.words.iter_mut().flatten() {
            word.track.draw_conditions = vec![DrawCondition::Function(WORD_GATE)];
        }
        let mut pop_up = popped(body);
        pop_up.track.draw_conditions = vec![DrawCondition::Function(POP_UP_GATE)];
        let host = MapHost::new();
        let timers = TimerState::new();
        let prepared = |lua: &Asked, data: FrameData<'_>| {
            pop_up.prepare(&SkinFrame { now_us: 0, timers: &timers, state: &host, lua: Some(lua), mouse: None, data }).is_some()
        };

        let unjudged = Asked { pop_up_holds: true, calls: RefCell::default() };
        assert!(!prepared(&unjudged, FrameData::default()));
        assert_eq!(*unjudged.calls.borrow(), Vec::new(), "a pop-up with nothing to report asks the skin nothing");

        let shown = Asked { pop_up_holds: true, calls: RefCell::default() };
        assert!(prepared(&shown, judged(0, 12)));
        assert_eq!(*shown.calls.borrow(), vec![POP_UP_GATE, WORD_GATE], "its own condition first, then its word's");

        let left_out = Asked { pop_up_holds: false, calls: RefCell::default() };
        assert!(!prepared(&left_out, judged(0, 12)), "a pop-up its own condition leaves out is not drawn");
        assert_eq!(*left_out.calls.borrow(), vec![POP_UP_GATE, WORD_GATE], "but its word is prepared all the same");
    }

    #[test]
    fn a_frame_answers_only_for_the_regions_it_has() {
        let hit = JudgeHit { judgement: 2, combo: 9, at_us: 5 };
        let frame = JudgeFrame::default().with_region(1, hit).with_region(JUDGE_REGIONS, hit);
        assert_eq!(frame.region(1), Some(hit));
        assert_eq!(frame.region(0), None, "a region that has judged nothing");
        assert_eq!((frame.region(-1), frame.region(JUDGE_REGIONS as i32)), (None, None), "and one no play screen has");
    }

    /// The size the document fixture is authored at, which is the canvas it is drawn on.
    const DOCUMENT_SIZE: (u32, u32) = CANVAS;

    /// The words of the document fixture's sheet, one pixel each and a colour each, left to right.
    const DOCUMENT_WORDS: u32 = 7;

    /// A play document with two pop-ups, a hidden cover and a lift cover, written the way a skin writes
    /// them: the count placed against its word, the covers hung below the judgement line.
    const PLAY_DOCUMENT: &str = r#"{
        "type": 0,
        "name": "judge and cover fixture",
        "w": 1280,
        "h": 720,
        "source": [
            { "id": "words", "path": "words.tex" },
            { "id": "digits", "path": "digits.tex" },
            { "id": "cover", "path": "cover.tex" }
        ],
        "image": [
            { "id": "word-pg", "src": "words", "x": 0, "y": 0, "w": 1, "h": 1 },
            { "id": "word-gr", "src": "words", "x": 1, "y": 0, "w": 1, "h": 1 },
            { "id": "word-gd", "src": "words", "x": 2, "y": 0, "w": 1, "h": 1 },
            { "id": "word-bd", "src": "words", "x": 3, "y": 0, "w": 1, "h": 1 }
        ],
        "value": [{ "id": "count", "src": "digits", "x": 0, "y": 0, "w": 10, "h": 1, "divx": 10, "digit": 6, "ref": 105 }],
        "hiddenCover": [{ "id": "hidden", "src": "cover", "x": 0, "y": 0, "w": 1, "h": 1, "disapearLine": 100 }],
        "liftCover": [{ "id": "lift", "src": "cover", "x": 0, "y": 0, "w": 1, "h": 1, "disapearLine": 100 }],
        "judge": [
            {
                "id": "left",
                "index": 0,
                "shift": true,
                "images": [
                    { "id": "word-pg", "timer": 46, "loop": -1, "offsets": [3], "dst": [{ "time": 0, "x": 100, "y": 200, "w": 40, "h": 20 }, { "time": 500 }] },
                    { "id": "word-gr", "timer": 46, "loop": -1, "offsets": [3], "dst": [{ "time": 0, "x": 100, "y": 200, "w": 40, "h": 20 }, { "time": 500 }] },
                    { "id": "word-gd", "timer": 46, "loop": -1, "offsets": [3], "dst": [{ "time": 0, "x": 100, "y": 200, "w": 40, "h": 20 }, { "time": 500 }] },
                    { "id": "word-bd", "timer": 46, "loop": -1, "offsets": [3], "dst": [{ "time": 0, "x": 100, "y": 200, "w": 40, "h": 20 }, { "time": 500 }] }
                ],
                "numbers": [
                    { "id": "count", "timer": 46, "loop": -1, "dst": [{ "time": 0, "x": 74, "y": 0, "w": 8, "h": 12 }, { "time": 500 }] },
                    { "id": "count", "timer": 46, "loop": -1, "dst": [{ "time": 0, "x": 74, "y": 0, "w": 8, "h": 12 }, { "time": 500 }] },
                    { "id": "count", "timer": 46, "loop": -1, "dst": [{ "time": 0, "x": 74, "y": 0, "w": 8, "h": 12 }, { "time": 500 }] },
                    { "id": "count", "timer": 46, "loop": -1, "dst": [{ "time": 0, "x": 74, "y": 0, "w": 8, "h": 12 }, { "time": 500 }] }
                ]
            },
            {
                "id": "right",
                "index": 1,
                "images": [
                    { "id": "word-pg", "timer": 47, "loop": -1, "dst": [{ "time": 0, "x": 700, "y": 200, "w": 40, "h": 20 }, { "time": 500 }] },
                    { "id": "word-gr", "timer": 47, "loop": -1, "dst": [{ "time": 0, "x": 700, "y": 200, "w": 40, "h": 20 }, { "time": 500 }] }
                ],
                "numbers": [
                    { "id": "count", "timer": 47, "loop": -1, "dst": [{ "time": 0, "x": 74, "y": 0, "w": 8, "h": 12 }, { "time": 500 }] },
                    { "id": "count", "timer": 47, "loop": -1, "dst": [{ "time": 0, "x": 74, "y": 0, "w": 8, "h": 12 }, { "time": 500 }] }
                ]
            },
            { "id": "unplaced", "index": 2 }
        ],
        "destination": [
            { "id": "hidden", "dst": [{ "x": 900, "y": -400, "w": 100, "h": 500 }] },
            { "id": "lift", "dst": [{ "x": 1050, "y": -400, "w": 100, "h": 500 }] },
            { "id": "left" },
            { "id": "right" }
        ]
    }"#;

    /// The red the document fixture's cover is drawn in.
    const DOCUMENT_COVER_SHADE: u8 = 210;

    /// Decodes the document fixture's three sheets by the name of the file a source resolved to.
    struct DocumentAssets;

    impl SkinAssets for DocumentAssets {
        fn image(&mut self, path: &Path) -> Option<SkinImage> {
            let stem = path.file_stem()?.to_str()?;
            let (width, pixels): (u32, Vec<u8>) = match stem {
                "words" => (DOCUMENT_WORDS, (0..DOCUMENT_WORDS as usize).flat_map(|slot| [0, word_shade(slot).g, 0, 255]).collect()),
                "digits" => (10, (0..10).flat_map(|digit| [digit_shade(digit), 0, 0, 255]).collect()),
                _ => (1, vec![DOCUMENT_COVER_SHADE, 0, 0, 255]),
            };
            SkinImage::new(width, 1, pixels)
        }
    }

    /// The document fixture, written out, loaded and built. Its folder is removed with it.
    struct PlayDocument {
        root: PathBuf,
        screen: SkinScreen,
        canvas: CpuCanvas,
        text: TextContext,
    }

    impl Drop for PlayDocument {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    impl PlayDocument {
        fn new(tag: &str) -> PlayDocument {
            crate::font::use_embedded_fonts_only();
            let root = std::env::temp_dir().join(format!("rbms-render-play-objects-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("the scratch folder is writable");
            for source in ["words.tex", "digits.tex", "cover.tex"] {
                std::fs::write(root.join(source), []).expect("the source file is writable");
            }
            let entry = root.join("play.json");
            std::fs::write(&entry, PLAY_DOCUMENT).expect("the document is writable");

            let user = SkinUserConfig::default();
            let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, Mode::BEAT_7K) };
            let skin = load_skin(&entry, options).expect("the fixture loads");
            let mut canvas = CpuCanvas::new(DOCUMENT_SIZE.0, DOCUMENT_SIZE.1);
            let mut text = TextContext::embedded_only();
            let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut DocumentAssets);
            assert_eq!(screen.warnings(), &[] as &[String], "the fixture builds whole");
            PlayDocument { root, screen, canvas, text }
        }

        /// Draws one frame on a cleared canvas and answers how many objects reached the screen.
        fn draw(&mut self, host: &MapHost, timers: &TimerState, judge: JudgeFrame, now_ms: i64) -> usize {
            let data = FrameData { judge, ..FrameData::default() };
            let frame = SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers, state: host, lua: None, mouse: None, data };
            self.canvas.clear(Color::BLACK);
            let mut ctx = RenderCtx::new(crate::theme::theme(), &mut self.text);
            self.screen.draw(&mut ctx, &mut self.canvas, &frame)
        }
    }

    /// A document's judge and cover records come out of the loader and the screen builder arranged as
    /// the reference arranges them: the counts paired with their words by position and pulled left by
    /// half their places, each pop-up following its own region and its own timer, and each cover given
    /// the offsets of its kind.
    #[test]
    fn a_documents_pop_ups_and_covers_are_built_the_way_the_reference_builds_them() {
        let mut document = PlayDocument::new("built");
        assert_eq!(document.screen.count_of(SkinObjectKind::Judge), 2);
        assert_eq!((document.screen.count_of(SkinObjectKind::HiddenCover), document.screen.count_of(SkinObjectKind::LiftCover)), (1, 1));

        let host = covering(COVER_LIFT, Some(COVER_HIDDEN));
        let mut timers = TimerState::new();
        timers.set_on(JUDGE_TIMER, JUDGED_AT_MS * MICROS_PER_MILLI);
        let judge = JudgeFrame::default().with_region(0, JudgeHit { judgement: 1, combo: 128, at_us: JUDGED_AT_MS * MICROS_PER_MILLI });

        assert_eq!(document.draw(&host, &timers, judge, JUDGED_AT_MS + 100), 3, "both covers and the left pop-up");
        let canvas = &document.canvas;
        let lifted = WORD_RECT.y + COVER_LIFT;
        let row = row_above(lifted + 5.0);
        let shown = COUNT_PLACE.0 as u32 * 3;
        let word = columns_of(canvas, row, word_shade(1));
        assert_eq!(
            word,
            Some((WORD_RECT.x as u32 - shown / 2, (WORD_RECT.x + WORD_RECT.w) as u32 - shown / 2 - 1)),
            "the great's word, raised by the lift and slid by half its count"
        );
        let (first, read) = {
            let digit_of = |column: u32| (0..10u8).find(|digit| canvas.pixel_at(column, row) == shade(digit_shade(*digit)));
            let first = (0..DOCUMENT_SIZE.0).find(|column| digit_of(*column).is_some());
            (first, first.map(|first| (first..DOCUMENT_SIZE.0).step_by(COUNT_PLACE.0 as usize).map_while(digit_of).collect::<Vec<u8>>()).unwrap_or_default())
        };
        assert_eq!(read, vec![1, 2, 8], "the region's combo, whatever property the value names");
        assert_eq!(
            first,
            Some(WORD_RECT.x as u32 + 74 - shown / 2),
            "centred on the place the document wrote, six places pulled left and three blanks pushed back"
        );

        let foot = COVER_LINE + COVER_LIFT;
        let cover = shade(DOCUMENT_COVER_SHADE);
        assert_eq!(rows_of(canvas, 950, cover), rows_between(foot, foot + COVER_HIDDEN), "the hidden cover follows the lift and the hidden offset");
        assert_eq!(rows_of(canvas, 1100, cover), rows_between(COVER_LINE, foot), "the lift cover follows the lift alone, under a line that stays put");

        timers.set_on(TimerId(47), JUDGED_AT_MS * MICROS_PER_MILLI);
        let both = judge.with_region(1, JudgeHit { judgement: 0, combo: 4, at_us: JUDGED_AT_MS * MICROS_PER_MILLI });
        assert_eq!(document.draw(&host, &timers, both, JUDGED_AT_MS + 100), 4, "the right pop-up joins once its own region is judged");
        assert_eq!(document.canvas.pixel_at(701, row_above(205.0)), word_shade(0), "unshifted and unlifted, as its own destinations say");

        assert_eq!(document.draw(&host, &timers, both, JUDGED_AT_MS + 501), 2, "and both are gone once their timers have played out");
        assert_eq!(document.draw(&covering(0.0, None), &timers, both, JUDGED_AT_MS + 501), 0, "as are the covers with no lift and no hidden modifier");
    }
}
