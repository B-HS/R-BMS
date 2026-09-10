use std::cell::Cell;
use std::collections::HashMap;

use super::{
    Acc, DestinationTrack, DrawCondition, DrawStateSource, Keyframe, LOOP_ONCE, LuaDrawEval, LuaExprId, MouseRect, OffsetSource, Resolved, STRETCH_UNSPECIFIED,
    SkinColor, SkinOffset, SkinRect, WarnOnce, draw_conditions_from_ops, prepare, resolve,
};
use crate::timer::{TimerState, timer_id};

/// Float slack for a rate that is not exactly representable, such as 650/1000.
const TOLERANCE: f32 = 1e-3;

/// Milliseconds of the last keyframe in the two-keyframe fixture.
const SPAN_MS: i64 = 1_000;

/// The x the fixture's second keyframe sits at, so a resolved x reads as the shaped rate per mille.
const TRAVEL: f32 = 1_000.0;

#[derive(Debug, Default)]
struct FakeState {
    options: HashMap<i32, bool>,
    offsets: HashMap<i32, SkinOffset>,
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
}

impl OffsetSource for FakeState {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offset_reads.set(self.offset_reads.get() + 1);
        self.offsets.get(&id).copied()
    }
}

impl DrawStateSource for FakeState {
    fn boolean(&self, id: i32) -> bool {
        let value = self.options.get(&id.abs()).copied().unwrap_or(false);
        if id < 0 { !value } else { value }
    }
}

#[derive(Debug, Default)]
struct FakeLua {
    answers: HashMap<u32, Option<bool>>,
}

impl FakeLua {
    fn with(mut self, expr: LuaExprId, answer: Option<bool>) -> Self {
        self.answers.insert(expr.0, answer);
        self
    }
}

impl LuaDrawEval for FakeLua {
    fn eval_draw(&self, expr: LuaExprId) -> Option<bool> {
        self.answers.get(&expr.0).copied().flatten()
    }
}

fn frame(time_ms: i64, x: f32, acc: Acc) -> Keyframe {
    Keyframe { time_ms, rect: SkinRect::new(x, 0.0, 100.0, 100.0), clip: None, acc, color: SkinColor::rgba(0, 0, 0, u8::MAX), angle_deg: 0.0 }
}

/// Two keyframes a second apart, travelling [`TRAVEL`] along x.
fn travelling_track(acc: Acc, loop_ms: i64) -> DestinationTrack {
    DestinationTrack { loop_ms, frames: vec![frame(0, 0.0, acc), frame(SPAN_MS, TRAVEL, acc)], ..DestinationTrack::default() }
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!((actual - expected).abs() < TOLERANCE, "{what}: expected {expected}, got {actual}");
}

fn resolved(track: &DestinationTrack, now_ms: i64) -> Option<Resolved> {
    resolve(track, now_ms, &TimerState::new(), &FakeState::default())
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
    let track =
        DestinationTrack { loop_ms: LOOP_ONCE, frames: vec![frame(500, 0.0, Acc::Linear), frame(1_500, TRAVEL, Acc::Linear)], ..DestinationTrack::default() };
    assert_eq!(resolved(&track, 499), None);
    assert_close(resolved(&track, 500).expect("the first keyframe draws").rect.x, 0.0, "at the first keyframe");
}

#[test]
fn a_single_keyframe_holds_its_rectangle() {
    let mut only = frame(0, 10.0, Acc::Linear);
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
    let track = DestinationTrack { timer: Some(timer_id::PLAY), ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let mut timers = TimerState::new();
    let state = FakeState::default();
    assert_eq!(resolve(&track, 5_250, &timers, &state), None, "an off timer draws nothing");

    timers.set_on(timer_id::PLAY, 5_000);
    let resolved = resolve(&track, 5_250, &timers, &state).expect("an on timer draws");
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
    let mut only = frame(0, 0.0, Acc::Linear);
    only.rect = SkinRect::new(100.0, 200.0, 50.0, 60.0);

    let moved = DestinationTrack { offsets: vec![OFFSET_ID], frames: vec![only], ..DestinationTrack::default() };
    let resolved = resolve(&moved, 0, &TimerState::new(), &state).expect("it draws");
    assert_eq!(resolved.rect, SkinRect::new(108.0, 216.0, 54.0, 68.0), "position takes the offset minus half its growth");

    let relative = DestinationTrack { relative: true, ..moved };
    let resolved = resolve(&relative, 0, &TimerState::new(), &state).expect("it draws");
    assert_eq!(resolved.rect, SkinRect::new(100.0, 200.0, 54.0, 68.0), "a relative track only grows");
}

#[test]
fn a_missing_offset_leaves_the_region_alone() {
    let mut only = frame(0, 0.0, Acc::Linear);
    only.rect = SkinRect::new(100.0, 200.0, 50.0, 60.0);
    let track = DestinationTrack { offsets: vec![3], frames: vec![only], ..DestinationTrack::default() };
    assert_eq!(resolved(&track, 0).expect("it draws").rect, only.rect);
}

fn clipped_track(first: Option<SkinRect>, second: Option<SkinRect>) -> DestinationTrack {
    let mut a = frame(0, 0.0, Acc::Linear);
    let mut b = frame(SPAN_MS, TRAVEL, Acc::Linear);
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
    let track = DestinationTrack {
        frames: clipped_track(Some(SkinRect::new(0.0, 0.0, 100.0, 100.0)), Some(SkinRect::new(100.0, 0.0, 200.0, 100.0)))
            .frames
            .into_iter()
            .map(|mut frame| {
                frame.acc = Acc::Step;
                frame
            })
            .collect(),
        loop_ms: LOOP_ONCE,
        ..DestinationTrack::default()
    };
    assert_eq!(resolved(&track, 500).expect("it draws").clip, Some(SkinRect::new(0.0, 0.0, 100.0, 100.0)));
}

fn coloured_track(first: SkinColor, second: SkinColor) -> DestinationTrack {
    let mut a = frame(0, 0.0, Acc::Linear);
    let mut b = frame(SPAN_MS, TRAVEL, Acc::Linear);
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
    let resolved = resolve(&track, 500, &TimerState::new(), &state).expect("it draws");
    assert_eq!(resolved.color, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, 100));
}

#[test]
fn an_alpha_offset_is_dropped_while_a_changing_colour_interpolates() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![OFFSET_ID], ..coloured_track(SkinColor::rgba(0, 0, 0, 0), SkinColor::rgba(100, 200, 40, 200)) };

    let midway = resolve(&track, 500, &TimerState::new(), &state).expect("it draws");
    assert_eq!(midway.color.a, 100, "the reference's interpolating path returns before it applies the offset");

    let at_keyframe = resolve(&track, SPAN_MS, &TimerState::new(), &state).expect("it draws");
    assert_eq!(at_keyframe.color.a, 250, "resting on a keyframe applies it");
}

#[test]
fn an_alpha_offset_clamps() {
    const OFFSET_ID: i32 = 2;
    let shade = SkinColor::rgba(0, 0, 0, 200);
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 500.0, ..SkinOffset::default() });
    let track = DestinationTrack { offsets: vec![OFFSET_ID], ..coloured_track(shade, shade) };
    assert_eq!(resolve(&track, 0, &TimerState::new(), &state).expect("it draws").color.a, u8::MAX);

    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: -500.0, ..SkinOffset::default() });
    assert_eq!(resolve(&track, 0, &TimerState::new(), &state).expect("it draws").color.a, 0);
}

fn angled_track(first: f32, second: f32) -> DestinationTrack {
    let mut a = frame(0, 0.0, Acc::Linear);
    let mut b = frame(SPAN_MS, TRAVEL, Acc::Linear);
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
    let track = DestinationTrack { offsets: vec![FIRST, SECOND], ..angled_track(0.0, 90.0) };
    let resolved = resolve(&track, 333, &TimerState::new(), &state).expect("it draws");
    assert_eq!(resolved.angle_deg, 29.0, "two 0.7 degree nudges truncate separately and add nothing");
}

#[test]
fn the_first_shaped_keyframe_sets_the_whole_track() {
    let track = DestinationTrack {
        frames: vec![frame(0, 0.0, Acc::Linear), frame(500, 500.0, Acc::Decelerate), frame(SPAN_MS, TRAVEL, Acc::Accelerate)],
        ..DestinationTrack::default()
    };
    assert_eq!(track.effective_acc(), Acc::Decelerate);
    assert_eq!(DestinationTrack::default().effective_acc(), Acc::Linear);
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

    let resolved = prepare(&track, 250, &TimerState::new(), &state, None, (0.0, 0.0), None).expect("both conditions hold");
    assert_close(resolved.rect.x, 255.0, "the offset moved it");
    assert_eq!(state.offset_reads.get(), 1, "the region resolved once");

    track.draw_conditions = vec![DrawCondition::Option(SHOWN), DrawCondition::Option(HIDDEN)];
    assert_eq!(prepare(&track, 250, &TimerState::new(), &state, None, (0.0, 0.0), None), None, "one false condition hides it");
    assert_eq!(state.offset_reads.get(), 1, "a hidden object never resolves its region");
}

#[test]
fn a_negative_option_id_reads_as_its_negation() {
    const OPTION: i32 = 21;
    let state = FakeState::default().with_option(OPTION, false);
    let track = DestinationTrack { draw_conditions: vec![DrawCondition::Option(-OPTION)], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    assert!(prepare(&track, 250, &TimerState::new(), &state, None, (0.0, 0.0), None).is_some(), "a false option draws under a negative id");

    let state = FakeState::default().with_option(OPTION, true);
    assert_eq!(prepare(&track, 250, &TimerState::new(), &state, None, (0.0, 0.0), None), None, "a true option hides it");
}

#[test]
fn a_track_without_conditions_always_draws() {
    let track = travelling_track(Acc::Linear, LOOP_ONCE);
    assert!(prepare(&track, 250, &TimerState::new(), &FakeState::default(), None, (0.0, 0.0), None).is_some());
}

#[test]
fn op_lists_drop_zero_duplicates_and_unknown_ids() {
    const KNOWN: i32 = 30;
    const ALSO_KNOWN: i32 = 31;
    const UNKNOWN: i32 = 32;
    let known = |id: i32| id == KNOWN || id == ALSO_KNOWN;

    assert_eq!(draw_conditions_from_ops(&[0, 0, 0], known), vec![], "zero is no condition");
    assert_eq!(draw_conditions_from_ops(&[KNOWN, KNOWN], known), vec![DrawCondition::Option(KNOWN)], "a repeat is dropped");
    assert_eq!(
        draw_conditions_from_ops(&[KNOWN, -KNOWN], known),
        vec![DrawCondition::Option(KNOWN), DrawCondition::Option(-KNOWN)],
        "an id and its negation are separate conditions"
    );
    assert_eq!(draw_conditions_from_ops(&[UNKNOWN], known), vec![], "an unimplemented option is ignored, not read as false");
    assert_eq!(
        draw_conditions_from_ops(&[0, ALSO_KNOWN, UNKNOWN, KNOWN], known),
        vec![DrawCondition::Option(ALSO_KNOWN), DrawCondition::Option(KNOWN)],
        "order is kept"
    );
    assert_eq!(draw_conditions_from_ops(&[i32::MIN], |_| false), vec![], "an id with no positive counterpart is unknown");
}

#[test]
fn a_lua_condition_follows_its_evaluator() {
    let expr = LuaExprId(4);
    let track = DestinationTrack { draw_conditions: vec![DrawCondition::Lua(expr)], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    let state = FakeState::default();

    let lua = FakeLua::default().with(expr, Some(true));
    assert!(prepare(&track, 250, &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None).is_some());

    let lua = FakeLua::default().with(expr, Some(false));
    assert_eq!(prepare(&track, 250, &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None), None);

    let lua = FakeLua::default().with(expr, None);
    assert_eq!(prepare(&track, 250, &TimerState::new(), &state, Some(&lua), (0.0, 0.0), None), None, "a raising expression hides its object");
}

#[test]
fn a_lua_condition_without_an_evaluator_hides_its_object() {
    let track = DestinationTrack { draw_conditions: vec![DrawCondition::Lua(LuaExprId(1))], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    assert_eq!(prepare(&track, 250, &TimerState::new(), &FakeState::default(), None, (0.0, 0.0), None), None);
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
    let resolved = prepare(&track, 500, &TimerState::new(), &FakeState::default(), None, (7.0, -3.0), None).expect("it draws");
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
    let inside = prepare(&track, 250, &timers, &state, None, (0.0, 0.0), Some((300.0, 50.0)));
    assert!(inside.is_some(), "the pointer sits 50 px into a region that starts at 250");

    assert_eq!(prepare(&track, 250, &timers, &state, None, (0.0, 0.0), Some((240.0, 50.0))), None, "left of the region");
    assert_eq!(prepare(&track, 250, &timers, &state, None, (0.0, 0.0), Some((300.0, 151.0))), None, "below the region");
    assert!(prepare(&track, 250, &timers, &state, None, (0.0, 0.0), Some((250.0, 0.0))).is_some(), "the near edge counts as inside");
    assert!(prepare(&track, 250, &timers, &state, None, (0.0, 0.0), Some((350.0, 100.0))).is_some(), "the far edge counts as inside");
    assert_eq!(prepare(&track, 250, &timers, &state, None, (0.0, 0.0), None), None, "no pointer means no draw");
}

#[test]
fn a_pointer_rectangle_follows_the_screen_offset() {
    let mut track = travelling_track(Acc::Linear, LOOP_ONCE);
    track.mouse_rect = Some(MouseRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 });
    let moved = prepare(&track, 250, &TimerState::new(), &FakeState::default(), None, (400.0, 0.0), Some((700.0, 50.0)));
    assert!(moved.is_some(), "the region moved to 650, so 700 is inside it");
    assert_eq!(
        prepare(&track, 250, &TimerState::new(), &FakeState::default(), None, (400.0, 0.0), Some((300.0, 50.0))),
        None,
        "where it used to be is now outside"
    );
}

#[test]
fn an_off_timer_hides_an_object_whose_conditions_hold() {
    const SHOWN: i32 = 5;
    let state = FakeState::default().with_option(SHOWN, true);
    let track =
        DestinationTrack { timer: Some(timer_id::FADEOUT), draw_conditions: vec![DrawCondition::Option(SHOWN)], ..travelling_track(Acc::Linear, LOOP_ONCE) };
    assert_eq!(prepare(&track, 250, &TimerState::new(), &state, None, (0.0, 0.0), None), None);
}

#[test]
fn a_step_track_holds_its_colour_and_drops_the_alpha_offset() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let mut track = coloured_track(SkinColor::rgba(0, 0, 0, 10), SkinColor::rgba(100, 200, 40, 200));
    for frame in &mut track.frames {
        frame.acc = Acc::Step;
    }
    track.offsets = vec![OFFSET_ID];

    let midway = resolve(&track, 500, &TimerState::new(), &state).expect("it draws");
    assert_eq!(midway.color, SkinColor::rgba(0, 0, 0, 10), "a step track holds the keyframe it is on");
    assert_eq!(midway.rect.x, 0.0, "and holds its region with it");
}

#[test]
fn a_step_track_applies_the_alpha_offset_while_it_rests_on_a_keyframe() {
    const OFFSET_ID: i32 = 2;
    let state = FakeState::default().with_offset(OFFSET_ID, SkinOffset { a: 50.0, ..SkinOffset::default() });
    let mut track = coloured_track(SkinColor::rgba(0, 0, 0, 10), SkinColor::rgba(100, 200, 40, 200));
    for frame in &mut track.frames {
        frame.acc = Acc::Step;
    }
    track.offsets = vec![OFFSET_ID];

    let at_end = resolve(&track, SPAN_MS, &TimerState::new(), &state).expect("it draws");
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
