//! The turntable with START held: a shade moved a thousandth of the lane at a time, a step a press
//! and a repeat while it is held, with the green number left where it was.
//!
//! The clock is the one each frame is handed, so a hold lasts exactly as long as a test says.

use std::time::{Duration, Instant};

use rbms_config::{Config, FixHiSpeed, HISPEED_MAX, HISPEED_MIN, JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, number_steps};
use rbms_model::Mode;
use rbms_play::SessionOptions;
use winit::keyboard::KeyCode;

use super::repeat::{REPEAT_DELAY, REPEAT_EVERY};
use super::*;
use crate::App;
use crate::stage::play::hold_tests::{app_on, enter, green, hispeed, run_of};
use crate::stage::play::shade_is_changing;
use crate::stage::select::tests::{press, release};
use crate::stage::{FrameCtx, StageHandler};

/// The keys START and SELECT ship on, and the two keys the turntable ships on, one a direction.
const START_KEY: KeyCode = KeyCode::KeyA;
const SELECT_KEY: KeyCode = KeyCode::KeyW;
const TURN_FORWARD_KEY: KeyCode = KeyCode::ShiftLeft;
const TURN_BACKWARD_KEY: KeyCode = KeyCode::ControlLeft;
const TURNTABLE_LANE: usize = 7;

/// The keys the judge offset ships on.
const OFFSET_DOWN_KEY: KeyCode = KeyCode::Digit9;
const OFFSET_UP_KEY: KeyCode = KeyCode::Digit0;

/// A frame of sixty to the second, near enough.
const FRAME: Duration = Duration::from_millis(16);

/// How long these tests wait between two holds of START, so the second press is not the other half
/// of the pair that switches the lane cover.
const BETWEEN_HOLDS: Duration = Duration::from_secs(2);

/// The white number these tests start the lane cover at, and the figures of the lift and the hidden
/// cover.
const COVER_AT: i64 = 300;
const LIFT_AT: i64 = 200;
const HIDDEN_AT: i64 = 150;

/// The steps from one end of a shade to the other.
const WHOLE_SHADE: i64 = 1_000;

/// How far apart two green numbers may read and still be the one number, for the rounding of the
/// travel time they are cut from.
const GREEN_ROUNDING: i32 = 1;

/// The tempo of the chart these tests play.
const CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 150\n#WAV01 a.wav\n#00111:0101\n#00118:01\n";

/// The settings these tests start from: the lane cover on at [`COVER_AT`], the lift and the hidden
/// cover off at their figures, and nothing holding the green number.
fn settings() -> Config {
    let mut config = Config::default();
    config.play.keep_green_number = false;
    config.play.fix_hispeed = FixHiSpeed::Off;
    config.play.constant_speed = false;
    config.play.enable_cover = true;
    config.play.enable_lift = false;
    config.play.enable_hidden = false;
    config.judge.auto_offset = false;
    rbms_config::set_number_steps(&mut config, SettingId::LaneCover, COVER_AT);
    rbms_config::set_number_steps(&mut config, SettingId::Lift, LIFT_AT);
    rbms_config::set_number_steps(&mut config, SettingId::Hidden, HIDDEN_AT);
    config
}

/// An app playing seven keys by hand on `config`, with the keys as they ship.
fn app(tag: &str, config: Config) -> App {
    let mut app = app_on(&format!("turn-{tag}"), config);
    app.shared.keyconfig = crate::keyconfig::KeyConfig::default();
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    assert_eq!(app.shared.active_reverse_keys, vec![(TURN_BACKWARD_KEY, TURNTABLE_LANE)], "the turntable does not ship with a key for its other direction");
    app
}

/// A run of the chart, entered the way the application enters one.
fn run(app: &mut App) -> PlayState {
    let mut state = run_of(CHART, SessionOptions::default());
    enter(&mut state, app);
    state
}

fn shade(app: &App, id: SettingId) -> i64 {
    number_steps(&app.shared.config, id).expect("a shade holds a number")
}

fn white(app: &App) -> i64 {
    shade(app, SettingId::LaneCover)
}

fn lift(app: &App) -> i64 {
    shade(app, SettingId::Lift)
}

fn hidden(app: &App) -> i64 {
    shade(app, SettingId::Hidden)
}

fn hold(app: &mut App, code: KeyCode, down: bool) {
    app.shared.note_key(&if down { press(code) } else { release(code) });
}

/// Hold `keys` down and read two frames, so the controls read the turntable from the next one on.
fn hold_down(state: &mut PlayState, app: &mut App, keys: &[KeyCode], now: &mut Instant) {
    for code in keys {
        hold(app, *code, true);
    }
    for _ in 0..2 {
        state.held_controls(&mut app.shared, *now);
        *now += FRAME;
    }
}

/// Let every key go and read a frame.
fn let_go(state: &mut PlayState, app: &mut App, now: &mut Instant) {
    app.shared.release_held_inputs();
    state.held_controls(&mut app.shared, *now);
    *now += FRAME;
}

/// Hold `code` down for `length`, reading a frame every [`FRAME`], then let it go and read one more.
fn tap(state: &mut PlayState, app: &mut App, code: KeyCode, length: Duration, now: &mut Instant) {
    hold(app, code, true);
    let down_at = *now;
    while now.duration_since(down_at) <= length {
        state.held_controls(&mut app.shared, *now);
        *now += FRAME;
    }
    hold(app, code, false);
    state.held_controls(&mut app.shared, *now);
    *now += FRAME;
}

/// START and one press of the turntable is one of the white number, however long a press of
/// ordinary length lasts, and what is stored is on the step a skin cuts the number from. The
/// turntable's first key brings the cover down over more of the lane and its second takes it back.
#[test]
fn start_and_a_press_of_the_turntable_moves_the_lane_cover_by_one() {
    let mut app = app("cover-one", settings());
    let mut state = run(&mut app);
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    assert_eq!(white(&app), COVER_AT, "START alone moved the cover");

    for (presses, millis) in [16, 80, 120, 250].into_iter().enumerate() {
        tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(millis), &mut now);
        let wanted = COVER_AT + presses as i64 + 1;
        assert_eq!(white(&app), wanted, "a press of {millis} ms");
        assert_eq!((app.shared.config.play.cover * WHOLE_SHADE as f32) as i64, wanted, "the white number a skin cuts from what is stored");
    }
    let lowered = white(&app);
    for presses in 1..=6 {
        tap(&mut state, &mut app, TURN_BACKWARD_KEY, Duration::from_millis(100), &mut now);
        assert_eq!(white(&app), lowered - presses, "the turntable's other key did not take a step back");
    }
    assert_eq!((lift(&app), hidden(&app)), (LIFT_AT, HIDDEN_AT), "the turntable with START alone moved a shade that is off");
}

/// START, SELECT and one press of the turntable is one of the lift's figure, the turntable's first
/// key lowering the line as it lowers the cover, and the lane cover stays where it is.
#[test]
fn start_select_and_a_press_of_the_turntable_moves_the_lift_by_one() {
    let mut config = settings();
    config.play.enable_lift = true;
    let mut app = app("lift-one", config);
    let mut state = run(&mut app);
    let (number, speed) = (green(&mut state, &app), hispeed(&app));
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY, SELECT_KEY], &mut now);
    assert_eq!(lift(&app), LIFT_AT, "START with SELECT alone moved the lift");

    for presses in 1..=3 {
        tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(90), &mut now);
        assert_eq!(lift(&app), LIFT_AT - presses, "the turntable's first key did not lower the line a step");
    }
    for presses in 1..=5 {
        tap(&mut state, &mut app, TURN_BACKWARD_KEY, Duration::from_millis(90), &mut now);
        assert_eq!(lift(&app), LIFT_AT - 3 + presses, "the turntable's other key did not raise the line a step");
    }
    assert_eq!((white(&app), hidden(&app)), (COVER_AT, HIDDEN_AT), "the lift's combination moved another shade");
    assert_eq!(green(&mut state, &app), number, "the lift is no part of the green number");
    assert!((hispeed(&app) - speed).abs() < 1e-9, "the lift moved a speed it has no part in");

    hold(&mut app, SELECT_KEY, false);
    state.held_controls(&mut app.shared, now);
    now += FRAME;
    tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(90), &mut now);
    assert_eq!((white(&app), lift(&app)), (COVER_AT + 1, LIFT_AT + 2), "with SELECT up again the turntable is the lane cover's");
}

/// A key of the turntable that stays down moves nothing more until the wait is over, then a step
/// every fifty milliseconds, then more than one a repeat; read at any instant, the cover stands
/// where the length of the hold puts it.
#[test]
fn a_held_turntable_waits_and_then_repeats_and_quickens() {
    let mut config = settings();
    rbms_config::set_number_steps(&mut config, SettingId::LaneCover, 0);
    let mut app = app("repeat", config);
    let mut state = run(&mut app);
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    hold(&mut app, TURN_FORWARD_KEY, true);
    let down = now;
    let at = |state: &mut PlayState, app: &mut App, held: Duration| {
        state.held_controls(&mut app.shared, down + held);
        white(app)
    };
    assert_eq!(at(&mut state, &mut app, Duration::ZERO), 1, "the press is a step");
    assert_eq!(at(&mut state, &mut app, FRAME), 1);
    assert_eq!(at(&mut state, &mut app, REPEAT_DELAY - Duration::from_millis(1)), 1, "the hold repeated before the wait was over");
    assert_eq!(at(&mut state, &mut app, REPEAT_DELAY), 2, "the first repeat comes as the wait ends");
    assert_eq!(at(&mut state, &mut app, REPEAT_DELAY + REPEAT_EVERY - Duration::from_millis(1)), 2);
    assert_eq!(at(&mut state, &mut app, REPEAT_DELAY + REPEAT_EVERY), 3);
    assert_eq!(at(&mut state, &mut app, REPEAT_DELAY + REPEAT_EVERY * 10), 12, "a frame that comes late takes every step the time earned");
    assert_eq!(at(&mut state, &mut app, Duration::from_millis(1_499)), 25);
    assert_eq!(at(&mut state, &mut app, Duration::from_millis(1_500)), 29, "a hold this long takes four steps a repeat");
    assert_eq!(at(&mut state, &mut app, Duration::from_millis(3_000)), 155, "and ten once it is plainly a long way to go");
}

/// Held long enough the cover reaches the end of the lane and stops there, in under eight seconds
/// from the other end, and the other key takes it all the way back.
#[test]
fn a_long_hold_crosses_the_lane_and_stops_at_its_end() {
    const LONG_HOLD: Duration = Duration::from_secs(8);
    let mut config = settings();
    rbms_config::set_number_steps(&mut config, SettingId::LaneCover, 0);
    let mut app = app("ends", config);
    let mut state = run(&mut app);
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    tap(&mut state, &mut app, TURN_BACKWARD_KEY, Duration::from_millis(600), &mut now);
    assert_eq!(white(&app), 0, "the cover went past the top of the lane");
    tap(&mut state, &mut app, TURN_FORWARD_KEY, LONG_HOLD, &mut now);
    assert_eq!(white(&app), WHOLE_SHADE, "eight seconds did not bring the cover to the foot of the lane");
    assert_eq!(app.shared.config.play.cover, 1.0);
    tap(&mut state, &mut app, TURN_BACKWARD_KEY, LONG_HOLD, &mut now);
    assert_eq!(white(&app), 0);
}

/// An analogue turntable moves a step a tick, either way, and what it turned before START went down
/// is not a move. A key of the keyboard still steps beside it.
#[test]
fn an_analogue_turntable_moves_a_step_a_tick() {
    let mut config = settings();
    config.play.enable_lift = true;
    let mut app = app("analogue", config);
    let mut state = run(&mut app);
    let mut now = Instant::now();
    let mut frame = |state: &mut PlayState, app: &mut App, ticks: i32| {
        state.read_held(&mut app.shared, now, Some(ticks));
        now += FRAME;
    };
    frame(&mut state, &mut app, 40);
    assert_eq!(white(&app), COVER_AT, "the turntable moved the cover with START up");
    hold(&mut app, START_KEY, true);
    frame(&mut state, &mut app, 40);
    assert_eq!(white(&app), COVER_AT, "what the turntable turned before START went down moved the cover");

    frame(&mut state, &mut app, 3);
    assert_eq!(white(&app), COVER_AT + 3);
    frame(&mut state, &mut app, 0);
    assert_eq!(white(&app), COVER_AT + 3, "a turntable that stands still moved the cover");
    frame(&mut state, &mut app, -5);
    assert_eq!(white(&app), COVER_AT - 2);
    hold(&mut app, TURN_FORWARD_KEY, true);
    frame(&mut state, &mut app, 1);
    assert_eq!(white(&app), COVER_AT, "a key of the keyboard and a tick of the turntable are a step each");
    hold(&mut app, TURN_FORWARD_KEY, false);
    frame(&mut state, &mut app, 0);

    hold(&mut app, SELECT_KEY, true);
    frame(&mut state, &mut app, 9);
    assert_eq!(lift(&app), LIFT_AT, "the frame SELECT joins START on is not read");
    frame(&mut state, &mut app, 4);
    frame(&mut state, &mut app, -1);
    assert_eq!((white(&app), lift(&app)), (COVER_AT, LIFT_AT - 3), "START, SELECT and the turntable did not move the lift a step a tick");
}

/// How these tests name the ways a run keeps, or does not keep, a green number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keeps {
    /// The KEEP GREEN NUMBER row is on.
    Held,
    /// The row is off and HI-SPEED FIX names a tempo.
    Pinned,
    /// The row is off and HI-SPEED FIX is off.
    Free,
}

fn keeping(keeps: Keeps, constant: bool) -> Config {
    let mut config = settings();
    config.play.keep_green_number = keeps == Keeps::Held;
    config.play.fix_hispeed = if keeps == Keeps::Free { FixHiSpeed::Off } else { FixHiSpeed::MainBpm };
    config.play.constant_speed = constant;
    config
}

/// The lane cover moved with the turntable leaves the green number on show where it was and moves
/// the scroll speed, on every kind of run: one that holds its number, one pinned to a tempo, one
/// with a free speed, and each of them under CONSTANT.
#[test]
fn the_turntable_moves_the_cover_and_the_speed_and_leaves_the_green_number() {
    for constant in [false, true] {
        for keeps in [Keeps::Held, Keeps::Pinned, Keeps::Free] {
            let mut app = app("green", keeping(keeps, constant));
            let mut state = run(&mut app);
            let (number, speed) = (green(&mut state, &app), hispeed(&app));
            let mut now = Instant::now();
            hold_down(&mut state, &mut app, &[START_KEY], &mut now);
            let what = format!("{keeps:?}, constant {constant}");

            tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(100), &mut now);
            assert_eq!(white(&app), COVER_AT + 1, "{what}");
            assert!(
                (green(&mut state, &app) - number).abs() <= GREEN_ROUNDING,
                "{what}: one step moved the number from {number} to {}",
                green(&mut state, &app)
            );
            assert!(hispeed(&app) < speed, "{what}: more cover with the same number is a slower scroll");

            state.read_held(&mut app.shared, now, Some(149));
            assert_eq!(white(&app), COVER_AT + 150, "{what}");
            assert!((green(&mut state, &app) - number).abs() <= GREEN_ROUNDING, "{what}: the number went from {number} to {}", green(&mut state, &app));
            let covered = hispeed(&app);
            assert!((covered - speed * 0.55 / 0.7).abs() < 1e-3, "{what}: the speed is {covered}, and {} keeps the number", speed * 0.55 / 0.7);

            state.read_held(&mut app.shared, now + FRAME, Some(-250));
            assert_eq!(white(&app), COVER_AT - 100, "{what}");
            assert!((green(&mut state, &app) - number).abs() <= GREEN_ROUNDING, "{what}: the number went from {number} to {}", green(&mut state, &app));
            assert!(hispeed(&app) > speed, "{what}: less cover with the same number is a faster scroll");
        }
    }
}

/// On a run with a free speed the number the turntable keeps is the one on show when it first moves
/// the cover under a hold of START: a speed key pressed under the same hold chooses a new number,
/// and the turntable keeps that one. Once START is up, the bound keys of the cover are what they
/// were, and move the number as the reference does.
#[test]
fn a_free_speed_keeps_the_number_it_has_when_the_turntable_starts() {
    let mut app = app("free", keeping(Keeps::Free, false));
    let mut state = run(&mut app);
    let first = green(&mut state, &app);
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    state.read_held(&mut app.shared, now, Some(50));
    assert!((green(&mut state, &app) - first).abs() <= GREEN_ROUNDING);

    let faster = app.shared.active_keys.iter().find(|(_, lane)| *lane == 1).map(|(code, _)| *code).expect("the second key has a key");
    tap(&mut state, &mut app, faster, Duration::from_millis(40), &mut now);
    let second = green(&mut state, &app);
    assert!(second < first - GREEN_ROUNDING, "START and the faster key did not choose a smaller number");
    state.read_held(&mut app.shared, now, Some(-80));
    assert!(
        (green(&mut state, &app) - second).abs() <= GREEN_ROUNDING,
        "the turntable went back to the number it held before the speed key: {}",
        green(&mut state, &app)
    );

    let_go(&mut state, &mut app, &mut now);
    let speed = hispeed(&app);
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    assert_eq!(hispeed(&app), speed, "a bound cover key moved a free speed");
    assert!(green(&mut state, &app) < second, "a bound cover key on a free speed leaves the number to the cover");
}

/// A speed that would have to leave its range to keep the number stops at the end of it, the number
/// on show is then the one the speed really comes to, and the speed comes back as the cover does.
#[test]
fn a_speed_at_the_end_of_its_range_stops_there_and_comes_back_with_the_cover() {
    const NEARLY_FASTEST: f64 = 19.0;
    for keeps in [Keeps::Pinned, Keeps::Free] {
        let mut config = keeping(keeps, false);
        config.play.hispeed = NEARLY_FASTEST;
        let mut app = app("limit", config);
        let mut state = run(&mut app);
        let number = green(&mut state, &app);
        let mut now = Instant::now();
        hold_down(&mut state, &mut app, &[START_KEY], &mut now);

        state.read_held(&mut app.shared, now, Some(-(COVER_AT as i32)));
        assert_eq!(white(&app), 0);
        assert_eq!(hispeed(&app), HISPEED_MAX, "{keeps:?}: the speed did not stop at the end of its range");
        let on_show = green(&mut state, &app);
        assert!(on_show > number + GREEN_ROUNDING, "{keeps:?}: the screen still shows {on_show} for a number of {number} no speed in range keeps");
        let really = rbms_config::green_of_duration(rbms_config::travel_duration_ms(150.0, HISPEED_MAX, Some(0.0)));
        assert!((on_show - really).abs() <= GREEN_ROUNDING, "{keeps:?}: {on_show} on show, and the speed comes to {really}");

        state.read_held(&mut app.shared, now + FRAME, Some(COVER_AT as i32));
        assert!((hispeed(&app) - NEARLY_FASTEST).abs() < 1e-6, "{keeps:?}: the speed came back to {}", hispeed(&app));
        assert!((green(&mut state, &app) - number).abs() <= GREEN_ROUNDING);
    }
}

/// The number a free speed is held to outlasts the hold of START it was taken under. A lane cover
/// turned to the end of the lane, where no speed in range keeps the number, and let go of there
/// comes back to the speed and the number it had when START is held again and the cover is turned
/// back: the speed that stopped at the end of its range is not taken for the one the player chose.
#[test]
fn a_free_speed_stopped_at_the_end_comes_back_under_another_hold_of_start() {
    let to_the_end = (WHOLE_SHADE - COVER_AT) as i32;
    let mut app = app("another-hold", keeping(Keeps::Free, false));
    let mut state = run(&mut app);
    let (number, speed) = (green(&mut state, &app), hispeed(&app));
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    state.read_held(&mut app.shared, now, Some(to_the_end));
    assert_eq!(white(&app), WHOLE_SHADE);
    assert_eq!(hispeed(&app), HISPEED_MIN, "the speed did not stop at the end of its range under a lane that is all cover");

    let_go(&mut state, &mut app, &mut now);
    now += BETWEEN_HOLDS;
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    state.read_held(&mut app.shared, now, Some(-to_the_end));
    assert_eq!(white(&app), COVER_AT);
    assert!((hispeed(&app) - speed).abs() < 1e-6, "the speed came back to {} and was {speed}", hispeed(&app));
    assert!((green(&mut state, &app) - number).abs() <= GREEN_ROUNDING, "the number came back to {} and was {number}", green(&mut state, &app));
}

/// What outlasts the hold is the number as the turntable left it. A cover or a speed that
/// something else moved between two holds of START chose a new number, and the turntable keeps
/// that one under the next hold.
#[test]
fn a_number_chosen_between_two_holds_of_start_is_the_one_the_turntable_keeps() {
    for moved_by in [ControlAction::CoverUp, ControlAction::HiSpeedUp] {
        let mut app = app("between-holds", keeping(Keeps::Free, false));
        let mut state = run(&mut app);
        let first = green(&mut state, &app);
        let mut now = Instant::now();
        hold_down(&mut state, &mut app, &[START_KEY], &mut now);
        state.read_held(&mut app.shared, now, Some(50));
        let_go(&mut state, &mut app, &mut now);

        state.in_play_control(&mut app.shared, moved_by);
        let chosen = green(&mut state, &app);
        assert!((chosen - first).abs() > GREEN_ROUNDING, "{moved_by:?} chose no new number");
        now += BETWEEN_HOLDS;
        hold_down(&mut state, &mut app, &[START_KEY], &mut now);
        state.read_held(&mut app.shared, now, Some(-80));
        let kept = green(&mut state, &app);
        assert!((kept - chosen).abs() <= GREEN_ROUNDING, "after {moved_by:?} the turntable went to {kept}, and {chosen} was chosen");
    }
}

/// A shade that is switched off moves as one that is on and stays off, and which shade START alone
/// moves follows the reference: the lane cover while it is on or nothing else is, else the lift,
/// else the hidden cover. START with SELECT is the lift whatever is on.
#[test]
fn a_shade_that_is_off_moves_and_stays_off() {
    let mut config = settings();
    config.play.enable_cover = false;
    let mut app = app("off", config);
    let mut state = run(&mut app);
    let speed = hispeed(&app);
    let mut now = Instant::now();
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(80), &mut now);
    assert_eq!(white(&app), COVER_AT + 1, "with nothing on, the lane cover is what moves");
    assert!(!app.shared.config.play.enable_cover, "moving a cover that is off switched it on");
    assert_eq!(hispeed(&app), speed, "a cover that is off moved the speed");
    let moved_us = state.shade_moved_us.expect("the move put the figures of the shades up");
    assert!(shade_is_changing(Some(moved_us), moved_us + 1_400_000), "the figures went before a second and a half was up");
    assert!(!shade_is_changing(Some(moved_us), moved_us + 1_600_000));

    hold(&mut app, SELECT_KEY, true);
    state.held_controls(&mut app.shared, now);
    now += FRAME;
    tap(&mut state, &mut app, TURN_BACKWARD_KEY, Duration::from_millis(80), &mut now);
    assert_eq!(lift(&app), LIFT_AT + 1, "START with SELECT did not move a lift that is off");
    assert!(!app.shared.config.play.enable_lift, "moving a lift that is off switched it on");
    let_go(&mut state, &mut app, &mut now);

    app.shared.config.play.enable_lift = true;
    now += BETWEEN_HOLDS;
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(80), &mut now);
    assert_eq!((white(&app), lift(&app)), (COVER_AT + 1, LIFT_AT), "with the lane cover off and the lift on, START alone moves the lift, down");
    let_go(&mut state, &mut app, &mut now);

    app.shared.config.play.enable_lift = false;
    app.shared.config.play.enable_hidden = true;
    now += BETWEEN_HOLDS;
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(80), &mut now);
    assert_eq!((white(&app), lift(&app), hidden(&app)), (COVER_AT + 1, LIFT_AT, HIDDEN_AT - 1), "with only the hidden cover on, START alone moves it");
}

/// The combinations beside these are what they were: SELECT with the turntable moves the travel
/// time a pinned speed holds a millisecond every fifty and no shade, and START twice switches the
/// lane cover. SELECT tapped under a held START is the way to the lift, and switches nothing.
#[test]
fn the_other_combinations_are_what_they_were() {
    let mut app = app("others", keeping(Keeps::Pinned, false));
    let mut state = run(&mut app);
    let mut now = Instant::now();
    let pinned = state.run_speed(&app.shared).fixed.expect("the run pins its speed").green;
    hold_down(&mut state, &mut app, &[SELECT_KEY], &mut now);
    hold(&mut app, TURN_FORWARD_KEY, true);
    for _ in 0..4 {
        now += Duration::from_millis(60);
        state.held_controls(&mut app.shared, now);
    }
    let after = state.run_speed(&app.shared).fixed.expect("the run still pins its speed").green;
    assert!((after - (pinned + 4.0)).abs() < 1e-9, "SELECT and the turntable moved the travel time from {pinned} to {after}");
    assert_eq!((white(&app), lift(&app)), (COVER_AT, LIFT_AT), "SELECT and the turntable moved a shade");
    let_go(&mut state, &mut app, &mut now);

    now += BETWEEN_HOLDS;
    hold_down(&mut state, &mut app, &[START_KEY], &mut now);
    for _ in 0..2 {
        hold(&mut app, SELECT_KEY, true);
        state.held_controls(&mut app.shared, now);
        now += FRAME;
        hold(&mut app, SELECT_KEY, false);
        state.held_controls(&mut app.shared, now);
        now += FRAME;
    }
    assert!(app.shared.config.play.enable_cover, "SELECT tapped under START switched the lane cover off");
    let_go(&mut state, &mut app, &mut now);

    now += BETWEEN_HOLDS;
    for _ in 0..2 {
        hold(&mut app, START_KEY, true);
        state.held_controls(&mut app.shared, now);
        now += FRAME;
        hold(&mut app, START_KEY, false);
        state.held_controls(&mut app.shared, now);
        now += Duration::from_millis(100);
    }
    assert!(!app.shared.config.play.enable_cover, "START twice did not switch the lane cover off");
}

/// A run that plays itself, and one that reproduces a replay, have their shades left alone by the
/// turntable with START, as their speed is left alone by START with a key.
#[test]
fn the_turntable_moves_nothing_on_a_run_the_player_is_not_playing() {
    let mut auto = app("auto", settings());
    auto.shared.config.play.autoplay = true;
    let mut replayed = app("replay", settings());
    replayed.shared.replay = Some(rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: rbms_store::ReplayJudge::default(),
        events: Vec::new(),
    });
    for mut app in [auto, replayed] {
        let mut state = run_of(CHART, SessionOptions::default());
        let mut now = Instant::now();
        hold_down(&mut state, &mut app, &[START_KEY], &mut now);
        tap(&mut state, &mut app, TURN_FORWARD_KEY, Duration::from_millis(500), &mut now);
        state.read_held(&mut app.shared, now, Some(30));
        hold(&mut app, SELECT_KEY, true);
        state.read_held(&mut app.shared, now + FRAME, Some(30));
        state.read_held(&mut app.shared, now + FRAME * 2, Some(30));
        assert_eq!((white(&app), lift(&app)), (COVER_AT, LIFT_AT));
    }
}

/// One frame of the screen at `now`.
fn frame(state: &mut PlayState, app: &mut App, now: Instant) {
    state.update(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
}

/// One key going down on the play screen at `now`, as the window reports it.
fn key_down(state: &mut PlayState, app: &mut App, code: KeyCode, now: Instant) {
    app.shared.note_key(&press(code));
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(code));
}

/// The judge offset moves a millisecond a press from the keys it ships on, and a key that stays
/// down repeats after the wait at a pace that does not quicken.
#[test]
fn the_offset_keys_ship_bound_and_repeat_while_held() {
    let mut app = app("offset", settings());
    let mut state = run_of(CHART, SessionOptions::default());
    app.shared.config.judge.offset_ms = 0;
    let down = Instant::now();
    key_down(&mut state, &mut app, OFFSET_UP_KEY, down);
    assert_eq!(app.shared.config.judge.offset_ms, 1, "the press is a millisecond");
    let at = |state: &mut PlayState, app: &mut App, held: Duration| {
        frame(state, app, down + held);
        app.shared.config.judge.offset_ms
    };
    assert_eq!(at(&mut state, &mut app, FRAME), 1, "the frame after the press counted it again");
    assert_eq!(at(&mut state, &mut app, FRAME + REPEAT_DELAY - Duration::from_millis(1)), 1);
    assert_eq!(at(&mut state, &mut app, FRAME + REPEAT_DELAY), 2);
    assert_eq!(at(&mut state, &mut app, FRAME + REPEAT_DELAY + REPEAT_EVERY * 4), 6);
    assert_eq!(at(&mut state, &mut app, FRAME + Duration::from_millis(3_300)), 62, "a step every fifty milliseconds, however long the hold");

    app.shared.note_key(&release(OFFSET_UP_KEY));
    assert_eq!(at(&mut state, &mut app, FRAME + Duration::from_millis(3_400)), 62, "a key that is up went on repeating");
    let later = down + Duration::from_secs(5);
    key_down(&mut state, &mut app, OFFSET_DOWN_KEY, later);
    key_down(&mut state, &mut app, OFFSET_DOWN_KEY, later);
    assert_eq!(app.shared.config.judge.offset_ms, 60, "the other key takes a millisecond off a press");
}

/// A key held to either end of the offset's range stops there.
#[test]
fn a_held_offset_key_stops_at_the_end_of_the_range() {
    const LONG_HOLD: Duration = Duration::from_secs(30);
    for (code, end) in [(OFFSET_UP_KEY, JUDGE_OFFSET_MAX_MS), (OFFSET_DOWN_KEY, JUDGE_OFFSET_MIN_MS)] {
        let mut app = app("offset-ends", settings());
        let mut state = run_of(CHART, SessionOptions::default());
        app.shared.config.judge.offset_ms = 0;
        let down = Instant::now();
        key_down(&mut state, &mut app, code, down);
        frame(&mut state, &mut app, down + FRAME);
        frame(&mut state, &mut app, down + LONG_HOLD);
        assert_eq!(app.shared.config.judge.offset_ms, end);
        frame(&mut state, &mut app, down + LONG_HOLD * 2);
        assert_eq!(app.shared.config.judge.offset_ms, end, "a key held at the end of the range took the offset past it");
    }
}

/// A run that plays itself leaves the offset alone under a held key as it does under a press.
#[test]
fn a_held_offset_key_moves_nothing_on_a_run_that_plays_itself() {
    let mut app = app("offset-auto", settings());
    app.shared.config.play.autoplay = true;
    app.shared.config.judge.offset_ms = 7;
    let mut state = run_of(CHART, SessionOptions::default());
    let down = Instant::now();
    key_down(&mut state, &mut app, OFFSET_UP_KEY, down);
    frame(&mut state, &mut app, down + FRAME);
    frame(&mut state, &mut app, down + Duration::from_secs(3));
    assert_eq!(app.shared.config.judge.offset_ms, 7);
}

/// A controller button bound to an offset key repeats as the key does while the controller holds
/// it, which a run with no controller never sees: the press alone moves the offset.
#[test]
fn a_press_from_a_controller_with_nothing_held_is_one_step() {
    let mut app = app("offset-pad", settings());
    let mut state = run_of(CHART, SessionOptions::default());
    app.shared.config.judge.offset_ms = 0;
    let down = Instant::now();
    state.handle_pad(&mut FrameCtx { shared: &mut app.shared, now: down, dt: 0.0 }, crate::gamepad::PadEvent::Control(ControlAction::OffsetDown));
    frame(&mut state, &mut app, down + FRAME);
    frame(&mut state, &mut app, down + Duration::from_secs(2));
    assert_eq!(app.shared.config.judge.offset_ms, -1);
}
