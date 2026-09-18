//! Unit tests for the play screen's own document objects: the note field, the gauge, the judgement
//! pop-up and the lane covers.
//!
//! Each one is drawn onto a [`CpuCanvas`] through the very dispatch a real frame goes through, so
//! what is asserted is the pixel a screen would have shown. The sources are one-pixel textures of a
//! colour per part, which is what lets a test name the cell that reached the screen rather than
//! recognising a picture.

use rbms_model::{LnKind, Mode, Note, NoteKind, TimeLine};
use rbms_skin::dst::{Acc, DestinationTrack, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinOffset, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::SkinLayer;
use rbms_skin::property::generated::{FLOAT_GROOVEGAUGE_1P, NUMBER_COMBO, OPTION_1P_PERFECT};
use rbms_skin::property::{SkinStateSource, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::TimerState;

use super::covers::{CoverBand, CoverBody};
use super::draw::draw_object;
use super::gauge::{GaugeAnimation, GaugeBody, SLOT_BELOW_BORDER, SLOT_LEADING, SLOT_UNLIT, SLOTS_PER_GAUGE};
use super::judge::JudgeBody;
use super::notes::BarLine;
use super::notes::{NoteBody, NoteLane};
use super::object::{Body, DigitLayout, ImageBody, NumberBody, SkinObject, Sprite, ValueSource};
use super::state::{
    OPTION_ROW_FOCUSED_FIRST, PLAY_TEXT_TARGET_DELTA, PLAY_TEXT_TARGET_NAME, RESULT_TEXT_HINT, RESULT_TEXT_IR, RESULT_TEXT_TARGET, SELECT_RECORD_FIRST,
    SELECT_TEXT_HINT,
};
use super::{FrameExtra, PlayObjectState, SkinFrame, SkinViewport};
use crate::ctx::with_render_ctx;
use crate::playfield::{LaneShade, PlayfieldView, render_playfield_view};
use crate::skin::{Skin, SkinConfig};
use crate::theme::OPTIONS_ROW_COUNT;
use crate::{Color, CpuCanvas, Renderer, TextureId};

/// The canvas every test draws on, which is also the size the documents are authored at so the
/// viewport maps one to one and a document pixel is a screen pixel.
const CANVAS: (u32, u32) = (1280, 720);

/// The lane the note tests put their chart in.
const TEST_LANE: usize = 1;

/// How fast the test chart scrolls, chosen so its one note lands in the middle of the field.
const TEST_HISPEED: f64 = 0.5;

/// When the test chart's note arrives.
const NOTE_TIME_US: i64 = 1_000_000;

/// The gauge the DoD names: not quite three quarters full.
const TEST_GAUGE: f32 = 0.74;

/// How many parts that gauge is cut into.
const TEST_PARTS: i32 = 50;

/// How tall the bar line the note tests draw is, in document pixels.
const BAR_HEIGHT: f32 = 2.0;

/// The lift the cover test plays with, chosen so the raised judgement line lands on a whole pixel.
const TEST_LIFT: f32 = 0.25;

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
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, acc: Acc::default(), color, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

/// A destination that holds one rectangle still, fully opaque.
fn still(rect: SkinRect) -> DestinationTrack {
    held(rect, SkinColor::rgba(255, 255, 255, 255))
}

/// One draw-list entry over a body and the rectangle its destination holds.
fn object(id: &str, rect: SkinRect, body: Body) -> SkinObject {
    SkinObject { id: id.to_owned(), layer: SkinLayer::Foreground, track: still(rect), stretch: StretchKind::from_id(-1), body }
}

/// The game state the play objects read: a gauge, a judgement and a combo, and nothing else.
#[derive(Debug, Default, Clone, Copy)]
struct PlayState {
    gauge: f32,
    judge: Option<usize>,
    combo: i32,
}

impl OffsetSource for PlayState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for PlayState {
    fn boolean(&self, id: i32) -> bool {
        self.judge.is_some_and(|judge| id == OPTION_1P_PERFECT + judge as i32)
    }
}

impl SkinStateSource for PlayState {
    fn integer(&self, id: i32) -> i32 {
        if id == NUMBER_COMBO { self.combo } else { UNMAPPED_INTEGER }
    }

    fn float(&self, id: i32) -> f32 {
        if id == FLOAT_GROOVEGAUGE_1P { self.gauge } else { UNMAPPED_FLOAT }
    }

    fn string(&self, _id: i32) -> &str {
        UNMAPPED_STRING
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        0
    }
}

/// Draws one object on a canvas that already holds its textures.
fn draw_on(canvas: &mut CpuCanvas, object: &SkinObject, state: &PlayState, extra: FrameExtra<'_>) -> bool {
    let timers = TimerState::new();
    let frame = SkinFrame { now_ms: 0, timers: &timers, state, lua: None, mouse: None, background: None, extra };
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

/// A chart with one note of each kind: a plain note and a mine in their own lanes, and a long note
/// spanning the two timelines in a third.
fn test_chart() -> Vec<TimeLine> {
    let lanes = Mode::BEAT_7K.key;
    let mut first = TimeLine::empty(lanes, 0, 0.0, 120.0);
    let mut second = TimeLine::empty(lanes, NOTE_TIME_US, 1.0, 120.0);
    second.section_line = true;
    second.notes[TEST_LANE] = Some(Note::normal(0, NOTE_TIME_US, 1.0));
    second.notes[TEST_LANE + 1] = Some(Note { kind: NoteKind::Mine { damage: 1.0 }, ..Note::normal(0, NOTE_TIME_US, 1.0) });
    first.notes[TEST_LANE + 2] = Some(Note { kind: NoteKind::LongStart { ln: LnKind::Ln }, ..Note::normal(0, 0, 0.0) });
    second.notes[TEST_LANE + 2] = Some(Note { kind: NoteKind::LongEnd { ln: LnKind::Ln }, ..Note::normal(0, NOTE_TIME_US, 1.0) });
    vec![first, second]
}

/// The document note field the chart above is drawn with: every lane on the resolved skin's own
/// rectangle, so the two can be compared column for column.
fn test_note_body(canvas: &mut CpuCanvas, field: &Skin, colors: [Color; 4]) -> NoteBody {
    let [note, mine, body, hidden] = colors;
    let lanes = (0..field.lane_count())
        .map(|lane| NoteLane {
            rect: SkinRect::new(field.x[lane].round(), 0.0, field.w[lane].round(), field.lane_height()),
            height: field.note_height,
            note: Some(solid(canvas, &format!("note{lane}"), note)),
            ln_start: None,
            ln_end: None,
            ln_body: Some(solid(canvas, &format!("body{lane}"), body)),
            ln_body_active: None,
            mine: Some(solid(canvas, &format!("mine{lane}"), mine)),
            hidden: Some(solid(canvas, &format!("hidden{lane}"), hidden)),
        })
        .collect();
    NoteBody { lanes, bars: Vec::new(), expansion: (1.0, 1.0) }
}

/// The column, in canvas pixels, running down the middle of one lane.
fn lane_column(field: &Skin, lane: usize) -> u32 {
    (field.x[lane] + field.w[lane] / 2.0).round() as u32
}

/// A bar line's own rectangle: as wide as the first lane and standing with its foot on the judgement
/// line, which is where a document that wants its lines to ride the chart puts them.
fn bar_rect(field: &Skin) -> SkinRect {
    SkinRect::new(field.x[0].round(), CANVAS.1 as f32 - field.judge_y, field.w[0].round(), BAR_HEIGHT)
}

/// A note field drawing nothing but one bar line over the destination given.
fn bar_only(canvas: &mut CpuCanvas, field: &Skin, track: DestinationTrack, color: Color) -> NoteBody {
    let mut body = test_note_body(canvas, field, [shade(200), shade(160), shade(120), shade(80)]);
    for lane in &mut body.lanes {
        lane.note = None;
        lane.mine = None;
        lane.ln_body = None;
        lane.hidden = None;
    }
    body.bars = vec![BarLine { track, sprite: solid(canvas, "bar", color) }];
    body
}

#[test]
fn the_play_and_hint_ids_sit_past_every_option_row() {
    let last_row = OPTION_ROW_FOCUSED_FIRST + OPTIONS_ROW_COUNT as i32 - 1;
    for id in [PLAY_TEXT_TARGET_NAME, PLAY_TEXT_TARGET_DELTA, SELECT_TEXT_HINT, RESULT_TEXT_HINT, RESULT_TEXT_IR, SELECT_RECORD_FIRST] {
        assert!(id > last_row, "{id} collides with the option panel's rows");
        assert!(id > RESULT_TEXT_TARGET, "{id} collides with the score screen's text band");
    }
}

#[test]
fn a_document_note_lands_on_the_row_the_built_in_field_puts_it_on() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };

    let mut native = CpuCanvas::new(CANVAS.0, CANVAS.1);
    render_playfield_view(&mut native, &field, &view);

    let mut document = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let colors = [shade(200), shade(160), shade(120), shade(80)];
    let body = test_note_body(&mut document, &field, colors);
    let note = object("note", SkinRect::new(0.0, 0.0, CANVAS.0 as f32, CANVAS.1 as f32), Body::Note(body));
    let drawn = draw_on(&mut document, &note, &PlayState::default(), FrameExtra::Play(&play));

    assert!(drawn, "the field drew nothing at all");
    for (lane, expected, actual) in [(TEST_LANE, field.note_color(TEST_LANE), colors[0]), (TEST_LANE + 1, field.mine_color, colors[1])] {
        let column = lane_column(&field, lane);
        let native_row = first_row_of(&native, column, expected).expect("the built-in field drew no note in this lane");
        let document_row = first_row_of(&document, column, actual).expect("the document drew no note in this lane");
        assert_eq!(document_row, native_row, "lane {lane} is drawn on a different row than the built-in field draws it");
    }
}

#[test]
fn a_long_note_body_runs_between_its_own_head_and_tail() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };

    let mut document = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let colors = [shade(200), shade(160), shade(120), shade(80)];
    let body = test_note_body(&mut document, &field, colors);
    let note = object("note", SkinRect::new(0.0, 0.0, CANVAS.0 as f32, CANVAS.1 as f32), Body::Note(body));
    draw_on(&mut document, &note, &PlayState::default(), FrameExtra::Play(&play));

    let column = lane_column(&field, TEST_LANE + 2);
    let tail = rbms_chart::scroll::visible_offsets(&timelines, 0, TEST_HISPEED, field.lane_height());
    let tail_offset = tail.iter().find(|(index, _)| *index == 1).map(|(_, offset)| *offset).expect("the tail is outside the visible window");

    let top = first_row_of(&document, column, colors[2]).expect("the long note drew no body");
    let bottom = last_row_of(&document, column, colors[2]).expect("the long note drew no body");
    assert_eq!(top, (field.judge_y - tail_offset) as u32, "the body starts where the tail is, which is where the built-in field starts it");
    assert!(bottom + 1 >= field.judge_y as u32 - 1, "the body should run down to the judgement line its head has already reached");
}

#[test]
fn a_bar_line_is_drawn_on_the_section_line_it_belongs_to() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };

    let mut document = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let line = shade(90);
    let body = bar_only(&mut document, &field, still(bar_rect(&field)), line);
    let note = object("note", SkinRect::new(0.0, 0.0, CANVAS.0 as f32, CANVAS.1 as f32), Body::Note(body));
    draw_on(&mut document, &note, &PlayState::default(), FrameExtra::Play(&play));

    let offsets = rbms_chart::scroll::visible_offsets(&timelines, 0, TEST_HISPEED, field.lane_height());
    let at = offsets.iter().find(|(index, _)| *index == 1).map(|(_, offset)| *offset).expect("the section line is outside the visible window");
    let row = first_row_of(&document, lane_column(&field, 0), line).expect("no bar line reached the screen");
    assert_eq!(row, (field.judge_y - at - BAR_HEIGHT) as u32, "the bar line sits on the row its own timeline scrolled to");
}

/// A bar line goes through its own destination like any other object, so a document that faded one
/// out -- which is what an `op` it gated the line on resolves to when the option does not hold --
/// draws no line at all rather than an opaque one.
#[test]
fn a_bar_line_the_document_faded_out_draws_nothing() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };

    let mut document = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let line = shade(90);
    let faded = held(bar_rect(&field), SkinColor::rgba(255, 255, 255, 0));
    let body = bar_only(&mut document, &field, faded, line);
    let note = object("note", SkinRect::new(0.0, 0.0, CANVAS.0 as f32, CANVAS.1 as f32), Body::Note(body));

    assert!(!draw_on(&mut document, &note, &PlayState::default(), FrameExtra::Play(&play)), "a faded line leaves the field empty");
    assert_eq!(first_row_of(&document, lane_column(&field, 0), line), None, "and nothing in its colour reaches the screen");
}

/// A gauge over a table of thirty-six distinct nodes, so a drawn part names the cell it read.
fn distinct_gauge(canvas: &mut CpuCanvas) -> GaugeBody {
    let nodes: Vec<Sprite> = (0..36).map(|slot| solid(canvas, &format!("node{slot}"), shade(slot as u8 + 1))).collect();
    let mut slots = [None; 36];
    for (slot, entry) in slots.iter_mut().enumerate() {
        *entry = Some(slot as u8);
    }
    GaugeBody { nodes, slots, parts: TEST_PARTS, animation: GaugeAnimation::Scatter, range: 0, cycle: 0 }
}

/// The part of a gauge drawn `part` places along a bar of `rect`, as a canvas column.
fn part_column(rect: SkinRect, part: i32) -> u32 {
    (rect.x + rect.w * (part as f32 - 0.5) / TEST_PARTS as f32) as u32
}

#[test]
fn a_gauge_lights_one_part_for_every_part_of_its_value() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };

    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let bar = object("gauge", rect, Body::Gauge(distinct_gauge(&mut canvas)));
    let state = PlayState { gauge: TEST_GAUGE, ..PlayState::default() };
    assert!(draw_on(&mut canvas, &bar, &state, FrameExtra::Play(&play)), "the gauge drew nothing at all");

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
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let rect = SkinRect::new(100.0, 100.0, 500.0, 20.0);
    let row = (CANVAS.1 as f32 - (rect.y + rect.h / 2.0)) as u32;
    let state = PlayState { gauge: TEST_GAUGE, ..PlayState::default() };

    let mut seen = Vec::new();
    for kind in 0..6 {
        let play =
            PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: kind, bomb: &[], keys_down: &[], recent_hits: &[] };
        let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
        let bar = object("gauge", rect, Body::Gauge(distinct_gauge(&mut canvas)));
        draw_on(&mut canvas, &bar, &state, FrameExtra::Play(&play));
        seen.push(canvas.pixel_at(part_column(rect, 1), row).r);
    }
    seen.dedup();
    assert_eq!(seen.len(), 6, "each kind of gauge should read a column of the table of its own");
}

/// A hidden cover reports the band the hidden modifier takes off the field, which the reference
/// numbers as its own offset and rbms measures as a share of the field.
#[test]
fn a_hidden_cover_takes_the_share_of_the_field_the_player_set() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState {
        field: &field,
        playfield: &view,
        shade: LaneShade { cover: 0.0, hidden: 0.3 },
        gauge_kind: 0,
        bomb: &[],
        keys_down: &[],
        recent_hits: &[],
    };

    let rect = SkinRect::new(200.0, 100.0, 300.0, 500.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let color = shade(210);
    let body = CoverBody { sprite: solid(&mut canvas, "cover", color), band: CoverBand::FromJudgement, disappear_line: -1.0, follows_lift: false };
    let cover = object("cover", rect, Body::HiddenCover(body));
    assert!(draw_on(&mut canvas, &cover, &PlayState::default(), FrameExtra::Play(&play)), "the cover drew nothing at all");

    let column = (rect.x + rect.w / 2.0) as u32;
    let foot = (CANVAS.1 as f32 - rect.y) as u32 - 1;
    let covered = (rect.h * 0.3) as u32;
    assert_eq!(last_row_of(&canvas, column, color), Some(foot), "the band stands on the foot of the rectangle the document gave it");
    assert_eq!(first_row_of(&canvas, column, color), Some(foot + 1 - covered), "and rises three tenths of that rectangle and no further");
}

#[test]
fn a_cover_the_player_has_not_asked_for_leaves_the_field_alone() {
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = PlayObjectState { field: &field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };
    let rect = SkinRect::new(200.0, 100.0, 300.0, 500.0);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let body = CoverBody { sprite: solid(&mut canvas, "cover", shade(210)), band: CoverBand::FromJudgement, disappear_line: -1.0, follows_lift: false };
    let cover = object("cover", rect, Body::HiddenCover(body));
    assert!(!draw_on(&mut canvas, &cover, &PlayState::default(), FrameExtra::Play(&play)), "a cover of no height should draw nothing");
}

/// A lift cover covers what the lift has left below the judgement line, so it is empty on a field
/// with no lift and exactly as tall as the lift on one that has it.
#[test]
fn a_lift_cover_follows_the_judgement_line_the_lift_raised() {
    let plain = Skin::default_for(Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let lifted = Skin::build(&SkinConfig { lift: TEST_LIFT, ..SkinConfig::default() }, Mode::BEAT_7K, CANVAS.0 as f32, CANVAS.1 as f32);
    let timelines = test_chart();
    let view = PlayfieldView { timelines: &timelines, microtime: 0, hispeed: TEST_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };

    let rect = SkinRect::new(200.0, CANVAS.1 as f32 - plain.judge_y, 300.0, plain.lane_height());
    let color = shade(190);
    let cover_over = |field: &Skin, canvas: &mut CpuCanvas| {
        let play = PlayObjectState { field, playfield: &view, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &[] };
        let body = CoverBody { sprite: solid(canvas, "lift", color), band: CoverBand::BelowJudgement, disappear_line: -1.0, follows_lift: false };
        let cover = object("cover", rect, Body::LiftCover(body));
        draw_on(canvas, &cover, &PlayState::default(), FrameExtra::Play(&play))
    };

    let mut flat = CpuCanvas::new(CANVAS.0, CANVAS.1);
    assert!(!cover_over(&plain, &mut flat), "with no lift there is nothing below the judgement line to cover");

    let mut raised = CpuCanvas::new(CANVAS.0, CANVAS.1);
    assert!(cover_over(&lifted, &mut raised), "the lift cover drew nothing at all");
    let column = (rect.x + rect.w / 2.0) as u32;
    assert_eq!(first_row_of(&raised, column, color), Some(lifted.judge_y as u32), "the band starts at the judgement line the lift raised");
    assert_eq!(last_row_of(&raised, column, color), Some(plain.judge_y as u32 - 1), "and runs down to where that line would have been");
}

/// Where the pop-up's word sits, in the document's own space.
const WORD_RECT: SkinRect = SkinRect::new(100.0, 200.0, 40.0, 20.0);

/// Where its combo sits, stated against the word's own corner so the two do not overlap.
const COMBO_RECT: SkinRect = SkinRect::new(40.0, -20.0, 8.0, 12.0);

/// How many places that combo reserves, which is what pulls the run half that width to the left.
const COMBO_DIGITS: u32 = 3;

/// Where one place of the combo lands, counting places from its own left edge.
fn combo_column(place: f32) -> u32 {
    (WORD_RECT.x + COMBO_RECT.x - COMBO_RECT.w * COMBO_DIGITS as f32 / 2.0 + COMBO_RECT.w * place) as u32
}

/// A pop-up whose word is a flat colour and whose combo is the colour-coded digit strip.
fn test_judge(canvas: &mut CpuCanvas, word: Color, shift: bool) -> JudgeBody {
    let images = (0..6)
        .map(|index| {
            let sprite = solid(canvas, &format!("word{index}"), word);
            Some(object("word", WORD_RECT, Body::Image(ImageBody { variants: vec![(sprite, 0, 1)], select: ValueSource::None })))
        })
        .collect();
    let strip = digits(canvas);
    let numbers = (0..6)
        .map(|_| {
            let body = NumberBody {
                sprite: strip,
                layout: DigitLayout::integer(10),
                digits: COMBO_DIGITS,
                zero_padding: 0,
                space: 0.0,
                align: 0,
                value: ValueSource::Id(NUMBER_COMBO),
                offsets: Vec::new(),
            };
            Some(object("combo", COMBO_RECT, Body::Number(body)))
        })
        .collect();
    JudgeBody { images, numbers, shift, player: 0 }
}

#[test]
fn a_pop_up_shows_the_judgement_it_was_given_and_the_combo_beside_it() {
    let word = shade(240);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let pop_up = object("judge", SkinRect::new(0.0, 0.0, 1.0, 1.0), Body::Judge(test_judge(&mut canvas, word, false)));
    let state = PlayState { judge: Some(1), combo: 12, gauge: 0.0 };
    assert!(draw_on(&mut canvas, &pop_up, &state, FrameExtra::None), "the pop-up drew nothing at all");

    let word_row = (CANVAS.1 as f32 - (WORD_RECT.y + WORD_RECT.h)) as u32;
    assert_eq!(canvas.pixel_at(120, word_row + 10), word, "the judgement's word should be drawn where its destination put it");

    let digit_row = (CANVAS.1 as f32 - (WORD_RECT.y + COMBO_RECT.y + COMBO_RECT.h)) as u32 + 6;
    assert_eq!(canvas.pixel_at(combo_column(1.5), digit_row), shade(digit_shade(1)), "the combo's tens place should read a one");
    assert_eq!(canvas.pixel_at(combo_column(2.5), digit_row), shade(digit_shade(2)), "the combo's units place should read a two");
    let right_of_anchor = (WORD_RECT.x + COMBO_RECT.x + COMBO_RECT.w * 2.5) as u32;
    assert_ne!(
        canvas.pixel_at(right_of_anchor, digit_row),
        shade(digit_shade(2)),
        "the run is centred on the anchor the document gave it rather than growing off to its right"
    );
}

#[test]
fn a_pop_up_with_nothing_to_report_draws_nothing() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let pop_up = object("judge", SkinRect::new(0.0, 0.0, 1.0, 1.0), Body::Judge(test_judge(&mut canvas, shade(240), false)));
    assert!(!draw_on(&mut canvas, &pop_up, &PlayState::default(), FrameExtra::None), "a run with no judgement yet should draw no pop-up");
}

#[test]
fn a_shifting_pop_up_slides_left_by_half_its_combo() {
    let word = shade(240);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let pop_up = object("judge", SkinRect::new(0.0, 0.0, 1.0, 1.0), Body::Judge(test_judge(&mut canvas, word, true)));
    let state = PlayState { judge: Some(0), combo: 12, gauge: 0.0 };
    draw_on(&mut canvas, &pop_up, &state, FrameExtra::None);

    let row = (CANVAS.1 as f32 - (WORD_RECT.y + WORD_RECT.h)) as u32 + 10;
    let left = (0..CANVAS.0).find(|column| canvas.pixel_at(*column, row) == word).expect("the word never reached the screen");
    assert_eq!(left, (WORD_RECT.x - COMBO_RECT.w) as u32, "two places of eight pixels slide the word left by one place");
}
