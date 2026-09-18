//! Unit tests for the character a play document draws from a `.chp` definition.
//!
//! The frames are one-pixel cells of a colour each, so a drawn pixel names the frame that reached
//! the screen, and every geometry assertion is read off the canvas rather than off the maths.

use rbms_skin::chp::CharaRect;
use rbms_skin::dst::{Acc, DestinationTrack, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinOffset, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::SkinLayer;
use rbms_skin::property::generated::{OPTION_1P_100, OPTION_1P_BORDER_OR_MORE};
use rbms_skin::property::{SkinStateSource, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{TimerId, TimerState, timer_id};

use super::draw::draw_object;
use super::object::{Body, SkinObject};
use super::pmchara::{CharaKind, CharaMotion, CharaMotionFrame, PmCharaBody, play_binding};
use super::screen::{PlayLanes, PlayTimers};
use super::{FrameExtra, SkinFrame, SkinViewport};
use crate::ctx::with_render_ctx;
use crate::hud::HudView;
use crate::{Color, CpuCanvas, Renderer, TextureId, UvRect};

/// The canvas every test draws on, which is also the size the documents are authored at so the
/// viewport maps one to one and a document pixel is a screen pixel.
const CANVAS: (u32, u32) = (1280, 720);

/// The box the definition states its destinations in.
const SIZE_BOX: (f32, f32) = (128.0, 128.0);

/// Milliseconds one frame of the test animation lasts.
const FRAME_MS: i64 = 100;

/// The colour of each frame of the two-frame sheet.
const FRAME_COLOURS: [Color; 2] = [Color { r: 200, g: 0, b: 0, a: 255 }, Color { r: 100, g: 0, b: 0, a: 255 }];

/// A two-pixel sheet, one pixel per frame.
fn sheet(canvas: &mut CpuCanvas) -> TextureId {
    let pixels: Vec<u8> = FRAME_COLOURS.iter().flat_map(|colour| [colour.r, colour.g, colour.b, colour.a]).collect();
    canvas.register_texture("chara", &pixels, FRAME_COLOURS.len() as u32, 1)
}

/// One frame over cell `index` of the sheet, landing on `destination` inside the size box.
fn frame(index: u32, destination: CharaRect) -> CharaMotionFrame {
    CharaMotionFrame {
        src: Some(UvRect::from_pixels(index, 0, 1, 1, FRAME_COLOURS.len() as u32, 1)),
        source: (1.0, 1.0),
        destination,
        alpha: 255,
        angle_deg: 0.0,
    }
}

/// A motion over every frame of the sheet, on `timer`, filling the whole size box.
fn motion(tex: TextureId, timer: Option<TimerId>) -> CharaMotion {
    let whole = CharaRect { x: 0, y: 0, w: SIZE_BOX.0 as i32, h: SIZE_BOX.1 as i32 };
    CharaMotion {
        tex,
        timer,
        options: [0; 3],
        frames: (0..FRAME_COLOURS.len() as u32).map(|index| frame(index, whole)).collect(),
        frame_ms: FRAME_MS,
        once: 0,
        size: SIZE_BOX,
    }
}

/// One draw-list entry over a character body and the rectangle its destination holds.
fn object(rect: SkinRect, kind: CharaKind) -> SkinObject {
    let track = DestinationTrack {
        frames: vec![Keyframe { time_ms: 0, rect, clip: None, acc: Acc::default(), color: SkinColor::rgba(255, 255, 255, 255), angle_deg: 0.0 }],
        ..DestinationTrack::default()
    };
    SkinObject { id: "chara".to_owned(), layer: SkinLayer::Foreground, track, stretch: StretchKind::from_id(-1), body: Body::PmChara(PmCharaBody { kind }) }
}

/// The game state a character reads: the two borderline options its win and loss poses are gated on.
#[derive(Debug, Default, Clone, Copy)]
struct CharaState {
    cleared: bool,
    perfect: bool,
}

impl OffsetSource for CharaState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for CharaState {
    fn boolean(&self, id: i32) -> bool {
        match id {
            OPTION_1P_BORDER_OR_MORE => self.cleared,
            OPTION_1P_100 => self.perfect,
            negative if negative < 0 => !self.boolean(-negative),
            _ => false,
        }
    }
}

impl SkinStateSource for CharaState {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
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

/// Draws one character at `now_ms` with `timers` as the run left them.
fn draw_at(canvas: &mut CpuCanvas, object: &SkinObject, timers: &TimerState, now_ms: i64, state: &CharaState) -> bool {
    let frame = SkinFrame { now_ms, timers, state, lua: None, mouse: None, background: None, extra: FrameExtra::None };
    let viewport = SkinViewport::new((CANVAS.0 as f32, CANVAS.1 as f32), (CANVAS.0 as f32, CANVAS.1 as f32));
    with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
}

/// The columns of row `row` that hold `colour`, as `(first, last)`.
fn span_of(canvas: &CpuCanvas, row: u32, colour: Color) -> Option<(u32, u32)> {
    let first = (0..CANVAS.0).find(|column| canvas.pixel_at(*column, row) == colour)?;
    let last = (0..CANVAS.0).rev().find(|column| canvas.pixel_at(*column, row) == colour)?;
    Some((first, last))
}

/// The rows of column `column` that hold `colour`, as `(first, last)`.
fn rows_of(canvas: &CpuCanvas, column: u32, colour: Color) -> Option<(u32, u32)> {
    let first = (0..CANVAS.1).find(|row| canvas.pixel_at(column, *row) == colour)?;
    let last = (0..CANVAS.1).rev().find(|row| canvas.pixel_at(column, *row) == colour)?;
    Some((first, last))
}

#[test]
fn the_frame_on_screen_follows_the_timer_the_motion_is_bound_to() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let object = object(SkinRect::new(100.0, 500.0, 128.0, 128.0), CharaKind::Motion(vec![motion(tex, Some(timer_id::PM_CHARA_1P_NEUTRAL))]));
    let mut timers = TimerState::new();
    timers.set_on(timer_id::PM_CHARA_1P_NEUTRAL, 0);
    let state = CharaState::default();

    assert!(draw_at(&mut canvas, &object, &timers, 0, &state), "the first frame draws");
    assert_eq!(canvas.pixel_at(150, 150), FRAME_COLOURS[0], "the motion starts on its first frame");

    canvas.clear(Color { r: 0, g: 0, b: 0, a: 255 });
    assert!(draw_at(&mut canvas, &object, &timers, FRAME_MS, &state), "the second frame draws");
    assert_eq!(canvas.pixel_at(150, 150), FRAME_COLOURS[1], "one frame time later the next frame is up");

    canvas.clear(Color { r: 0, g: 0, b: 0, a: 255 });
    assert!(draw_at(&mut canvas, &object, &timers, FRAME_MS * 2, &state), "the animation repeats");
    assert_eq!(canvas.pixel_at(150, 150), FRAME_COLOURS[0], "a motion with no loop point repeats from its first frame");
}

#[test]
fn a_motion_whose_timer_is_off_draws_nothing() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let object = object(SkinRect::new(100.0, 500.0, 128.0, 128.0), CharaKind::Motion(vec![motion(tex, Some(timer_id::PM_CHARA_1P_NEUTRAL))]));

    assert!(!draw_at(&mut canvas, &object, &TimerState::new(), 0, &CharaState::default()), "a character nobody switched on is not on screen");
}

#[test]
fn a_frames_own_rectangle_is_mapped_into_the_destination_with_the_y_axis_turned_over() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let inset = CharaRect { x: 16, y: 8, w: 96, h: 120 };
    let mut single = motion(tex, None);
    single.frames = vec![frame(0, inset)];
    let object = object(SkinRect::new(200.0, 400.0, 64.0, 64.0), CharaKind::Motion(vec![single]));

    assert!(draw_at(&mut canvas, &object, &TimerState::new(), 0, &CharaState::default()), "a motion with no timer runs off the frame clock");

    let (first_column, last_column) = span_of(&canvas, 290, FRAME_COLOURS[0]).expect("the frame reached the canvas");
    assert_eq!((first_column, last_column), (208, 255), "x = 200 + 16 * 64 / 128 and w = 96 * 64 / 128");
    let (first_row, last_row) = rows_of(&canvas, 230, FRAME_COLOURS[0]).expect("the frame reached the canvas");
    assert_eq!((first_row, last_row), (260, 319), "the foot of the box sits on the destination's foot and the frame is 60 tall");
}

#[test]
fn a_frame_the_definition_sized_at_nothing_draws_nothing() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let mut empty = motion(tex, None);
    empty.frames = vec![CharaMotionFrame { src: None, ..frame(0, CharaRect { x: 0, y: 0, w: 128, h: 128 }) }];
    let object = object(SkinRect::new(100.0, 500.0, 128.0, 128.0), CharaKind::Motion(vec![empty]));

    assert!(!draw_at(&mut canvas, &object, &TimerState::new(), 0, &CharaState::default()), "an empty rectangle has nothing to show");
}

#[test]
fn a_still_type_fills_the_destination_it_was_given() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let kind = CharaKind::Still { tex, src: UvRect::from_pixels(1, 0, 1, 1, FRAME_COLOURS.len() as u32, 1), source: (1.0, 1.0) };
    let object = object(SkinRect::new(300.0, 300.0, 40.0, 20.0), kind);

    assert!(draw_at(&mut canvas, &object, &TimerState::new(), 0, &CharaState::default()), "a still character draws");
    let (first_column, last_column) = span_of(&canvas, 405, FRAME_COLOURS[1]).expect("the still frame reached the canvas");
    assert_eq!((first_column, last_column), (300, 339), "a still type is placed by the document's own destination alone");
}

#[test]
fn a_reaction_plays_once_and_then_gives_the_screen_back_to_the_neutral_pose() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let whole = CharaRect { x: 0, y: 0, w: SIZE_BOX.0 as i32, h: SIZE_BOX.1 as i32 };
    let mut neutral = motion(tex, Some(timer_id::PM_CHARA_1P_NEUTRAL));
    neutral.frames = vec![frame(0, whole)];
    let mut great = motion(tex, Some(timer_id::PM_CHARA_1P_GREAT));
    great.frames = vec![frame(1, whole)];
    let object = object(SkinRect::new(100.0, 500.0, 128.0, 128.0), CharaKind::Motion(vec![neutral, great]));
    let mut timers = TimerState::new();
    timers.set_on(timer_id::PM_CHARA_1P_NEUTRAL, 0);
    timers.set_on(timer_id::PM_CHARA_1P_GREAT, 1_000);
    let state = CharaState::default();

    assert!(draw_at(&mut canvas, &object, &timers, 1_000, &state), "the reaction draws");
    assert_eq!(canvas.pixel_at(150, 150), FRAME_COLOURS[1], "the reaction is the only pose on screen while it runs");

    canvas.clear(Color { r: 0, g: 0, b: 0, a: 255 });
    assert!(draw_at(&mut canvas, &object, &timers, 1_000 + FRAME_MS, &state), "the neutral pose comes back");
    assert_eq!(canvas.pixel_at(150, 150), FRAME_COLOURS[0], "one frame time is the whole reaction, so the neutral pose has the screen again");
}

#[test]
fn a_motion_gated_on_an_option_waits_for_it() {
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let tex = sheet(&mut canvas);
    let mut win = motion(tex, Some(timer_id::MUSIC_END));
    win.options = [OPTION_1P_BORDER_OR_MORE, -OPTION_1P_100, 0];
    let object = object(SkinRect::new(100.0, 500.0, 128.0, 128.0), CharaKind::Motion(vec![win]));
    let mut timers = TimerState::new();
    timers.set_on(timer_id::MUSIC_END, 0);

    assert!(!draw_at(&mut canvas, &object, &timers, 0, &CharaState { cleared: false, perfect: false }), "a run under the border has not won");
    assert!(!draw_at(&mut canvas, &object, &timers, 0, &CharaState { cleared: true, perfect: true }), "a perfect run takes the other pose");
    assert!(draw_at(&mut canvas, &object, &timers, 0, &CharaState { cleared: true, perfect: false }), "a cleared run wins");
}

#[test]
fn the_loop_point_splits_a_motion_into_a_run_once_and_a_run_that_repeats() {
    let played_once = 2;
    let motion = CharaMotion {
        tex: TextureId(0),
        timer: None,
        options: [0; 3],
        frames: (0..4).map(|index| frame(index % FRAME_COLOURS.len() as u32, CharaRect { x: 0, y: 0, w: 128, h: 128 })).collect(),
        frame_ms: FRAME_MS,
        once: played_once,
        size: SIZE_BOX,
    };

    let played: Vec<usize> = (0..6).map(|step| motion.frame_at(FRAME_MS * step)).collect();
    assert_eq!(played, vec![0, 1, 2, 3, 2, 3], "the frames up to the loop point play once and the rest repeat");
}

#[test]
fn the_play_bindings_follow_the_reference_for_both_sides() {
    let plain = [0; 3];
    assert_eq!(play_binding(false, 1), Some((timer_id::PM_CHARA_1P_NEUTRAL, plain)));
    assert_eq!(play_binding(false, 6), Some((timer_id::PM_CHARA_1P_FEVER, plain)));
    assert_eq!(play_binding(false, 7), Some((timer_id::PM_CHARA_1P_GREAT, plain)));
    assert_eq!(play_binding(false, 8), Some((timer_id::PM_CHARA_1P_GOOD, plain)));
    assert_eq!(play_binding(false, 10), Some((timer_id::PM_CHARA_1P_BAD, plain)));
    assert_eq!(play_binding(false, 15), Some((timer_id::MUSIC_END, [OPTION_1P_BORDER_OR_MORE, -OPTION_1P_100, 0])));
    assert_eq!(play_binding(false, 16), Some((timer_id::MUSIC_END, [-OPTION_1P_BORDER_OR_MORE, 0, 0])));
    assert_eq!(play_binding(false, 17), Some((timer_id::MUSIC_END, [OPTION_1P_100, 0, 0])));
    assert_eq!(play_binding(false, 3), None, "the first player has no timer for the nuisance motion");

    assert_eq!(play_binding(true, 1), Some((timer_id::PM_CHARA_2P_NEUTRAL, plain)));
    assert_eq!(play_binding(true, 7), Some((timer_id::PM_CHARA_2P_GREAT, plain)));
    assert_eq!(play_binding(true, 10), Some((timer_id::PM_CHARA_2P_BAD, plain)));
    assert_eq!(play_binding(true, 15), Some((timer_id::MUSIC_END, [-OPTION_1P_BORDER_OR_MORE, 0, 0])), "the opponent wins when the run does not");
    assert_eq!(play_binding(true, 6), None, "the opponent has no fever timer");
}

/// A HUD snapshot with one judgement reported and a gauge to go with it.
fn hud(counts: [u32; 6], last_judge: Option<u8>, gauge: f32) -> HudView<'static> {
    HudView {
        mode_label: "9K",
        combo: counts.iter().sum(),
        last_judge,
        last_fast: false,
        fast: [0; crate::result::LANE_KIND_COUNT],
        slow: [0; crate::result::LANE_KIND_COUNT],
        counts,
        ex_score: 0,
        gauge,
        green_number: 0.0,
        white_number: 0.0,
        judge_text_y: 0.0,
        max_ex: 0,
        best_ex: None,
        pace: None,
    }
}

/// A run with no lane state to report, which is what the character timers are exercised with.
fn no_lanes() -> PlayLanes<'static> {
    PlayLanes { lanes: &[], judged_side: 0 }
}

#[test]
fn a_run_starting_puts_both_characters_in_their_neutral_pose() {
    let mut memory = PlayTimers::new();
    let mut timers = TimerState::new();
    timers.set_on(timer_id::MUSIC_END, 0);
    memory.start(&mut timers, 1_000);

    assert_eq!(timers.get(timer_id::PM_CHARA_1P_NEUTRAL), Some(1_000));
    assert_eq!(timers.get(timer_id::PM_CHARA_2P_NEUTRAL), Some(1_000));
    assert!(timers.is_off(timer_id::MUSIC_END), "a run that is starting has not ended");
}

#[test]
fn each_judgement_restarts_the_character_timer_it_belongs_to() {
    let cases = [
        (0u8, 50.0, timer_id::PM_CHARA_1P_GREAT),
        (1, 50.0, timer_id::PM_CHARA_1P_GREAT),
        (0, 100.0, timer_id::PM_CHARA_1P_FEVER),
        (2, 50.0, timer_id::PM_CHARA_1P_GOOD),
        (3, 50.0, timer_id::PM_CHARA_1P_BAD),
        (4, 50.0, timer_id::PM_CHARA_1P_BAD),
    ];
    for (judge, gauge, expected) in cases {
        let mut memory = PlayTimers::new();
        let mut timers = TimerState::new();
        memory.update(&mut timers, &hud([1, 0, 0, 0, 0, 0], None, gauge), 100, 1_000, &no_lanes());
        memory.update(&mut timers, &hud([2, 0, 0, 0, 0, 0], Some(judge), gauge), 100, 1_500, &no_lanes());

        assert_eq!(timers.get(expected), Some(1_500), "judgement {judge} at gauge {gauge} did not reach its own timer");
    }
}

#[test]
fn the_last_note_being_judged_ends_the_music_and_clears_the_character_band() {
    let mut memory = PlayTimers::new();
    let mut timers = TimerState::new();
    memory.start(&mut timers, 0);
    memory.update(&mut timers, &hud([1, 0, 0, 0, 0, 0], Some(0), 50.0), 2, 1_000, &no_lanes());
    assert!(timers.is_off(timer_id::MUSIC_END), "a chart with a note left has not ended");

    memory.update(&mut timers, &hud([2, 0, 0, 0, 0, 0], Some(0), 50.0), 2, 1_500, &no_lanes());
    assert_eq!(timers.get(timer_id::MUSIC_END), Some(1_500), "the last note judged is the end of the chart");
    for timer in [timer_id::PM_CHARA_1P_NEUTRAL, timer_id::PM_CHARA_1P_GREAT, timer_id::PM_CHARA_2P_NEUTRAL, timer_id::PM_CHARA_DANCE] {
        assert!(timers.is_off(timer), "the character band is cleared so the win and loss poses have the screen");
    }
}
