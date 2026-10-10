//! Tests of the note field, drawn onto a [`CpuCanvas`] through the dispatch a real frame goes
//! through.
//!
//! Every chart here is written by hand at 120 beats a minute, where a measure lasts two seconds, and
//! every expected row is worked out by hand from the reference's own expressions. Rows are stated
//! the way the reference states them, counted up from the foot of the screen, so a note said to be
//! at 200 has its foot 200 pixels above the bottom edge.

use std::borrow::Cow;

use rbms_model::{LnKind, Note, NoteKind, TimeLine};
use rbms_skin::dst::{DestinationTrack, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinOffset, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::NoteSet;
use rbms_skin::property::{SkinHost, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState};

use super::super::draw::draw_object;
use super::super::object::{Body, SkinObject, Sprite};
use super::super::{FrameData, SkinFrame, SkinViewport};
use super::{
    ConstantScroll, FieldLine, HELL_BODY_DRAINING, HELL_BODY_GAINING, HELL_BODY_HELD, HELL_BODY_IDLE, HELL_END, HELL_START, JudgeArea, LONG_BODY_HELD,
    LONG_BODY_IDLE, LONG_END, LONG_IMAGES, LONG_START, LaneLong, LaneNotes, NoteBody, NoteDisplay, NoteImage, NoteLane, NoteStates, PLACEHOLDER_HIDDEN,
    PLACEHOLDER_PROCESSED, current_duration_ms, fixed_hispeed, lane_offsets, long_names,
};
use crate::ctx::with_render_ctx;
use crate::{Color, CpuCanvas, Renderer};

/// The canvas every test draws on.
const CANVAS: (u32, u32) = (1280, 720);

/// Where the four lanes of the test field sit. The third is shorter and higher than the others,
/// which nothing drawn may follow.
const LANES: [SkinRect; 4] = [
    SkinRect::new(100.0, 100.0, 40.0, 400.0),
    SkinRect::new(150.0, 100.0, 40.0, 400.0),
    SkinRect::new(200.0, 160.0, 40.0, 300.0),
    SkinRect::new(250.0, 100.0, 40.0, 400.0),
];

/// How tall a note of the test field is.
const NOTE_H: f32 = 10.0;

/// The foot of lane zero, which is the judgement line of a field with no lift.
const LINE: u32 = 100;

/// The tempo of every test chart, at which a measure lasts two seconds.
const BPM: f64 = 120.0;

/// The offset the test note object names.
const NUDGE_OFFSET: i32 = 30;

/// Where the test field's lines are drawn from: across the first two lanes, on the judgement line.
const LINE_RECT: SkinRect = SkinRect::new(100.0, 100.0, 90.0, 2.0);

/// The reds the parts of the test field are drawn in.
const NOTE_RED: u8 = 200;
const MINE_RED: u8 = 60;
const LONG_RED_FIRST: u8 = 100;
const LONG_RED_STEP: u8 = 10;
const SECTION_RED: u8 = 20;
const BPM_RED: u8 = 30;
const STOP_RED: u8 = 40;

/// A red that names one part.
fn shade(value: u8) -> Color {
    Color { r: value, g: 0, b: 0, a: 255 }
}

/// The red a long-note image is drawn in, by its place among the ten.
fn long_shade(slot: usize) -> Color {
    shade(LONG_RED_FIRST + LONG_RED_STEP * slot as u8)
}

/// A one-pixel sprite of one colour.
fn solid(canvas: &mut CpuCanvas, key: &str, color: Color) -> Sprite {
    let tex = canvas.register_texture(key, &[color.r, color.g, color.b, color.a], 1, 1);
    Sprite { tex, size: (1, 1), origin: (0, 0), cell: (1, 1), columns: 1, rows: 1, timer: None, cycle: 0 }
}

/// A destination that holds one rectangle still and opaque.
fn still(rect: SkinRect) -> DestinationTrack {
    let color = SkinColor::rgba(255, 255, 255, 255);
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

/// The test field over `rects`: every kind of note in a colour of its own, the hidden and judged
/// notes as the reference's own placeholders, and no lines.
fn field_over(canvas: &mut CpuCanvas, rects: &[SkinRect], note_height: f32) -> NoteBody {
    let lanes = rects
        .iter()
        .enumerate()
        .map(|(lane, rect)| NoteLane {
            note: NoteImage::Sprite(solid(canvas, &format!("note{lane}"), shade(NOTE_RED))),
            long: std::array::from_fn(|slot| NoteImage::Sprite(solid(canvas, &format!("long{lane}.{slot}"), long_shade(slot)))),
            mine: NoteImage::Sprite(solid(canvas, &format!("mine{lane}"), shade(MINE_RED))),
            ..NoteLane::placeholder(*rect, note_height)
        })
        .collect();
    NoteBody::new(lanes)
}

/// The test field on [`LANES`].
fn field(canvas: &mut CpuCanvas) -> NoteBody {
    field_over(canvas, &LANES, NOTE_H)
}

/// One line of the test field in the red given.
fn line(canvas: &mut CpuCanvas, key: &str, red: u8) -> FieldLine {
    FieldLine::new(still(LINE_RECT), solid(canvas, key, shade(red)))
}

/// The note object over a field, naming the one offset the tests nudge it with.
fn note_object(body: NoteBody) -> SkinObject {
    let track = DestinationTrack { offsets: vec![NUDGE_OFFSET], ..still(SkinRect::default()) };
    SkinObject { stretch: StretchKind::from_id(track.stretch), track, body: Body::Note(body) }
}

/// A game that answers nothing but one offset.
#[derive(Debug, Default, Clone, Copy)]
struct Host {
    nudge: Option<SkinOffset>,
}

impl OffsetSource for Host {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.nudge.filter(|_| id == NUDGE_OFFSET)
    }
}

impl DrawStateSource for Host {
    fn boolean(&self, _id: i32) -> Option<bool> {
        Some(false)
    }
}

impl SkinHost for Host {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
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

/// What a run did to some of its notes, as `(lane, head time in milliseconds, state)`.
struct Judged(&'static [(usize, i64, u8)]);

impl NoteStates for Judged {
    fn state(&self, lane: usize, time_us: i64) -> u8 {
        self.0.iter().find(|(at, ms, _)| *at == lane && ms * MICROS_PER_MILLI == time_us).map_or(0, |(_, _, state)| *state)
    }
}

/// A timeline `ms` milliseconds and `section` measures into a chart at the test tempo.
fn at(ms: i64, section: f64) -> TimeLine {
    TimeLine::empty(LANES.len(), ms * MICROS_PER_MILLI, section, BPM)
}

/// The same timeline with a note of `kind` in `lane`.
fn with(mut timeline: TimeLine, lane: usize, kind: NoteKind) -> TimeLine {
    timeline.notes[lane] = Some(Note { kind, ..Note::normal(0, timeline.time_us, timeline.section) });
    timeline
}

/// A timeline with one plain note.
fn note_at(ms: i64, section: f64, lane: usize) -> TimeLine {
    with(at(ms, section), lane, NoteKind::Normal)
}

/// The chart most tests draw: its first timeline at the start, then whatever the test adds.
fn chart(rest: Vec<TimeLine>) -> Vec<TimeLine> {
    let mut start = at(0, 0.0);
    start.section_line = true;
    std::iter::once(start).chain(rest).collect()
}

/// A run of `timelines` with the play head `ms` milliseconds in.
fn playing(timelines: &[TimeLine], ms: i64) -> LaneNotes<'_> {
    LaneNotes::new(timelines, ms * MICROS_PER_MILLI, BPM)
}

/// Draws a field on a canvas that already holds its textures, answering whether anything was drawn.
fn draw(canvas: &mut CpuCanvas, object: &SkinObject, notes: &LaneNotes<'_>) -> bool {
    draw_with(canvas, object, notes, &Host::default(), (CANVAS.0 as f32, CANVAS.1 as f32))
}

/// The same against a host and for a document authored at `authored`.
fn draw_with(canvas: &mut CpuCanvas, object: &SkinObject, notes: &LaneNotes<'_>, host: &Host, authored: (f32, f32)) -> bool {
    canvas.clear(Color::BLACK);
    let timers = TimerState::new();
    let data = FrameData { notes: Some(notes), ..FrameData::default() };
    let frame = SkinFrame { now_us: 0, timers: &timers, state: host, lua: None, mouse: None, data };
    let viewport = SkinViewport::new(authored, (CANVAS.0 as f32, CANVAS.1 as f32));
    with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
}

/// The rows of `column` holding `color`, as the lowest one and the one above the highest, both
/// counted up from the foot of the canvas. `None` when the colour is nowhere in the column.
fn span(canvas: &CpuCanvas, column: u32, color: Color) -> Option<(u32, u32)> {
    let rows: Vec<u32> = (0..CANVAS.1).filter(|row| canvas.pixel_at(column, *row) == color).collect();
    Some((CANVAS.1 - 1 - rows.last()?, CANVAS.1 - rows.first()?))
}

/// The pixel `up` rows above the foot of the canvas in `column`.
fn pixel(canvas: &CpuCanvas, column: u32, up: u32) -> Color {
    canvas.pixel_at(column, CANVAS.1 - 1 - up)
}

/// The column running down the middle of one lane.
fn middle(lane: usize) -> u32 {
    (LANES[lane].x + LANES[lane].w / 2.0) as u32
}

/// Where the plain notes of `lane` are, drawn fresh from a chart.
fn note_span(notes: &LaneNotes<'_>, lane: usize) -> Option<(u32, u32)> {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    draw(&mut canvas, &object, notes);
    span(&canvas, middle(lane), shade(NOTE_RED))
}

/// A note half a measure in is a quarter of the field up when a quarter of a measure is still to
/// come: `0.5 * (1000 - 500) / 1000 * 400`.
#[test]
fn a_note_is_as_far_up_the_field_as_the_share_of_its_measure_still_to_come() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    assert_eq!(note_span(&playing(&timelines, 500), 0), Some((200, 210)));
    assert_eq!(note_span(&playing(&timelines, 250), 0), Some((250, 260)));
    assert_eq!(note_span(&playing(&timelines, 1_000), 0), Some((100, 110)), "at its own moment it stands on the line");
    assert_eq!(note_span(&playing(&timelines, 1_001), 0), None, "and a millisecond later it is gone");
}

#[test]
fn the_scroll_speed_and_the_charts_own_speed_both_stretch_the_field() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let fast = LaneNotes { hispeed: 2.0, ..playing(&timelines, 500) };
    assert_eq!(note_span(&fast, 0), Some((300, 310)));
    let slowed = LaneNotes { hispeed: 2.0, speed: 0.5, ..playing(&timelines, 500) };
    assert_eq!(note_span(&slowed, 0), Some((200, 210)));
}

/// A measure is as tall after a tempo change as before it: the section decides the height and the
/// tempo only how soon it is covered.
#[test]
fn a_tempo_change_moves_the_moment_and_not_the_height_of_a_section() {
    let mut faster = at(1_000, 0.5);
    faster.bpm = 240.0;
    let mut note = note_at(1_500, 1.0, 0);
    note.bpm = 240.0;
    let timelines = chart(vec![faster, note]);
    assert_eq!(note_span(&playing(&timelines, 500), 0), Some((400, 410)), "100 for the half measure in hand and 200 for the next");
    assert_eq!(note_span(&playing(&timelines, 1_250), 0), Some((200, 210)), "half of the faster half measure is left");
}

/// While the chart stands still the note above the stop keeps its whole distance, and the stop is
/// taken out of the time it then has to cover it in.
#[test]
fn a_stop_holds_the_notes_above_it_still() {
    let mut stop = at(1_000, 0.5);
    stop.stop_us = 1_000 * MICROS_PER_MILLI;
    let timelines = chart(vec![stop, note_at(3_000, 1.0, 0)]);
    assert_eq!(note_span(&playing(&timelines, 500), 0), Some((400, 410)));
    assert_eq!(note_span(&playing(&timelines, 1_500), 0), Some((300, 310)), "half a measure above the line all through the stop");
    assert_eq!(note_span(&playing(&timelines, 1_999), 0), Some((300, 310)));
    assert_eq!(note_span(&playing(&timelines, 2_500), 0), Some((200, 210)), "and half of that once half the second after it has run");
}

#[test]
fn a_scroll_of_nothing_stacks_a_section_and_a_negative_one_runs_it_backwards() {
    let scrolled = |scroll: f64| {
        let mut turn = at(1_000, 0.5);
        turn.scroll = scroll;
        chart(vec![turn, note_at(2_000, 1.0, 0)])
    };
    let stacked = scrolled(0.0);
    assert_eq!(note_span(&playing(&stacked, 500), 0), Some((200, 210)), "on the row of the timeline that set it");
    let backwards = scrolled(-1.0);
    assert_eq!(note_span(&playing(&backwards, 500), 0), Some((0, 10)), "200 down from that row, which is below the line");
}

/// The reference's CONSTANT option draws nothing further ahead than its duration, fades in what is
/// about to cross it, and reads the chart's own speed as one.
#[test]
fn the_constant_option_cuts_the_field_off_at_its_duration_and_fades_the_edge_in() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0), note_at(1_700, 0.85, 1), note_at(2_100, 1.05, 3)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    let constant = Some(ConstantScroll { duration_ms: 1_000, fadein_ms: 500.0 });
    draw(&mut canvas, &object, &LaneNotes { constant, speed: 2.0, ..playing(&timelines, 500) });

    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)), "inside the duration a note is opaque, at the speed of one");
    let fading = pixel(&canvas, middle(1), 345);
    assert!((118..=122).contains(&fading.r), "200 ms past the duration of a 500 ms fade is three fifths there: {fading:?}");
    assert_eq!(pixel(&canvas, middle(1), 339), Color::BLACK, "and it sits 0.35 of a measure above the first");
    assert!((0..CANVAS.1).all(|row| canvas.pixel_at(middle(3), row) == Color::BLACK), "600 ms past it nothing is drawn");

    let early = Some(ConstantScroll { duration_ms: 1_000, fadein_ms: -500.0 });
    let timelines = chart(vec![note_at(1_200, 0.6, 0), note_at(1_700, 0.85, 1)]);
    draw(&mut canvas, &object, &LaneNotes { constant: early, ..playing(&timelines, 500) });
    let fading = pixel(&canvas, middle(0), 245);
    assert!((118..=122).contains(&fading.r), "a negative fade starts before the duration: {fading:?}");
    assert!((0..CANVAS.1).all(|row| canvas.pixel_at(middle(1), row) == Color::BLACK), "and cuts off exactly on it");
}

/// The judgement line and the top of the field are lane zero's, whatever another lane's rectangle
/// says: only its column is its own.
#[test]
fn every_lane_is_measured_against_lane_zero() {
    let timelines = chart(vec![note_at(1_000, 0.5, 2)]);
    assert_eq!(note_span(&playing(&timelines, 500), 2), Some((200, 210)));
    assert_eq!(note_span(&playing(&timelines, 1_000), 2), Some((LINE, LINE + NOTE_H as u32)), "sixty pixels below that lane's own foot");
}

#[test]
fn the_lift_raises_the_line_and_shortens_the_field_above_it() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let lifted = LaneNotes { lift: Some(0.25), ..playing(&timelines, 500) };
    assert_eq!(note_span(&lifted, 0), Some((275, 285)), "a line at 200 and a quarter of the 300 above it");
    let covered = LaneNotes { lanecover: Some(0.5), hidden: Some(0.5), ..playing(&timelines, 500) };
    assert_eq!(note_span(&covered, 0), Some((200, 210)), "the covers only cover: they move no note");
}

/// The first timeline's row is `section * (time - now) / time`, which is no number at all while
/// that timeline is at time zero and the play head has not passed it. The walk ends there, so a
/// chart that has not started shows only what is drawn before a row is asked for.
#[test]
fn a_chart_that_has_not_started_draws_no_note_and_its_first_line_on_the_judgement_line() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let mut body = field(&mut canvas);
    body.section_lines = vec![line(&mut canvas, "section", SECTION_RED)];
    let object = note_object(body);
    draw(&mut canvas, &object, &playing(&timelines, 0));
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), None);
    assert_eq!(span(&canvas, middle(0), shade(SECTION_RED)), Some((100, 102)));
    assert!(!draw(&mut canvas, &object, &playing(&timelines, -20)), "before zero even that line is ahead of its own first keyframe");
    draw(&mut canvas, &object, &playing(&timelines, 500));
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)), "once it has, the notes are there");
}

#[test]
fn each_kind_of_note_is_drawn_from_its_own_image() {
    let both = with(with(at(1_000, 0.5), 0, NoteKind::Normal), 1, NoteKind::Mine { damage: 1.0 });
    let timelines = chart(vec![both]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    assert!(draw(&mut canvas, &object, &playing(&timelines, 500)));
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)));
    assert_eq!(span(&canvas, middle(1), shade(MINE_RED)), Some((200, 210)));

    draw(&mut canvas, &object, &playing(&timelines, 1_500));
    assert_eq!(span(&canvas, middle(1), shade(MINE_RED)), None, "a mine is gone once its moment has passed");
}

/// A note the player hit early is still drawn until its own moment comes, and only the option that
/// marks judged notes changes what it is drawn from.
#[test]
fn a_note_hit_early_stays_on_screen_until_its_moment() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let hit = Judged(&[(0, 1_000, 1)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));

    draw(&mut canvas, &object, &LaneNotes { states: &hit, ..playing(&timelines, 500) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)));

    let marked = NoteDisplay { mark_processed: true, ..NoteDisplay::default() };
    draw(&mut canvas, &object, &LaneNotes { states: &hit, show: marked, ..playing(&timelines, 500) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), None);
    assert_eq!(pixel(&canvas, LANES[0].x as u32, 200), PLACEHOLDER_PROCESSED, "the judged note is the reference's outlined placeholder");
    assert_eq!(pixel(&canvas, middle(0), 205), Color::BLACK, "which is hollow");

    draw(&mut canvas, &object, &LaneNotes { show: marked, ..playing(&timelines, 500) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)), "a note nobody has hit is not marked");
}

#[test]
fn a_note_nobody_hit_waits_on_the_line_only_when_past_notes_are_kept() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0), note_at(4_000, 2.0, 1)]);
    let kept = NoteDisplay { past_notes: true, ..NoteDisplay::default() };
    assert_eq!(note_span(&playing(&timelines, 1_500), 0), None);
    assert_eq!(note_span(&LaneNotes { show: kept, ..playing(&timelines, 1_500) }, 0), Some((100, 110)));
    let missed = Judged(&[(0, 1_000, 5)]);
    assert_eq!(note_span(&LaneNotes { show: kept, states: &missed, ..playing(&timelines, 1_500) }, 0), None, "one that has been judged does not");
}

/// A hidden note is drawn from the outlined placeholder on the lane's own rectangle, untouched by
/// the note object's offsets, and only when the player asked for hidden notes.
#[test]
fn a_hidden_note_is_drawn_only_on_request_and_ignores_the_note_offsets() {
    let mut carrier = at(1_000, 0.5);
    carrier.hidden[1] = Some(Note::normal(0, carrier.time_us, carrier.section));
    let timelines = chart(vec![carrier]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    let host = Host { nudge: Some(SkinOffset { x: 300.0, y: 50.0, ..SkinOffset::default() }) };
    let authored = (CANVAS.0 as f32, CANVAS.1 as f32);

    assert!(!draw_with(&mut canvas, &object, &playing(&timelines, 500), &host, authored), "not asked for, not drawn");
    let shown = NoteDisplay { hidden_notes: true, ..NoteDisplay::default() };
    assert!(draw_with(&mut canvas, &object, &LaneNotes { show: shown, ..playing(&timelines, 500) }, &host, authored));
    assert_eq!(pixel(&canvas, LANES[1].x as u32, 200), PLACEHOLDER_HIDDEN);
    assert_eq!(pixel(&canvas, LANES[1].x as u32 + 39, 209), PLACEHOLDER_HIDDEN);
    assert_eq!(pixel(&canvas, middle(1), 205), Color::BLACK, "the placeholder is a frame with nothing in it");
}

/// The ends of a long note half a measure long whose head is a quarter of the field up.
fn long_chart(start: LnKind, end_ms: i64, end_section: f64) -> Vec<TimeLine> {
    let head = with(at(1_000, 0.5), 0, NoteKind::LongStart { ln: start });
    let tail = with(at(end_ms, end_section), 0, NoteKind::LongEnd { ln: start });
    chart(vec![head, tail])
}

/// Draws a long note of `kind` and answers where its start, body and end images are, the body by
/// its place among the ten.
fn long_spans(notes: &LaneNotes<'_>, start: usize, body: usize, end: usize) -> [Option<(u32, u32)>; 3] {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    draw(&mut canvas, &object, notes);
    [start, body, end].map(|slot| span(&canvas, middle(0), long_shade(slot)))
}

/// The body runs from the top of the start image to the foot of the end, and only a charge note
/// draws an image for its end.
#[test]
fn a_long_note_is_laid_out_from_the_foot_of_its_end() {
    let timelines = long_chart(LnKind::Undefined, 2_000, 1.0);
    let plain = playing(&timelines, 500);
    assert_eq!(long_spans(&plain, LONG_START, LONG_BODY_IDLE, LONG_END), [Some((200, 210)), Some((210, 400)), None]);

    let charge = LaneNotes { ln_mode: LnKind::Cn, ..playing(&timelines, 500) };
    assert_eq!(long_spans(&charge, LONG_START, LONG_BODY_IDLE, LONG_END), [Some((200, 210)), Some((210, 400)), Some((400, 410))]);

    let hell = LaneNotes { ln_mode: LnKind::Hcn, ..playing(&timelines, 500) };
    assert_eq!(long_spans(&hell, HELL_START, HELL_BODY_IDLE, HELL_END), [Some((200, 210)), Some((210, 400)), Some((400, 410))]);

    let stated = long_chart(LnKind::Cn, 2_000, 1.0);
    let overridden = LaneNotes { ln_mode: LnKind::Ln, ..playing(&stated, 500) };
    assert_eq!(long_spans(&overridden, LONG_START, LONG_BODY_IDLE, LONG_END)[2], Some((400, 410)), "a note that states its kind keeps it");
}

/// A note shorter than its own images overlaps them, which shows the order they are drawn in: the
/// body, then the end, then the start over both.
#[test]
fn a_long_notes_start_is_drawn_over_its_end_and_its_body() {
    let timelines = long_chart(LnKind::Cn, 1_020, 0.51);
    assert_eq!(long_spans(&playing(&timelines, 500), LONG_START, LONG_BODY_IDLE, LONG_END), [Some((200, 210)), None, Some((210, 214))]);
    let plain = long_chart(LnKind::Ln, 1_020, 0.51);
    assert_eq!(long_spans(&playing(&plain, 500), LONG_START, LONG_BODY_IDLE, LONG_END), [Some((200, 210)), None, None]);
}

/// Nothing clips a long note. One whose head has passed stops at the line because a timeline in the
/// past is never moved off it.
#[test]
fn a_long_note_whose_head_has_passed_is_anchored_on_the_line() {
    let timelines = long_chart(LnKind::Ln, 2_000, 1.0);
    let held = [LaneLong { processing: Some(1_000 * MICROS_PER_MILLI), ..LaneLong::default() }];
    let notes = LaneNotes { longs: &held, ..playing(&timelines, 1_500) };
    assert_eq!(long_spans(&notes, LONG_START, LONG_BODY_HELD, LONG_END), [Some((100, 110)), Some((110, 200)), None]);
    assert_eq!(long_spans(&playing(&timelines, 2_001), LONG_START, LONG_BODY_IDLE, LONG_END), [None, None, None], "and is gone with its end");
}

/// Which body a long note is drawn with: held or not for a plain or a charge note, and for a hell
/// charge note held, untouched, gaining or draining.
#[test]
fn a_long_notes_body_follows_what_the_player_is_doing_to_it() {
    let head_us = 1_000 * MICROS_PER_MILLI;
    let body_of = |kind: LnKind, long: LaneLong, states: &'static [(usize, i64, u8)], slot: usize| {
        let timelines = long_chart(kind, 2_000, 1.0);
        let longs = [long];
        let judged = Judged(states);
        let notes = LaneNotes { longs: &longs, states: &judged, ..playing(&timelines, 500) };
        long_spans(&notes, LONG_START, slot, LONG_END)[1]
    };
    let whole = Some((210, 400));
    let hit: &[(usize, i64, u8)] = &[(0, 1_000, 2)];
    let held = LaneLong { processing: Some(head_us), ..LaneLong::default() };
    let another = LaneLong { processing: Some(head_us + 1), ..LaneLong::default() };
    assert_eq!(body_of(LnKind::Ln, held, &[], LONG_BODY_HELD), whole);
    assert_eq!(body_of(LnKind::Ln, another, &[], LONG_BODY_IDLE), whole, "holding another note of the lane is not holding this one");
    assert_eq!(body_of(LnKind::Cn, held, &[], LONG_BODY_HELD), whole);

    let gaining = LaneLong { passing: Some(head_us), increasing: true, ..LaneLong::default() };
    let draining = LaneLong { passing: Some(head_us), increasing: false, ..LaneLong::default() };
    assert_eq!(body_of(LnKind::Hcn, held, hit, HELL_BODY_HELD), whole);
    assert_eq!(body_of(LnKind::Hcn, gaining, hit, HELL_BODY_GAINING), whole);
    assert_eq!(body_of(LnKind::Hcn, draining, hit, HELL_BODY_DRAINING), whole);
    assert_eq!(body_of(LnKind::Hcn, gaining, &[], HELL_BODY_IDLE), whole, "a note whose head was never judged is not passing yet");
    assert_eq!(body_of(LnKind::Hcn, LaneLong::default(), hit, HELL_BODY_IDLE), whole);
}

/// The older naming says the opposite of what it does, and it is the one in force whenever the
/// newer active list is left empty.
#[test]
fn the_older_long_note_names_are_read_the_way_the_reference_reads_them() {
    let list = |name: &str| vec![name.to_owned()];
    let older = NoteSet {
        lnend: list("end"),
        lnstart: list("start"),
        lnbody: list("body"),
        lnactive: list("active"),
        hcnend: list("hend"),
        hcnstart: list("hstart"),
        hcnbody: list("hbody"),
        hcnactive: list("hactive"),
        hcndamage: list("hdamage"),
        hcnreactive: list("hreactive"),
        ..NoteSet::default()
    };
    let names = long_names(&older);
    let read: [&str; LONG_IMAGES] = std::array::from_fn(|slot| names[slot][0].as_str());
    assert_eq!(read, ["end", "start", "body", "active", "hend", "hstart", "hbody", "hactive", "hdamage", "hreactive"]);
    assert_eq!((read[LONG_BODY_HELD], read[LONG_BODY_IDLE]), ("body", "active"), "`lnbody` is the body of a note being held");
    assert_eq!((read[HELL_BODY_GAINING], read[HELL_BODY_DRAINING]), ("hdamage", "hreactive"));

    let newer =
        NoteSet { lnbody_active: list("held"), hcnbody_active: list("hheld"), hcnbody_reactive: list("hgaining"), hcnbody_miss: list("hdraining"), ..older };
    let names = long_names(&newer);
    let read: [&str; LONG_IMAGES] = std::array::from_fn(|slot| names[slot][0].as_str());
    assert_eq!(read, ["end", "start", "held", "body", "hend", "hstart", "hheld", "hbody", "hgaining", "hdraining"]);
}

/// A line is drawn from its own destination, moved up by how far its timeline is above the
/// judgement line with the fraction of a pixel dropped.
#[test]
fn a_measure_line_rides_its_timeline_in_whole_pixels() {
    let mut second = at(1_000, 0.5);
    second.section_line = true;
    let timelines = chart(vec![second]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let mut body = field(&mut canvas);
    body.section_lines = vec![line(&mut canvas, "section", SECTION_RED)];
    let object = note_object(body);

    assert!(draw(&mut canvas, &object, &playing(&timelines, 500)));
    assert_eq!(span(&canvas, middle(0), shade(SECTION_RED)), Some((200, 202)));
    assert_eq!(span(&canvas, middle(1), shade(SECTION_RED)), Some((200, 202)), "as wide as its own destination");
    assert_eq!(span(&canvas, middle(3), shade(SECTION_RED)), None);

    draw(&mut canvas, &object, &playing(&timelines, 332));
    assert_eq!(span(&canvas, middle(0), shade(SECTION_RED)), Some((233, 235)), "133.6 above the line is drawn 133 above it");
}

#[test]
fn tempo_and_stop_lines_are_drawn_only_with_the_guide_on() {
    let mut faster = at(1_000, 0.5);
    faster.bpm = 240.0;
    let mut stop = at(1_500, 1.0);
    (stop.bpm, stop.stop_us) = (240.0, 500 * MICROS_PER_MILLI);
    let mut brief = at(2_250, 1.25);
    (brief.bpm, brief.stop_us) = (240.0, MICROS_PER_MILLI - 1);
    let timelines = chart(vec![faster, stop, brief]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let mut body = field(&mut canvas);
    body.bpm_lines = vec![line(&mut canvas, "bpm", BPM_RED)];
    body.stop_lines = vec![line(&mut canvas, "stop", STOP_RED)];
    let object = note_object(body);

    assert!(!draw(&mut canvas, &object, &playing(&timelines, 500)), "with the guide off neither is drawn");
    let guided = NoteDisplay { bpm_guide: true, ..NoteDisplay::default() };
    assert!(draw(&mut canvas, &object, &LaneNotes { show: guided, ..playing(&timelines, 500) }));
    assert_eq!(span(&canvas, middle(0), shade(BPM_RED)), Some((200, 202)), "where the tempo becomes another one, and not where it stays");
    assert_eq!(span(&canvas, middle(0), shade(STOP_RED)), Some((400, 402)), "where the chart stops for a millisecond or more");
}

/// The walk ends with the first timeline past the top of the field, which is itself still drawn.
#[test]
fn nothing_is_drawn_past_the_first_timeline_above_the_field() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0), note_at(3_000, 1.5, 0), note_at(3_500, 1.75, 1)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    draw(&mut canvas, &object, &playing(&timelines, 500));
    assert_eq!(pixel(&canvas, middle(0), 205), shade(NOTE_RED));
    assert_eq!(pixel(&canvas, middle(0), 605), shade(NOTE_RED), "the one that crossed the top at 600 is drawn above it");
    assert_eq!(span(&canvas, middle(1), shade(NOTE_RED)), None, "and the one after it is not walked to");
}

#[test]
fn the_note_objects_offsets_move_and_resize_every_note() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    let host = Host { nudge: Some(SkinOffset { x: 5.0, y: -12.0, w: 10.0, h: 4.0, ..SkinOffset::default() }) };
    draw_with(&mut canvas, &object, &playing(&timelines, 500), &host, (CANVAS.0 as f32, CANVAS.1 as f32));
    assert_eq!(span(&canvas, 105, shade(NOTE_RED)), Some((188, 202)), "twelve lower and four taller");
    assert_eq!(span(&canvas, 154, shade(NOTE_RED)), Some((188, 202)), "five to the right and ten wider");
    assert_eq!((span(&canvas, 104, shade(NOTE_RED)), span(&canvas, 155, shade(NOTE_RED))), (None, None));
}

/// A document authored at half the screen's size is drawn at twice its own numbers, the note height
/// included.
#[test]
fn a_smaller_document_is_scaled_onto_the_screen() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let halved: Vec<SkinRect> = LANES.iter().map(|lane| SkinRect::new(lane.x / 2.0, lane.y / 2.0, lane.w / 2.0, lane.h / 2.0)).collect();
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field_over(&mut canvas, &halved, NOTE_H / 2.0));
    draw_with(&mut canvas, &object, &playing(&timelines, 500), &Host::default(), (CANVAS.0 as f32 / 2.0, CANVAS.1 as f32 / 2.0));
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)));
    assert_eq!((span(&canvas, 99, shade(NOTE_RED)), span(&canvas, 100, shade(NOTE_RED)).is_some(), span(&canvas, 140, shade(NOTE_RED))), (None, true, None));
}

/// The walk is picked up where the last frame left it, and started over when the chart's clock has
/// gone back, as it does when a replay is wound back.
#[test]
fn the_walk_starts_over_when_the_clock_goes_back() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0), note_at(3_000, 1.5, 0), note_at(4_000, 2.0, 1)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    draw(&mut canvas, &object, &playing(&timelines, 3_500));
    assert_eq!(span(&canvas, middle(1), shade(NOTE_RED)), Some((200, 210)));
    draw(&mut canvas, &object, &playing(&timelines, 3_600));
    assert_eq!(span(&canvas, middle(1), shade(NOTE_RED)), Some((180, 190)));
    draw(&mut canvas, &object, &playing(&timelines, 500));
    assert_eq!(pixel(&canvas, middle(0), 205), shade(NOTE_RED), "the notes the later frames had walked past are back");
}

/// On a quarter note a note swells about its lane's middle and then settles back.
#[test]
fn a_note_swells_on_the_quarter_note_when_the_document_gives_a_rate() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let mut body = field(&mut canvas);
    body.expansion = (200, 200);
    let object = note_object(body);

    draw(&mut canvas, &object, &LaneNotes { quarter_note_ms: 9, ..playing(&timelines, 500) });
    assert_eq!(span(&canvas, 80, shade(NOTE_RED)), Some((195, 215)), "twice as tall about its own middle");
    assert_eq!((span(&canvas, 79, shade(NOTE_RED)), span(&canvas, 159, shade(NOTE_RED)).is_some(), span(&canvas, 160, shade(NOTE_RED))), (None, true, None));
    draw(&mut canvas, &object, &LaneNotes { quarter_note_ms: 160, ..playing(&timelines, 500) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((200, 210)), "and at rest again once the 159 ms are over");
}

/// A field with `dst2` drops the notes the player missed: each waits on the line for the late
/// window and then falls at the speed of one down to the row the document names.
#[test]
fn a_field_that_drops_its_missed_notes_lets_them_fall_below_the_line() {
    let timelines = chart(vec![note_at(1_000, 0.5, 0), note_at(4_000, 2.0, 1)]);
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let mut body = field(&mut canvas);
    body.fall_to = Some(0);
    let object = note_object(body);
    let late_window_us = 100 * MICROS_PER_MILLI;

    draw(&mut canvas, &object, &LaneNotes { late_window_us, ..playing(&timelines, 1_050) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((100, 110)), "inside the late window it waits on the line");
    draw(&mut canvas, &object, &LaneNotes { late_window_us, ..playing(&timelines, 1_350) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((50, 60)), "a quarter of a second later it is an eighth of a measure down");
    draw(&mut canvas, &object, &LaneNotes { late_window_us, ..playing(&timelines, 1_600) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), Some((0, 10)));
    draw(&mut canvas, &object, &LaneNotes { late_window_us, ..playing(&timelines, 1_700) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), None, "and past the row it falls to it is gone");

    let hit = Judged(&[(0, 1_000, 1)]);
    draw(&mut canvas, &object, &LaneNotes { late_window_us, states: &hit, ..playing(&timelines, 1_350) });
    assert_eq!(span(&canvas, middle(0), shade(NOTE_RED)), None, "a note that was hit does not fall");
}

/// The judgement windows are drawn above the line as tall as each is long at the speed the next
/// timeline is coming in at, here 400 pixels a second.
#[test]
fn the_judgement_area_is_as_tall_as_its_windows_are_long() {
    let timelines = chart(vec![at(1_000, 0.5)]);
    let scratch_lanes = [false, true];
    let ms = MICROS_PER_MILLI;
    let area = JudgeArea {
        key_us: [20 * ms, 60 * ms, 150 * ms, 250 * ms, 500 * ms],
        scratch_us: [50 * ms, 100 * ms, 150 * ms, 250 * ms, 500 * ms],
        scratch_lanes: &scratch_lanes,
    };
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let object = note_object(field(&mut canvas));
    let shown = NoteDisplay { judge_area: Some(area), ..NoteDisplay::default() };
    assert!(draw(&mut canvas, &object, &LaneNotes { show: shown, ..playing(&timelines, 500) }));

    let (best, second, third) = (Color { r: 0, g: 0, b: 32, a: 255 }, Color { r: 0, g: 32, b: 0, a: 255 }, Color { r: 32, g: 32, b: 0, a: 255 });
    assert_eq!(span(&canvas, middle(0), best), Some((100, 104)), "twenty milliseconds at a fifth of a pixel each");
    assert_eq!(span(&canvas, middle(0), second), Some((104, 112)));
    assert_eq!(span(&canvas, middle(0), third), Some((112, 130)));
    assert_eq!(span(&canvas, middle(1), best), Some((100, 110)), "a turntable has windows of its own");
}

/// The three offsets the note field publishes, from lane zero's rectangle and the player's settings.
#[test]
fn the_lift_and_cover_offsets_follow_the_reference_expressions() {
    let lane = LANES[0];
    let close = |value: f32, expected: f32| (value - expected).abs() < 1e-3;

    let off = lane_offsets(lane, None, None, None);
    assert_eq!((off.lift.y, off.lanecover.y, off.hidden.a), (0.0, 0.0, -255.0), "nothing on: no travel, and what follows the hidden cover is hidden");

    let plain = lane_offsets(lane, None, Some(0.5), Some(0.2));
    assert!(close(plain.lanecover.y, -200.0), "half of the 400 from the top to the line, downwards: {plain:?}");
    assert!(close(plain.hidden.y, 80.0) && plain.hidden.a == 0.0, "a fifth of the lane, upwards: {plain:?}");

    let lifted = lane_offsets(lane, Some(0.25), Some(0.5), Some(0.2));
    assert!(close(lifted.lift.y, 100.0), "a quarter of the lane: {lifted:?}");
    assert!(close(lifted.lanecover.y, -150.0), "half of the 300 the lift leaves: {lifted:?}");
    assert!(close(lifted.hidden.y, 60.0), "a fifth of what the lift leaves: {lifted:?}");
    assert_eq!((lifted.lift.x, lifted.lift.a, lifted.lanecover.a), (0.0, 0.0, 0.0));
}

#[test]
fn the_duration_and_the_fixed_speed_follow_the_reference_expressions() {
    assert_eq!(current_duration_ms(120.0, 1.0, 1.0, 1.0, None), 2_000, "a measure at 120 lasts two seconds");
    assert_eq!(current_duration_ms(120.0, 1.0, 2.0, 1.0, Some(0.25)), 750, "half of it at twice the speed, and three quarters of that under the cover");
    assert_eq!(current_duration_ms(150.0, 2.0, 1.0, 0.5, None), 1_600);
    assert_eq!(current_duration_ms(120.0, 0.0, 1.0, 1.0, None), 0, "a chart that is not scrolling has none");
    assert_eq!(current_duration_ms(120.0, -1.0, 1.0, 1.0, None), 0);
    assert!((fixed_hispeed(150.0, 300, None) - 5.333_333).abs() < 1e-4);
    assert!((fixed_hispeed(150.0, 300, Some(0.5)) - 2.666_666).abs() < 1e-4);

    let mut faster = at(1_000, 0.5);
    (faster.bpm, faster.scroll) = (240.0, 2.0);
    let timelines = chart(vec![faster]);
    assert_eq!(playing(&timelines, 999).tempo(), (BPM, 1.0));
    assert_eq!(playing(&timelines, 1_000).tempo(), (240.0, 2.0));
    assert_eq!(playing(&timelines, 1_000).current_duration_ms(), 500);
    assert_eq!(LaneNotes::new(&timelines[1..], 0, 90.0).tempo(), (90.0, 1.0), "before the first timeline the chart's opening tempo is in force");
}
