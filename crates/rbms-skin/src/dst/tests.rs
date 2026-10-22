use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use super::{
    Acc, DestinationTrack, DrawCondition, DrawStateSource, Keyframe, LOOP_ONCE, LuaDrawEval, LuaFnId, MouseRect, NullLuaEval, OffsetSource, OpLists, Resolved,
    STRETCH_UNSPECIFIED, SkinColor, SkinOffset, SkinRect, TimerRef, WarnOnce, draw_conditions_from_ops, prepare, resolve,
};
use crate::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState, timer_id};

/// Float slack for a rate that is not exactly representable, such as 650/1000.
const TOLERANCE: f32 = 1e-3;

/// Milliseconds of the last keyframe in the two-keyframe fixture.
const SPAN_MS: i64 = 1_000;

/// The x the fixture's second keyframe sits at, so a resolved x reads as the shaped rate per mille.
const TRAVEL: f32 = 1_000.0;

/// A quarter turn in the document's own counter-clockwise degrees.
const DOCUMENT_QUARTER_TURN: f32 = 90.0;

#[derive(Debug, Default)]
struct FakeState {
    options: HashMap<i32, bool>,
    offsets: HashMap<i32, SkinOffset>,
    colors: HashMap<i32, SkinColor>,
    offset_reads: Cell<u32>,
}

impl FakeState {
    fn with_option(mut self, id: i32, value: bool) -> Self {
        self.options.insert(id, value);
        self
    }

    fn with_offset(mut self, id: i32, offset: SkinOffset) -> Self {
        self.offsets.insert(id, offset);
        self
    }

    fn with_color(mut self, id: i32, color: SkinColor) -> Self {
        self.colors.insert(id, color);
        self
    }
}

impl OffsetSource for FakeState {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offset_reads.set(self.offset_reads.get() + 1);
        self.offsets.get(&id).copied()
    }

    fn color(&self, id: i32) -> Option<SkinColor> {
        self.colors.get(&id).copied()
    }
}

impl DrawStateSource for FakeState {
    fn boolean(&self, id: i32) -> Option<bool> {
        let value = self.options.get(&id.abs()).copied().unwrap_or(false);
        Some(if id < 0 { !value } else { value })
    }
}

/// An evaluator that answers function values and names from tables, and remembers which functions
/// it was asked to call and in what order.
#[derive(Debug, Default)]
struct FakeScript {
    booleans: HashMap<u32, bool>,
    timers: HashMap<u32, i64>,
    names: HashMap<String, bool>,
    called: RefCell<Vec<u32>>,
}

impl FakeScript {
    fn with_boolean(mut self, function: LuaFnId, answer: bool) -> Self {
        self.booleans.insert(function.0, answer);
        self
    }

    fn with_timer(mut self, function: LuaFnId, started_us: i64) -> Self {
        self.timers.insert(function.0, started_us);
        self
    }

    fn with_name(mut self, name: &str, answer: bool) -> Self {
        self.names.insert(name.to_owned(), answer);
        self
    }

    fn called(&self) -> Vec<u32> {
        self.called.borrow().clone()
    }
}

impl LuaDrawEval for FakeScript {
    fn call_boolean(&self, function: LuaFnId) -> bool {
        self.called.borrow_mut().push(function.0);
        self.booleans.get(&function.0).copied().unwrap_or(false)
    }

    fn call_timer(&self, function: LuaFnId) -> i64 {
        self.called.borrow_mut().push(function.0);
        self.timers.get(&function.0).copied().unwrap_or(TIMER_OFF)
    }

    fn named_boolean(&self, name: &str) -> bool {
        self.names.get(name).copied().unwrap_or(false)
    }
}

/// The frame clock at a whole millisecond, in the microseconds the interpolator is handed.
const fn us(ms: i64) -> i64 {
    ms * MICROS_PER_MILLI
}

fn frame(time_ms: i64, x: f32) -> Keyframe {
    Keyframe { time_ms, rect: SkinRect::new(x, 0.0, 100.0, 100.0), clip: None, color: SkinColor::rgba(0, 0, 0, u8::MAX), angle_deg: 0.0 }
}

/// Two keyframes a second apart, travelling [`TRAVEL`] along x.
fn travelling_track(acc: Acc, loop_ms: i64) -> DestinationTrack {
    DestinationTrack { acc, loop_ms, frames: vec![frame(0, 0.0), frame(SPAN_MS, TRAVEL)], ..DestinationTrack::default() }
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!((actual - expected).abs() < TOLERANCE, "{what}: expected {expected}, got {actual}");
}

fn resolved(track: &DestinationTrack, now_ms: i64) -> Option<Resolved> {
    resolve(track, us(now_ms), &TimerState::new(), &FakeState::default(), None)
}

#[test]
fn default_track_leaves_stretch_unspecified() {
    let track = DestinationTrack::default();
    assert_eq!(track.stretch, STRETCH_UNSPECIFIED);
    assert_eq!(track.loop_ms, 0, "a document that names no loop point repeats the whole animation");
    assert!(!track.relative);
}

#[test]
fn an_empty_track_draws_nothing() {
    assert_eq!(resolved(&DestinationTrack::default(), 0), None);
}

#[test]
fn acceleration_and_loop_matrix() {
    let cases: [(&str, i64, i64, [f32; 4]); 4] = [
        ("plays once, inside its span", LOOP_ONCE, 250, [250.0, 62.5, 437.5, 0.0]),
        ("wraps the whole animation", 0, 1_250, [250.0, 62.5, 437.5, 0.0]),
        ("wraps from a mid loop point", 400, 1_250, [650.0, 422.5, 877.5, 0.0]),
        ("holds when the loop point is the end", SPAN_MS, 1_250, [TRAVEL, TRAVEL, TRAVEL, TRAVEL]),
    ];
    let accelerations = [Acc::Linear, Acc::Accelerate, Acc::Decelerate, Acc::Step];
    for (what, loop_ms, now_ms, expected) in cases {
        for (acc, expected) in accelerations.into_iter().zip(expected) {
            let track = travelling_track(acc, loop_ms);
            let resolved = resolved(&track, now_ms).unwrap_or_else(|| panic!("{what} with {acc:?} drew nothing"));
            assert_close(resolved.rect.x, expected, &format!("{what} with {acc:?}"));
        }
    }
}

#[test]
fn a_one_shot_animation_stops_after_its_last_keyframe() {
    let track = travelling_track(Acc::Linear, LOOP_ONCE);
    assert_close(resolved(&track, SPAN_MS).expect("the last keyframe still draws").rect.x, TRAVEL, "at the end");
    assert_eq!(resolved(&track, SPAN_MS + 1), None, "a millisecond past the end it is gone");
}

#[test]
fn nothing_draws_before_the_first_keyframe() {
    let track = DestinationTrack { loop_ms: LOOP_ONCE, frames: vec![frame(500, 0.0), frame(1_500, TRAVEL)], ..DestinationTrack::default() };
    assert_eq!(resolved(&track, 499), None);
    assert_close(resolved(&track, 500).expect("the first keyframe draws").rect.x, 0.0, "at the first keyframe");
}

#[test]
fn a_single_keyframe_holds_its_rectangle() {
    let mut only = frame(0, 10.0);
    only.rect = SkinRect::new(10.0, 20.0, 30.0, 40.0);
    let held = DestinationTrack { frames: vec![only], ..DestinationTrack::default() };
    assert_eq!(resolved(&held, 0).expect("it draws at zero").rect, only.rect);
    assert_eq!(resolved(&held, 60_000).expect("it still draws a minute later").rect, only.rect);

    let once = DestinationTrack { loop_ms: LOOP_ONCE, ..held };
    assert_eq!(resolved(&once, 0).expect("it draws at zero").rect, only.rect);
    assert_eq!(resolved(&once, 1), None, "a one-shot single keyframe is gone the next millisecond");
}

#[test]
fn a_loop_point_past_the_end_falls_back_to_the_first_keyframe() {
    let track = travelling_track(Acc::Linear, 5_000);
    assert_close(resolved(&track, 2_000).expect("it draws").rect.x, 0.0, "past the end but before the loop point");
}

#[test]
fn an_off_timer_hides_the_track_and_an_on_one_shifts_its_clock() {
    let track = DestinationTrack { timer: Some(TimerRef::Id(timer_id::PLAY)), ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let mut timers = TimerState::new();
    let state = FakeState::default();
    assert_eq!(resolve(&track, us(5_250), &timers, &state, None), None, "an off timer draws nothing");

    timers.set_on(timer_id::PLAY, us(5_000));
    let resolved = resolve(&track, us(5_250), &timers, &state, None).expect("an on timer draws");
    assert_close(resolved.rect.x, 250.0, "250 ms after the timer started");
}

#[test]
fn a_track_without_a_timer_reads_the_callers_clock() {
    let track = travelling_track(Acc::Linear, LOOP_ONCE);
    assert_close(resolved(&track, 250).expect("it draws").rect.x, 250.0, "no timer means no shift");
}

#[test]
fn offsets_move_and_resize_the_region_unless_it_is_relative() {
    const OFFSET_ID: i32 = 7;
    let offset = SkinOffset { x: 10.0, y: 20.0, w: 4.0, h: 8.0, r: 0.0, a: 0.0 };
    let state = FakeState::default().with_offset(OFFSET_ID, offset);
    let mut only = frame(0, 0.0);
    only.rect = SkinRect::new(100.0, 200.0, 50.0, 60.0);

    let moved = DestinationTrack { offsets: vec![OFFSET_ID], frames: vec![only], ..DestinationTrack::default() };
    let resolved = resolve(&moved, us(0), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(resolved.rect, SkinRect::new(108.0, 216.0, 54.0, 68.0), "position takes the offset minus half its growth");

    let relative = DestinationTrack { relative: true, ..moved };
    let resolved = resolve(&relative, us(0), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(resolved.rect, SkinRect::new(100.0, 200.0, 54.0, 68.0), "a relative track only grows");
}

#[test]
fn a_missing_offset_leaves_the_region_alone() {
    let mut only = frame(0, 0.0);
    only.rect = SkinRect::new(100.0, 200.0, 50.0, 60.0);
    let track = DestinationTrack { offsets: vec![3], frames: vec![only], ..DestinationTrack::default() };
    assert_eq!(resolved(&track, 0).expect("it draws").rect, only.rect);
}

fn clipped_track(first: Option<SkinRect>, second: Option<SkinRect>) -> DestinationTrack {
    let mut a = frame(0, 0.0);
    let mut b = frame(SPAN_MS, TRAVEL);
    a.clip = first;
    b.clip = second;
    DestinationTrack { loop_ms: LOOP_ONCE, frames: vec![a, b], ..DestinationTrack::default() }
}

#[test]
fn a_keyframe_without_a_clip_turns_clipping_off() {
    let track = clipped_track(None, Some(SkinRect::new(0.0, 0.0, 10.0, 10.0)));
    assert_eq!(resolved(&track, 500).expect("it draws").clip, None);
}

#[test]
fn a_next_keyframe_without_a_clip_holds_the_current_one() {
    let clip = SkinRect::new(4.0, 5.0, 60.0, 70.0);
    let track = clipped_track(Some(clip), None);
    assert_eq!(resolved(&track, 500).expect("it draws").clip, Some(clip));
}

#[test]
fn two_clips_interpolate_like_the_region() {
    let track = clipped_track(Some(SkinRect::new(0.0, 0.0, 100.0, 100.0)), Some(SkinRect::new(100.0, 0.0, 200.0, 100.0)));
    assert_eq!(resolved(&track, 500).expect("it draws").clip, Some(SkinRect::new(50.0, 0.0, 150.0, 100.0)));
}

#[test]
fn an_empty_clip_turns_clipping_off() {
    let track = clipped_track(Some(SkinRect::new(10.0, 10.0, 0.0, 40.0)), Some(SkinRect::new(10.0, 10.0, 0.0, 40.0)));
    assert_eq!(resolved(&track, 500).expect("it draws").clip, None, "a zero-width clip is no clip");

    let track = clipped_track(Some(SkinRect::new(10.0, 10.0, 40.0, 0.0)), Some(SkinRect::new(10.0, 10.0, 40.0, 0.0)));
    assert_eq!(resolved(&track, 500).expect("it draws").clip, None, "a zero-height clip is no clip");
}

#[test]
fn a_step_track_holds_its_clip_too() {
    let track =
        DestinationTrack { acc: Acc::Step, ..clipped_track(Some(SkinRect::new(0.0, 0.0, 100.0, 100.0)), Some(SkinRect::new(100.0, 0.0, 200.0, 100.0))) };
    assert_eq!(resolved(&track, 500).expect("it draws").clip, Some(SkinRect::new(0.0, 0.0, 100.0, 100.0)));
}

fn coloured_track(first: SkinColor, second: SkinColor) -> DestinationTrack {
    let mut a = frame(0, 0.0);
    let mut b = frame(SPAN_MS, TRAVEL);
    a.color = first;
    b.color = second;
    DestinationTrack { loop_ms: LOOP_ONCE, frames: vec![a, b], ..DestinationTrack::default() }
}

#[test]
fn colours_interpolate_channel_by_channel() {
    let track = coloured_track(SkinColor::rgba(0, 0, 0, 0), SkinColor::rgba(100, 200, 40, 200));
    assert_eq!(resolved(&track, 500).expect("it draws").color, SkinColor::rgba(50, 100, 20, 100));
}

#[test]
fn an_alpha_offset_applies_to_a_track_whose_colour_never_changes() {
    const OFFSET_ID: i32 = 2;
    let shade = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 200);
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: -100.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![OFFSET_ID], ..coloured_track(shade, shade) };
    let resolved = resolve(&track, us(500), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(resolved.color, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 100));
}

/// An alpha offset shifts every frame of a track whose colour changes, between its keyframes as on
/// them. The reference leaves it out between keyframes; an object that fades would flash there.
#[test]
fn an_alpha_offset_applies_while_a_changing_colour_interpolates() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![OFFSET_ID], ..coloured_track(SkinColor::rgba(0, 0, 0, 0), SkinColor::rgba(100, 200, 40, 200)) };

    let midway = resolve(&track, us(500), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(midway.color, SkinColor::rgba(50, 100, 20, 150), "the colour is interpolated and its alpha shifted");

    let at_keyframe = resolve(&track, us(SPAN_MS), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(at_keyframe.color.a, 250, "resting on a keyframe applies it");

    let stepped = DestinationTrack { acc: Acc::Step, ..track };
    let held = resolve(&stepped, us(500), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(held.color.a, 50, "a track that holds its keyframes is shifted while it holds one");
}

/// An offset that takes everything away leaves nothing of a fading object on any frame of the fade.
#[test]
fn an_alpha_offset_of_the_whole_range_hides_every_frame_of_a_fade() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: -f32::from(u8::MAX), ..SkinOffset::default() });
    let fade = coloured_track(SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX), SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 0));
    let track = DestinationTrack { offsets: vec![OFFSET_ID], ..fade };
    for at in [0, 1, 250, 500, 999, SPAN_MS] {
        let seen = resolve(&track, us(at), &TimerState::new(), &state, None).expect("it draws");
        assert_eq!(seen.color.a, 0, "{at} ms into the fade");
    }
}

/// Each alpha offset is clamped as it is applied, in the order the track names them, so what one
/// took below nothing is not owed by the next (`SkinObject.prepareColor`, which clamps inside its
/// loop).
#[test]
fn alpha_offsets_are_clamped_one_by_one_in_the_order_they_are_named() {
    const FIRST: i32 = 2;
    const SECOND: i32 = 3;
    let shade = SkinColor::rgba(0, 0, 0, 100);
    let state = FakeState::default()
        .with_offset(FIRST, SkinOffset { a: -200.0, ..SkinOffset::default() })
        .with_offset(SECOND, SkinOffset { a: 60.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![FIRST, SECOND], ..coloured_track(shade, shade) };
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &state, None).expect("it draws").color.a, 60);
}

#[test]
fn an_alpha_offset_clamps() {
    const OFFSET_ID: i32 = 2;
    let shade = SkinColor::rgba(0, 0, 0, 200);
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 500.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![OFFSET_ID], ..coloured_track(shade, shade) };
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &state, None).expect("it draws").color.a, u8::MAX);

    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: -500.0, ..SkinOffset::default() });
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &state, None).expect("it draws").color.a, 0);
}

/// The colour slot the tint tests name.
const TINT_ID: i32 = 7;

#[test]
fn a_tinted_white_track_comes_out_as_exactly_the_colour_the_player_set() {
    let track = DestinationTrack { tint: Some(TINT_ID), ..coloured_track(SkinColor::WHITE, SkinColor::WHITE) };
    let chosen = SkinColor::rgba(30, 144, 255, 200);
    let state = FakeState::default().with_color(TINT_ID, chosen);
    assert_eq!(resolve(&track, us(500), &TimerState::new(), &state, None).expect("it draws").color, chosen);
}

#[test]
fn a_tint_multiplies_every_channel_of_the_colour_the_track_is_drawn_in() {
    let drawn_in = SkinColor::rgba(102, 102, 102, 128);
    let track = DestinationTrack { tint: Some(TINT_ID), ..coloured_track(drawn_in, drawn_in) };
    let state = FakeState::default().with_color(TINT_ID, SkinColor::rgba(255, 128, 0, 255));
    assert_eq!(
        resolve(&track, us(0), &TimerState::new(), &state, None).expect("it draws").color,
        SkinColor::rgba(102, 51, 0, 128),
        "white leaves a channel, black empties it, and a half takes half of it"
    );
}

#[test]
fn a_tint_with_no_colour_behind_it_changes_nothing() {
    let drawn_in = SkinColor::rgba(10, 20, 30, 40);
    let track = DestinationTrack { tint: Some(TINT_ID), ..coloured_track(drawn_in, drawn_in) };
    let untinted = DestinationTrack { tint: None, ..track.clone() };
    let set_elsewhere = FakeState::default().with_color(TINT_ID + 1, SkinColor::rgba(0, 0, 0, 0));
    for state in [FakeState::default(), set_elsewhere] {
        let expected = resolve(&untinted, us(0), &TimerState::new(), &state, None).expect("it draws");
        assert_eq!(resolve(&track, us(0), &TimerState::new(), &state, None).expect("it draws"), expected, "a colour nobody set tinted the track");
    }
}

#[test]
fn a_track_that_names_no_tint_ignores_every_colour_the_player_set() {
    let drawn_in = SkinColor::rgba(10, 20, 30, 40);
    let track = coloured_track(drawn_in, drawn_in);
    let state = FakeState::default().with_color(TINT_ID, SkinColor::rgba(0, 0, 0, 0));
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &state, None).expect("it draws").color, drawn_in);
}

/// The offsets' alpha shifts are sums and the tint is a product, and the product comes last, which
/// is the order a lane's colour reaches its notes in: a half-clear tint is half of whatever alpha
/// the offsets leave. Over an opaque track shifted further up that is 128; at half opacity it is a
/// quarter, where adding the shift last would have left nothing.
#[test]
fn the_tints_alpha_is_a_share_of_the_alpha_the_offsets_leave() {
    const OFFSET_ID: i32 = 2;
    const UP: f32 = 50.0;
    const HALF_DOWN: f32 = -127.5;
    const A_QUARTER: u8 = 64;
    let half_clear = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 128);
    let shifted = |a: f32| FakeState::default().with_color(TINT_ID, half_clear).with_offset(OFFSET_ID, SkinOffset { a, ..SkinOffset::default() });
    let track = DestinationTrack { tint: Some(TINT_ID), offsets: vec![OFFSET_ID], ..coloured_track(SkinColor::WHITE, SkinColor::WHITE) };
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &shifted(UP), None).expect("it draws").color.a, half_clear.a);
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &shifted(HALF_DOWN), None).expect("it draws").color.a, A_QUARTER, "half of half");

    let clear = FakeState::default()
        .with_color(TINT_ID, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 0))
        .with_offset(OFFSET_ID, SkinOffset { a: UP, ..SkinOffset::default() });
    assert_eq!(resolve(&track, us(0), &TimerState::new(), &clear, None).expect("it draws").color.a, 0, "a tint with no alpha hides the track");
}

#[test]
fn a_tint_follows_the_colour_the_track_interpolates_through() {
    let track = DestinationTrack { tint: Some(TINT_ID), ..coloured_track(SkinColor::rgba(0, 0, 0, 255), SkinColor::rgba(200, 200, 200, 255)) };
    let state = FakeState::default().with_color(TINT_ID, SkinColor::rgba(128, 255, 0, 255));
    assert_eq!(resolve(&track, us(500), &TimerState::new(), &state, None).expect("it draws").color, SkinColor::rgba(50, 100, 0, 255));
}

#[test]
fn a_colour_answered_differently_on_the_next_frame_tints_the_next_frame() {
    let track = DestinationTrack { tint: Some(TINT_ID), ..coloured_track(SkinColor::WHITE, SkinColor::WHITE) };
    let first = FakeState::default().with_color(TINT_ID, SkinColor::rgba(255, 0, 0, 255));
    let second = FakeState::default().with_color(TINT_ID, SkinColor::rgba(0, 0, 255, 255));
    let timers = TimerState::new();
    assert_eq!(resolve(&track, us(0), &timers, &first, None).expect("it draws").color, SkinColor::rgba(255, 0, 0, 255));
    assert_eq!(resolve(&track, us(16), &timers, &second, None).expect("it draws").color, SkinColor::rgba(0, 0, 255, 255));
}

#[test]
fn a_colour_multiplies_channel_by_channel_and_rounds_to_nearest() {
    for value in 0..=u8::MAX {
        let shade = SkinColor::rgba(value, value, value, value);
        assert_eq!(shade.tinted(SkinColor::WHITE), shade, "white changed {value}");
        assert_eq!(SkinColor::WHITE.tinted(shade), shade, "{value} was not kept");
        assert_eq!(shade.tinted(SkinColor::rgba(0, 0, 0, 0)), SkinColor::rgba(0, 0, 0, 0), "black kept {value}");
    }
    assert_eq!(
        SkinColor::rgba(1, 2, 3, 4).tinted(SkinColor::rgba(128, 128, 128, 128)),
        SkinColor::rgba(1, 1, 2, 2),
        "0.502, 1.004, 1.506 and 2.008 round to nearest"
    );
}

#[test]
fn a_colour_is_read_from_six_or_eight_hex_digits_and_written_as_eight() {
    assert_eq!(SkinColor::from_hex("30E050"), Some(SkinColor::rgba(0x30, 0xE0, 0x50, u8::MAX)));
    assert_eq!(SkinColor::from_hex("#30e05080"), Some(SkinColor::rgba(0x30, 0xE0, 0x50, 0x80)));
    for malformed in ["", "#", "30E05", "30E0508", "30E050801", "30E05G", "+f0000", "#-10000", " 30E050", "30E050 ", "ÀÀÀ", "#30E0 50"] {
        assert_eq!(SkinColor::from_hex(malformed), None, "{malformed:?} was read as a colour");
    }
    let colour = SkinColor::rgba(1, 171, 255, 16);
    assert_eq!(colour.to_hex(), "01ABFF10");
    assert_eq!(SkinColor::from_hex(&colour.to_hex()), Some(colour));
}

fn angled_track(first: f32, second: f32) -> DestinationTrack {
    let mut a = frame(0, 0.0);
    let mut b = frame(SPAN_MS, TRAVEL);
    a.angle_deg = first;
    b.angle_deg = second;
    DestinationTrack { loop_ms: LOOP_ONCE, frames: vec![a, b], ..DestinationTrack::default() }
}

#[test]
fn an_interpolated_angle_truncates_like_the_reference_int() {
    let track = angled_track(0.0, 90.0);
    assert_eq!(resolved(&track, 333).expect("it draws").angle_deg, 29.0, "29.97 degrees truncates to 29");
}

#[test]
fn each_angle_offset_truncates_on_its_own() {
    const FIRST: i32 = 1;
    const SECOND: i32 = 2;
    let nudge = SkinOffset { r: 0.7, ..SkinOffset::default() };
    let state = FakeState::default().with_offset(FIRST, nudge).with_offset(SECOND, nudge);
    let track = DestinationTrack { offsets: vec![FIRST, SECOND], ..angled_track(0.0, -DOCUMENT_QUARTER_TURN) };
    let resolved = resolve(&track, us(333), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(resolved.angle_deg, -29.0, "the document's 29 takes 0.7 twice as `(int)(29 + 0.7)`, which adds nothing either time");

    let track = DestinationTrack { offsets: vec![FIRST, SECOND], ..angled_track(0.0, DOCUMENT_QUARTER_TURN) };
    let resolved = resolve(&track, us(333), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(resolved.angle_deg, 27.0, "the document's -29 becomes `(int)(-29 + 0.7)` = -28 and then -27");
}

#[test]
fn a_positive_angle_offset_turns_the_way_a_positive_document_angle_does() {
    const OFFSET_ID: i32 = 4;
    const DOCUMENT_ANGLE: f32 = 30.0;
    const NUDGE: f32 = 15.0;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { r: NUDGE, ..SkinOffset::default() });

    let mut written = frame(0, 0.0);
    written.angle_deg = -(DOCUMENT_ANGLE + NUDGE);
    let written = DestinationTrack { frames: vec![written], ..DestinationTrack::default() };
    let expected = resolved(&written, 0).expect("it draws").angle_deg;

    let mut nudged = frame(0, 0.0);
    nudged.angle_deg = -DOCUMENT_ANGLE;
    let nudged = DestinationTrack { offsets: vec![OFFSET_ID], frames: vec![nudged], ..DestinationTrack::default() };
    let resolved = resolve(&nudged, 0, &TimerState::new(), &state, None).expect("it draws");

    assert_eq!(resolved.angle_deg, expected, "`angle: 30` nudged by `r: 15` is `angle: 45`, as `angle += off.r` has it");
    assert_eq!(resolved.angle_deg, -45.0, "which is 45 degrees counter-clockwise, so a negative clockwise angle");
}

#[test]
fn a_negative_angle_offset_turns_clockwise() {
    const OFFSET_ID: i32 = 4;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { r: -10.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![OFFSET_ID], frames: vec![frame(0, 0.0)], ..DestinationTrack::default() };
    assert_eq!(resolve(&track, 0, &TimerState::new(), &state, None).expect("it draws").angle_deg, 10.0);
}

#[test]
fn the_tracks_one_acceleration_shapes_every_keyframe_pair() {
    let track =
        DestinationTrack { acc: Acc::Accelerate, frames: vec![frame(0, 0.0), frame(500, 500.0), frame(SPAN_MS, TRAVEL)], ..DestinationTrack::default() };
    assert_close(resolved(&track, 250).expect("it draws").rect.x, 125.0, "half way through the first pair is a quarter of its travel");
    assert_close(resolved(&track, 750).expect("it draws").rect.x, 625.0, "and the second pair is shaped the same way");
    assert_eq!(DestinationTrack::default().acc, Acc::Linear, "a track nobody shaped is linear");
}

#[test]
fn the_clock_and_the_timer_are_truncated_to_milliseconds_separately() {
    let track = DestinationTrack { timer: Some(TimerRef::Id(timer_id::PLAY)), ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let state = FakeState::default();
    let mut timers = TimerState::new();

    timers.set_on(timer_id::PLAY, 1_999);
    let resolved = resolve(&track, 2_000, &timers, &state, None).expect("an on timer draws");
    assert_close(resolved.rect.x, 1.0, "2000 us is millisecond 2 and 1999 us is millisecond 1, so one whole millisecond has passed");

    timers.set_on(timer_id::PLAY, 1_000);
    let resolved = resolve(&track, 1_999, &timers, &state, None).expect("an on timer draws");
    assert_close(resolved.rect.x, 0.0, "999 us into the same millisecond is no time at all");

    timers.set_on(timer_id::PLAY, us(5_000) + 999);
    let resolved = resolve(&track, us(5_250), &timers, &state, None).expect("an on timer draws");
    assert_close(resolved.rect.x, 250.0, "the timer's own microseconds are dropped before the subtraction");
}

#[test]
fn unknown_acceleration_ids_interpolate_linearly() {
    assert_eq!(Acc::from_id(0), Acc::Linear);
    assert_eq!(Acc::from_id(1), Acc::Accelerate);
    assert_eq!(Acc::from_id(2), Acc::Decelerate);
    assert_eq!(Acc::from_id(3), Acc::Step);
    assert_eq!(Acc::from_id(4), Acc::Linear);
    assert_eq!(Acc::from_id(-1), Acc::Linear);
}

#[test]
fn every_condition_must_hold_before_the_region_is_resolved() {
    const OFFSET_ID: i32 = 9;
    const SHOWN: i32 = 11;
    const HIDDEN: i32 = 12;
    let state = FakeState::default().with_option(SHOWN, true).with_option(HIDDEN, false).with_offset(OFFSET_ID, SkinOffset { x: 5.0, ..SkinOffset::default() });
    let mut track = travelling_track(Acc::Linear, LOOP_ONCE);
    track.offsets = vec![OFFSET_ID];
    track.draw_conditions = vec![DrawCondition::Option(SHOWN), DrawCondition::Option(SHOWN)];

    let resolved = prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None).expect("both conditions hold");
    assert_close(resolved.rect.x, 255.0, "the offset moved it");
    assert_eq!(state.offset_reads.get(), 1, "the region resolved once");

    track.draw_conditions = vec![DrawCondition::Option(SHOWN), DrawCondition::Option(HIDDEN)];
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None), None, "one false condition hides it");
    assert_eq!(state.offset_reads.get(), 1, "a hidden object never resolves its region");
}

#[test]
fn a_negative_option_id_reads_as_its_negation() {
    const OPTION: i32 = 21;
    let state = FakeState::default().with_option(OPTION, false);
    let track = DestinationTrack { draw_conditions: vec![DrawCondition::Option(-OPTION)], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    assert!(prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None).is_some(), "a false option draws under a negative id");

    let state = FakeState::default().with_option(OPTION, true);
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None), None, "a true option hides it");
}

#[test]
fn a_track_without_conditions_always_draws() {
    let track = travelling_track(Acc::Linear, LOOP_ONCE);
    assert!(prepare(&track, us(250), &TimerState::new(), &FakeState::default(), None, (0.0, 0.0), None).is_some());
}

#[test]
fn op_lists_drop_zeros_and_repeats_and_set_the_ids_no_property_answers_apart() {
    const KNOWN: i32 = 30;
    const ALSO_KNOWN: i32 = 31;
    const UNKNOWN: i32 = 32;
    let known = |id: i32| id == KNOWN || id == ALSO_KNOWN;

    assert_eq!(draw_conditions_from_ops(&[0, 0, 0], known), OpLists::default(), "zero is no condition");
    assert_eq!(draw_conditions_from_ops(&[KNOWN, KNOWN], known).conditions, vec![DrawCondition::Option(KNOWN)], "a repeat is dropped");
    assert_eq!(
        draw_conditions_from_ops(&[KNOWN, -KNOWN], known).conditions,
        vec![DrawCondition::Option(KNOWN), DrawCondition::Option(-KNOWN)],
        "an id and its negation are separate conditions"
    );
    assert_eq!(
        draw_conditions_from_ops(&[UNKNOWN, -UNKNOWN, UNKNOWN], known),
        OpLists { conditions: vec![], options: vec![UNKNOWN, -UNKNOWN] },
        "an id no property answers is one of the skin's own options under either sign, and never a condition"
    );
    assert_eq!(
        draw_conditions_from_ops(&[0, ALSO_KNOWN, UNKNOWN, KNOWN], known),
        OpLists { conditions: vec![DrawCondition::Option(ALSO_KNOWN), DrawCondition::Option(KNOWN)], options: vec![UNKNOWN] },
        "order is kept in both lists"
    );
    assert_eq!(draw_conditions_from_ops(&[i32::MIN], |_| false).options, vec![i32::MIN], "an id with no positive counterpart is nobody's property");
}

#[test]
fn a_warning_latch_fires_once() {
    let latch = WarnOnce::new();
    assert!(latch.should_warn());
    assert!(!latch.should_warn());
    assert!(!latch.should_warn());
    assert!(WarnOnce::default().should_warn(), "a fresh latch is unfired");
}

#[test]
fn the_screen_offset_moves_the_region_and_its_clip() {
    let clip = SkinRect::new(10.0, 20.0, 30.0, 40.0);
    let track = clipped_track(Some(clip), Some(clip));
    let resolved = prepare(&track, us(500), &TimerState::new(), &FakeState::default(), None, (7.0, -3.0), None).expect("it draws");
    assert_close(resolved.rect.x, 507.0, "region x");
    assert_close(resolved.rect.y, -3.0, "region y");
    assert_eq!(resolved.clip, Some(SkinRect::new(17.0, 17.0, 30.0, 40.0)));
}

#[test]
fn a_pointer_rectangle_gates_on_the_moved_region() {
    let mut track = travelling_track(Acc::Linear, LOOP_ONCE);
    track.mouse_rect = Some(MouseRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 });
    let state = FakeState::default();
    let timers = TimerState::new();
    let inside = prepare(&track, us(250), &timers, &state, None, (0.0, 0.0), Some((300.0, 50.0)));
    assert!(inside.is_some(), "the pointer sits 50 px into a region that starts at 250");

    assert_eq!(prepare(&track, us(250), &timers, &state, None, (0.0, 0.0), Some((240.0, 50.0))), None, "left of the region");
    assert_eq!(prepare(&track, us(250), &timers, &state, None, (0.0, 0.0), Some((300.0, 151.0))), None, "below the region");
    assert!(prepare(&track, us(250), &timers, &state, None, (0.0, 0.0), Some((250.0, 0.0))).is_some(), "the near edge counts as inside");
    assert!(prepare(&track, us(250), &timers, &state, None, (0.0, 0.0), Some((350.0, 100.0))).is_some(), "the far edge counts as inside");
    assert_eq!(prepare(&track, us(250), &timers, &state, None, (0.0, 0.0), None), None, "no pointer means no draw");
}

#[test]
fn a_pointer_rectangle_follows_the_screen_offset() {
    let mut track = travelling_track(Acc::Linear, LOOP_ONCE);
    track.mouse_rect = Some(MouseRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 });
    let moved = prepare(&track, us(250), &TimerState::new(), &FakeState::default(), None, (400.0, 0.0), Some((700.0, 50.0)));
    assert!(moved.is_some(), "the region moved to 650, so 700 is inside it");
    assert_eq!(
        prepare(&track, us(250), &TimerState::new(), &FakeState::default(), None, (400.0, 0.0), Some((300.0, 50.0))),
        None,
        "where it used to be is now outside"
    );
}

#[test]
fn an_off_timer_hides_an_object_whose_conditions_hold() {
    const SHOWN: i32 = 5;
    let state = FakeState::default().with_option(SHOWN, true);
    let track = DestinationTrack {
        timer: Some(TimerRef::Id(timer_id::FADEOUT)),
        draw_conditions: vec![DrawCondition::Option(SHOWN)],
        ..travelling_track(Acc::Linear, LOOP_ONCE)
    };
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None), None);
}

#[test]
fn a_step_track_holds_its_colour_and_takes_the_alpha_offset_with_it() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let mut track = coloured_track(SkinColor::rgba(0, 0, 0, 10), SkinColor::rgba(100, 200, 40, 200));
    track.acc = Acc::Step;
    track.offsets = vec![OFFSET_ID];

    let midway = resolve(&track, us(500), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(midway.color, SkinColor::rgba(0, 0, 0, 60), "a step track holds the keyframe it is on, shifted like any other frame");
    assert_eq!(midway.rect.x, 0.0, "and holds its region with it");
}

#[test]
fn a_step_track_applies_the_alpha_offset_while_it_rests_on_a_keyframe() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let mut track = coloured_track(SkinColor::rgba(0, 0, 0, 10), SkinColor::rgba(100, 200, 40, 200));
    track.acc = Acc::Step;
    track.offsets = vec![OFFSET_ID];

    let at_end = resolve(&track, us(SPAN_MS), &TimerState::new(), &state, None).expect("it draws");
    assert_eq!(at_end.color.a, 250);
}

#[test]
fn a_clip_on_the_last_keyframe_needs_no_next_one() {
    let clip = SkinRect::new(1.0, 2.0, 30.0, 40.0);
    let track = clipped_track(Some(clip), Some(clip));
    assert_eq!(resolved(&track, SPAN_MS).expect("it draws").clip, Some(clip), "resting on the last keyframe reads no keyframe past it");

    let track = clipped_track(None, Some(clip));
    assert_eq!(resolved(&track, SPAN_MS).expect("it draws").clip, Some(clip));
}

#[test]
fn the_null_evaluator_answers_the_fallback_of_every_type() {
    let function = LuaFnId(7);
    let lua = NullLuaEval;
    assert!(!lua.call_boolean(function));
    assert_eq!(lua.call_integer(function), 0);
    assert_eq!(lua.call_float(function), 0.0);
    assert_eq!(lua.call_text(function), "");
    assert_eq!(lua.call_timer(function), TIMER_OFF, "a timer nobody can compute is off, not on since the scene began");
    assert!(!lua.named_boolean("anything"));
    assert_eq!(lua.named_integer("anything"), 0);
    assert_eq!(lua.named_float("anything"), 0.0);
    assert_eq!(lua.named_text("anything"), "");
}

#[test]
fn a_function_condition_follows_its_evaluator() {
    let function = LuaFnId(3);
    let track = DestinationTrack { draw_conditions: vec![DrawCondition::Function(function)], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let state = FakeState::default();

    let lua = FakeScript::default().with_boolean(function, true);
    assert!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None).is_some());

    let lua = FakeScript::default().with_boolean(function, false);
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None), None);

    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, Some(&NullLuaEval), (0.0, 0.0), None), None, "the fallback of a condition is false");
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None), None, "and so is having no evaluator at all");
}

#[test]
fn a_named_condition_follows_its_evaluator() {
    const NAME: &str = "!is_autoplay";
    let track = DestinationTrack { draw_conditions: vec![DrawCondition::Name(NAME.to_owned())], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let state = FakeState::default();

    let lua = FakeScript::default().with_name(NAME, true);
    assert!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None).is_some(), "the name is handed over exactly as written");

    let lua = FakeScript::default().with_name("is_autoplay", true);
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None), None, "a name nothing answers to is false");
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, None, (0.0, 0.0), None), None);
}

#[test]
fn conditions_are_asked_in_declared_order_and_stop_at_the_first_false() {
    const SHOWN: i32 = 5;
    let (first, second, third) = (LuaFnId(1), LuaFnId(2), LuaFnId(3));
    let state = FakeState::default().with_option(SHOWN, true);
    let track = DestinationTrack {
        draw_conditions: vec![DrawCondition::Option(SHOWN), DrawCondition::Function(first), DrawCondition::Function(second), DrawCondition::Function(third)],
        ..travelling_track(Acc::Linear, LOOP_ONCE)
    };

    let lua = FakeScript::default().with_boolean(first, true).with_boolean(second, true).with_boolean(third, true);
    assert!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None).is_some());
    assert_eq!(lua.called(), vec![first.0, second.0, third.0], "every function runs, in the order the document declared them");

    let lua = FakeScript::default().with_boolean(first, true).with_boolean(third, true);
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None), None);
    assert_eq!(lua.called(), vec![first.0, second.0], "the function after the first false one is never called");
}

#[test]
fn a_function_timer_times_its_track() {
    let function = LuaFnId(9);
    let track = DestinationTrack { timer: Some(TimerRef::Lua(function)), ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let state = FakeState::default();
    let timers = TimerState::new();

    let lua = FakeScript::default().with_timer(function, us(5_000));
    let resolved = resolve(&track, us(5_250), &timers, &state, Some(&lua)).expect("a timer the function reports as on draws");
    assert_close(resolved.rect.x, 250.0, "elapsed is measured from the moment the function answered");

    let lua = FakeScript::default().with_timer(function, TIMER_OFF);
    assert_eq!(resolve(&track, us(5_250), &timers, &state, Some(&lua)), None, "a function that answers off hides the object");

    let lua = FakeScript::default().with_timer(function, 0);
    let resolved = resolve(&track, us(250), &timers, &state, Some(&lua)).expect("zero is on since the scene began, not off");
    assert_close(resolved.rect.x, 250.0, "a timer on at zero");
}

#[test]
fn a_function_timer_is_off_without_something_to_call_it() {
    let function = LuaFnId(9);
    let track = DestinationTrack { timer: Some(TimerRef::Lua(function)), ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let state = FakeState::default();
    let mut timers = TimerState::new();
    timers.set_on(timer_id::PLAY, 0);

    assert_eq!(resolve(&track, us(250), &timers, &state, None), None, "no evaluator");
    assert_eq!(resolve(&track, us(250), &timers, &state, Some(&NullLuaEval)), None, "the null evaluator");
    assert_eq!(TimerRef::Lua(function).value_us(&timers, None), TIMER_OFF);
    assert_eq!(TimerRef::Lua(function).value_us(&timers, Some(&NullLuaEval)), TIMER_OFF);
    assert_eq!(TimerRef::Id(timer_id::PLAY).value_us(&timers, None), 0, "an id never needed one");
}

#[test]
fn a_function_timer_is_only_called_once_the_conditions_hold() {
    let (gate, timer) = (LuaFnId(1), LuaFnId(2));
    let track = DestinationTrack {
        timer: Some(TimerRef::Lua(timer)),
        draw_conditions: vec![DrawCondition::Function(gate)],
        ..travelling_track(Acc::Linear, LOOP_ONCE)
    };
    let state = FakeState::default();

    let lua = FakeScript::default().with_boolean(gate, false).with_timer(timer, 0);
    assert_eq!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None), None);
    assert_eq!(lua.called(), vec![gate.0], "a hidden object never asks for its timer");

    let lua = FakeScript::default().with_boolean(gate, true).with_timer(timer, 0);
    assert!(prepare(&track, us(250), &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None).is_some());
    assert_eq!(lua.called(), vec![gate.0, timer.0], "conditions first, then the timer");
}
