//! What the PLAY screen holds for the player from one input and one chart to the next: a green
//! number that is kept whatever the chart's tempo and the lane cover are, and a judge offset that
//! adjusts itself as inputs land.
//!
//! Both are read here the way a skin reads them, off the frame the screen lends the host
//! ([`PlayShown`]), so what is asserted is the number on show and not a figure worked out beside it.
//! The speed keys and the covers are driven through the screen's own controls; the held controls of
//! START and SELECT have their tests next door (`controls/tests.rs`).

use rbms_config::{
    DEFAULT_HISPEED, GREEN_NUMBER_FINE_STEP, GREEN_NUMBER_STEP, HISPEED_MAX, JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, duration_of_green, green_of_duration,
};
use rbms_play::{NullSink, SessionOptions};
use rbms_skin::property::generated::{NUMBER_DURATION, NUMBER_DURATION_GREEN, NUMBER_JUDGETIMING, NUMBER_LANECOVER1, NUMBER_LIFT1};

use super::*;
use crate::skin_host::ClusterState;
use crate::stage::KeyInput;
use crate::{App, Config, LaunchOptions};

/// The two tempos of the request this is built for: a chart at one, then a chart at the other.
const SLOW_BPM: u32 = 150;
const FAST_BPM: u32 = 200;

/// How much of the lane the cover and the lift take in these runs.
const COVER: f32 = 0.3;
const LIFT: f32 = 0.15;

/// How far apart two green numbers may read and still be the one number, for the rounding of the
/// travel time they are cut from.
const GREEN_ROUNDING: i32 = 1;

/// How far ahead of its note the steady player of these tests presses, in microseconds: one step of
/// the offset's own (`JudgeManager.java:701`: 15 ms to under 45).
const EARLY_BY_US: i64 = 40_000;

/// How long a key stays down, in microseconds.
const HELD_FOR_US: i64 = 20_000;

/// The first key lane.
const FIRST_LANE: usize = 0;

/// Far enough ahead that the wall clock never catches the song clock a test has set.
const CLOCK_HELD_BACK: Duration = Duration::from_secs(3_600);

/// A one-measure seven-key chart at `bpm`, with two notes in its first key.
fn chart(bpm: u32) -> String {
    format!("#PLAYER 1\n#TITLE t\n#BPM {bpm}\n#WAV01 a.wav\n#00111:0101\n")
}

pub(super) fn run_of(chart: &str, options: SessionOptions) -> PlayState {
    let src = rbms_parser::parse_with(chart.as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "t.bms"));
    PlayState::new(PlaySession::new(model, options), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

fn run_at(bpm: u32) -> PlayState {
    run_of(&chart(bpm), SessionOptions::default())
}

/// A run of the chart at [`SLOW_BPM`] set up the way the application sets one up for `app`: at the
/// judge offset the settings hold.
fn run_judged_for(app: &App) -> PlayState {
    run_of(&chart(SLOW_BPM), SessionOptions { judge_offset_us: app.shared.offset_us(), ..SessionOptions::default() })
}

/// An app with no window, no audio and no server, played by hand on `config`.
pub(super) fn app_on(tag: &str, mut config: Config) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-play-hold-tests-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&dir);
    config.play.autoplay = false;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.replay = None;
    app
}

/// The settings of a player who holds a green number under a cover and a lift.
fn holding() -> Config {
    let mut config = Config::default();
    config.play.keep_green_number = true;
    config.play.cover = COVER;
    config.play.enable_cover = true;
    config.play.lift = LIFT;
    config.play.enable_lift = true;
    config
}

/// Put the play screen up for `state`, the way the application does when a chart is entered.
pub(super) fn enter(state: &mut PlayState, app: &mut App) {
    state.on_enter(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
}

/// The frame the screen lends a skin right now.
fn shown(state: &mut PlayState, app: &App) -> PlayShown {
    state.ensure_feed(&app.shared);
    state.ensure_field(&app.shared);
    let (bpm, scroll) = state.tempo_at(state.song_us);
    let travel = state.field.as_ref().and_then(FieldChart::tempo).unwrap_or((bpm, scroll));
    let frame = DocumentFrame { bpm, travel, hispeed: app.shared.config.play.hispeed, background: None };
    let feed = state.feed.as_deref().expect("the screen has something to feed a skin from");
    PlayShown::of(&state.session, &state.live(&app.shared, feed, frame))
}

/// One number of that frame, as a skin's number object reads it.
fn number(state: &mut PlayState, app: &App, id: i32) -> i32 {
    let frame = shown(state, app);
    crate::skin_host::play::PlayState::of(&frame).integer(id).expect("the play cluster answers the id")
}

/// The green number on show (`IntegerPropertyFactory.duration_green`).
pub(super) fn green(state: &mut PlayState, app: &App) -> i32 {
    number(state, app, NUMBER_DURATION_GREEN)
}

pub(super) fn hispeed(app: &App) -> f64 {
    app.shared.config.play.hispeed
}

/// The request, as it was put: a green number set once on a chart at 150 under a cover and a lift
/// is the green number a chart at 200 starts at, and the speed is what changed.
#[test]
fn a_green_number_set_on_one_chart_is_what_a_chart_at_another_tempo_starts_at() {
    let mut app = app_on("across-charts", holding());
    let mut slow = run_at(SLOW_BPM);
    enter(&mut slow, &mut app);
    assert_eq!(green(&mut slow, &app), green_of_duration(app.shared.config.play.duration_ms), "the run starts at the number the settings hold");
    for _ in 0..3 {
        slow.in_play_control(&mut app.shared, ControlAction::HiSpeedDown);
    }
    let set = green(&mut slow, &app);
    assert_eq!(set, 300 + 3 * GREEN_NUMBER_STEP, "three presses of the slower key are three steps of the number");
    let slow_speed = hispeed(&app);
    assert_eq!((number(&mut slow, &app, NUMBER_LANECOVER1), number(&mut slow, &app, NUMBER_LIFT1)), (300, 150), "the white numbers are the cover and the lift");

    let mut fast = run_at(FAST_BPM);
    enter(&mut fast, &mut app);
    let carried = green(&mut fast, &app);
    assert!((carried - set).abs() <= GREEN_ROUNDING, "a chart at {FAST_BPM} started at {carried}, not at the {set} set at {SLOW_BPM}");
    let fast_speed = hispeed(&app);
    let wanted = slow_speed * f64::from(SLOW_BPM) / f64::from(FAST_BPM);
    assert!((fast_speed - wanted).abs() < 1e-9, "the speed went from {slow_speed} to {fast_speed}, and {wanted} is what holds the number");
    assert_eq!(number(&mut fast, &app, NUMBER_DURATION), app.shared.config.play.duration_ms, "the travel time on show is the one that is stored");
}

/// With the row off nothing of this happens: the speed the settings hold is the speed of every
/// chart, and the green number is whatever that comes to.
#[test]
fn with_the_row_off_every_chart_scrolls_at_the_speed_the_settings_hold() {
    let mut config = holding();
    config.play.keep_green_number = false;
    config.play.hispeed = 2.5;
    let mut app = app_on("row-off", config);
    let mut numbers = Vec::new();
    for bpm in [SLOW_BPM, FAST_BPM] {
        let mut state = run_at(bpm);
        enter(&mut state, &mut app);
        state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
        assert_eq!(hispeed(&app), 2.5, "a chart at {bpm} moved the speed");
        numbers.push(green(&mut state, &app));
    }
    assert_eq!(numbers, [green_of_duration(448), green_of_duration(336)], "240000 / bpm / 2.5 * 0.7, which is what the screen showed before the row existed");
    assert_eq!(app.shared.config.play.duration_ms, Config::default().play.duration_ms, "a row that is off stored a travel time");
}

/// Moving the lane cover while a chart plays leaves the number where it was and moves the speed,
/// whichever way the cover is moved and whether it is moved or switched.
#[test]
fn moving_or_switching_the_cover_in_a_run_keeps_the_number_and_moves_the_speed() {
    let mut app = app_on("cover", holding());
    let mut state = run_at(SLOW_BPM);
    enter(&mut state, &mut app);
    let (held, mut speed) = (green(&mut state, &app), hispeed(&app));

    for _ in 0..12 {
        state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    }
    assert!(app.shared.config.play.cover > COVER, "the bound key did not move the cover");
    assert!((green(&mut state, &app) - held).abs() <= GREEN_ROUNDING, "a bound key moved the number to {}", green(&mut state, &app));
    assert!(hispeed(&app) < speed, "more cover with the same number is a slower scroll");
    speed = hispeed(&app);

    state.move_cover(&mut app.shared, -0.2);
    assert!((green(&mut state, &app) - held).abs() <= GREEN_ROUNDING, "the wheel moved the number to {}", green(&mut state, &app));
    assert!(hispeed(&app) > speed, "less cover with the same number is a faster scroll");
    speed = hispeed(&app);

    app.shared.config.play.enable_cover = false;
    state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
    assert!((green(&mut state, &app) - held).abs() <= GREEN_ROUNDING, "switching the cover off moved the number to {}", green(&mut state, &app));
    assert!(hispeed(&app) > speed, "no cover at all is a faster scroll still");
    app.shared.config.play.enable_cover = true;
    state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
    assert!((hispeed(&app) - speed).abs() < 1e-9, "switching the cover back on did not put the speed back");
    assert_eq!(app.shared.config.play.duration_ms, duration_of_green(held), "the travel time that is stored never moved");
}

/// The lift does the same without a speed to move: the field scrolls a note over the lane the lift
/// leaves in the time it scrolls it over the whole lane, so the number stands whatever the lift is.
#[test]
fn moving_or_switching_the_lift_in_a_run_keeps_the_number() {
    let mut app = app_on("lift", holding());
    let mut state = run_at(SLOW_BPM);
    enter(&mut state, &mut app);
    let (held, speed) = (green(&mut state, &app), hispeed(&app));
    for _ in 0..20 {
        state.in_play_control(&mut app.shared, ControlAction::LiftUp);
    }
    assert!(app.shared.config.play.lift > LIFT, "the bound key did not move the lift");
    assert_eq!(green(&mut state, &app), held);
    app.shared.config.play.enable_lift = false;
    state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
    assert_eq!(green(&mut state, &app), held, "switching the lift off moved the number");
    assert!((hispeed(&app) - speed).abs() < 1e-9, "the lift moved a speed it has no part in");
    for _ in 0..5 {
        state.in_play_control(&mut app.shared, ControlAction::HiddenUp);
    }
    assert_eq!(green(&mut state, &app), held, "the hidden cover is no part of the number either");
}

/// The speed keys of such a run move the number, by its step and by the finer one with the modifier
/// down, and what they leave is what the settings hold for the next chart.
#[test]
fn the_speed_keys_move_the_held_number_and_the_settings_keep_it() {
    let mut app = app_on("speed-keys", holding());
    let mut state = run_at(SLOW_BPM);
    enter(&mut state, &mut app);
    let (start, speed) = (green(&mut state, &app), hispeed(&app));

    state.in_play_control(&mut app.shared, ControlAction::HiSpeedUp);
    assert_eq!(green(&mut state, &app), start - GREEN_NUMBER_STEP, "a faster scroll is a smaller number");
    assert!(hispeed(&app) > speed);
    state.in_play_control(&mut app.shared, ControlAction::HiSpeedDown);
    assert_eq!(green(&mut state, &app), start);
    assert!((hispeed(&app) - speed).abs() < 1e-9);

    app.shared.note_key(&KeyInput { code: KeyCode::ShiftRight, pressed: true, released: false, text: None });
    state.in_play_control(&mut app.shared, ControlAction::HiSpeedDown);
    assert_eq!(green(&mut state, &app), start + GREEN_NUMBER_FINE_STEP, "the modifier asks for the finer step");
    assert_eq!(green_of_duration(app.shared.config.play.duration_ms), start + GREEN_NUMBER_FINE_STEP, "the settings hold what the keys left");
}

/// How far one notch of these tests' wheel moves the cover, as a share of the field.
const WHEEL_NOTCH: f32 = 0.005;

/// How many presses of the slower speed key these tests move a held number by.
const SLOWER_PRESSES: i32 = 3;

/// A run that held its number and then stops holding it keeps the number it was left at: the next
/// move of the cover holds that one, not the number the run had when its cover first moved.
#[test]
fn a_run_that_stops_holding_its_number_pins_the_one_it_was_left_at() {
    let mut config = holding();
    config.play.fix_hispeed = FixHiSpeed::StartBpm;
    let mut app = app_on("stops-holding", config);
    let mut state = run_at(SLOW_BPM);
    enter(&mut state, &mut app);
    let start = green(&mut state, &app);

    state.move_cover(&mut app.shared, WHEEL_NOTCH);
    for _ in 0..SLOWER_PRESSES {
        state.in_play_control(&mut app.shared, ControlAction::HiSpeedDown);
    }
    let left_at = start + SLOWER_PRESSES * GREEN_NUMBER_STEP;
    assert_eq!(green(&mut state, &app), left_at);

    app.shared.config.play.keep_green_number = false;
    state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
    assert_eq!(green(&mut state, &app), left_at, "switching the row off moved the number");
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    let after = green(&mut state, &app);
    assert!((after - left_at).abs() <= GREEN_ROUNDING, "the cover put the number back to {after}: the run started at {start} and was left at {left_at}");
}

/// A number no speed in range can show stops the speed at the end of its range, and the screen
/// shows the number the speed really comes to, not the one that was asked for.
#[test]
fn a_number_out_of_reach_stops_the_speed_at_its_end_and_the_screen_shows_the_real_one() {
    let mut config = holding();
    config.play.duration_ms = duration_of_green(10);
    let mut app = app_on("out-of-reach", config);
    let mut state = run_at(SLOW_BPM);
    enter(&mut state, &mut app);
    assert_eq!(hispeed(&app), HISPEED_MAX);
    let on_show = green(&mut state, &app);
    assert_eq!(on_show, 33, "240000 / 150 / 20 * 0.7 is 56 ms, three fifths of which is 33");
    assert_eq!(green_of_duration(app.shared.config.play.duration_ms), 10, "what was asked for is still what is stored");
}

/// Under CONSTANT the field scrolls at one tempo of its own, so the same number comes to the same
/// speed on every chart and is the number on show on every one.
#[test]
fn under_constant_the_held_number_is_the_one_on_show_at_every_tempo() {
    let mut config = holding();
    config.play.constant_speed = true;
    let mut app = app_on("constant", config);
    let mut speeds = Vec::new();
    for bpm in [SLOW_BPM, FAST_BPM] {
        let mut state = run_at(bpm);
        enter(&mut state, &mut app);
        assert_eq!(green(&mut state, &app), 300, "a chart at {bpm}");
        speeds.push(hispeed(&app));
    }
    assert!((speeds[0] - speeds[1]).abs() < 1e-9, "a field that ignores the tempo scrolled at {speeds:?}");
}

/// The number is held at the tempo the HI-SPEED FIX row names, and at the one the most notes are
/// played at while that row names none.
#[test]
fn the_number_is_held_at_the_tempo_the_fix_row_names_and_at_the_main_one_without() {
    const VARIABLE_CHART: &str =
        "#PLAYER 1\n#TITLE t\n#BPM 120\n#WAV01 a.wav\n#BPM01 240\n#BPM02 90\n#00108:01\n#00111:0101\n#00208:02\n#00211:0101\n#00311:01010101\n";
    let rows = [(FixHiSpeed::StartBpm, 120.0), (FixHiSpeed::MaxBpm, 240.0), (FixHiSpeed::MinBpm, 90.0), (FixHiSpeed::MainBpm, 90.0), (FixHiSpeed::Off, 90.0)];
    for (fix, bpm) in rows {
        let mut config = holding();
        config.play.enable_cover = false;
        config.play.fix_hispeed = fix;
        let mut app = app_on("fix-row", config);
        let mut state = run_of(VARIABLE_CHART, SessionOptions::default());
        enter(&mut state, &mut app);
        let wanted = 240_000.0 / bpm / f64::from(app.shared.config.play.duration_ms);
        assert!((hispeed(&app) - wanted).abs() < 1e-9, "{fix:?}: the speed is {}, and {wanted} holds the number at {bpm}", hispeed(&app));
    }
}

/// A run that reproduces a replay starts at the watcher's own number like any other, and moves
/// nothing the settings hold by doing so.
#[test]
fn a_replay_is_watched_at_the_watchers_own_number() {
    let mut app = app_on("replay", holding());
    let replay = rbms_store::Replay {
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
    };
    app.shared.replay = Some(replay.clone());
    let mut state = run_of(&chart(FAST_BPM), SessionOptions { replay: Some(replay), ..SessionOptions::default() });
    enter(&mut state, &mut app);
    assert_eq!(green(&mut state, &app), 300);
    assert_ne!(hispeed(&app), DEFAULT_HISPEED);
    assert_eq!(app.shared.config.play.duration_ms, Config::default().play.duration_ms);
}

/// A course that locks the speed holds nothing: the run scrolls at what the course left the speed
/// at, and the speed keys do nothing on it, as they do nothing on any run of such a course
/// (`LaneRenderer.java:95-103`, `BMSPlayer.java:419-423`).
#[test]
fn a_course_that_locks_the_speed_holds_no_green_number() {
    use rbms_course::{Course, CourseConstraint};
    let mut app = app_on("locked-course", holding());
    let locked = Course { name: "locked".to_string(), constraints: vec![CourseConstraint::NoSpeed], ..Course::default() };
    app.shared.course_overrides = Some(crate::course_ui::CourseOverrides::of(&locked));
    app.shared.config.play.hispeed = 1.0;
    let mut state = run_at(FAST_BPM);
    enter(&mut state, &mut app);
    state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 });
    state.in_play_control(&mut app.shared, ControlAction::HiSpeedUp);
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    assert_eq!(hispeed(&app), 1.0, "a run of a course that locks the speed moved it");
    assert_eq!(app.shared.config.play.duration_ms, Config::default().play.duration_ms, "a locked speed key moved the number that is held");
    assert_eq!(app.shared.config.play.cover, COVER);
}

/// When this chart's notes fall.
pub(super) fn note_times(state: &PlayState) -> Vec<i64> {
    state.session.model().timelines.iter().filter(|line| line.notes.iter().flatten().next().is_some()).map(|line| line.time_us).collect()
}

/// Put the song clock at `at_us` and hold it there, so a key read off it is read at that moment
/// however long the machine takes over the lines in between.
pub(super) fn song_at(app: &mut App, at_us: i64) {
    app.shared.clock = Instant::now() + CLOCK_HELD_BACK;
    app.shared.song_us_last.set(at_us);
}

/// Press and let go of the first lane's key through the screen, the press landing at `at_us`.
fn strike(state: &mut PlayState, app: &mut App, at_us: i64) {
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let mut send = |app: &mut App, at_us: i64, pressed: bool| {
        song_at(app, at_us);
        let key = KeyInput { code: lane_key, pressed, released: !pressed, text: None };
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, key);
    };
    send(app, at_us, true);
    send(app, at_us + HELD_FOR_US, false);
}

/// Settings that let the judge offset adjust itself, starting from `offset_ms`.
fn adjusting(offset_ms: i32) -> Config {
    let mut config = Config::default();
    config.judge.auto_offset = true;
    config.judge.offset_ms = offset_ms;
    config
}

/// The range a run adjusts its offset inside is the range the JUDGE OFFSET row can hold. The play
/// crate states the two ends itself rather than depending on the configuration.
#[test]
fn a_run_adjusts_its_offset_inside_the_range_the_row_can_hold() {
    assert_eq!((rbms_play::AUTO_ADJUST_OFFSET_MIN_MS, rbms_play::AUTO_ADJUST_OFFSET_MAX_MS), (JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_MAX_MS));
}

/// An early input has the notes drawn later from then on: the settings hold the move at once, a
/// skin's judge timing number shows it, and the note field is drawn by it on the next frame.
#[test]
fn an_early_key_moves_the_offset_the_settings_hold_and_a_skin_shows() {
    let mut app = app_on("adjust-on", adjusting(3));
    let mut state = run_judged_for(&app);
    let notes = note_times(&state);
    assert_eq!(state.field_lead_us(&app.shared), 3_000);
    strike(&mut state, &mut app, notes[0] - EARLY_BY_US);
    assert_eq!(state.session.judge().total_judged(), 1, "the key did not reach the note");
    assert_eq!(app.shared.config.judge.offset_ms, 2, "40 ms early is the notes one millisecond later (JudgeManager.java:701)");
    assert_eq!(number(&mut state, &app, NUMBER_JUDGETIMING), 2);
    assert_eq!(state.field_lead_us(&app.shared), 2_000, "the field is drawn by the offset the input left");
    strike(&mut state, &mut app, notes[1] - EARLY_BY_US);
    assert_eq!(app.shared.config.judge.offset_ms, 1, "the second input moved the offset the first left");
}

/// A late input has them drawn earlier.
#[test]
fn a_late_key_moves_the_offset_the_other_way() {
    let mut app = app_on("adjust-late", adjusting(3));
    let mut state = run_judged_for(&app);
    let notes = note_times(&state);
    strike(&mut state, &mut app, notes[0] + EARLY_BY_US);
    assert_eq!(state.session.judge().total_judged(), 1, "the key did not reach the note");
    assert_eq!(app.shared.config.judge.offset_ms, 4);
}

/// With the row off the same keys move nothing.
#[test]
fn with_auto_adjust_off_the_offset_stays_where_the_player_set_it() {
    let mut config = adjusting(3);
    config.judge.auto_offset = false;
    let mut app = app_on("adjust-off", config);
    let mut state = run_judged_for(&app);
    let notes = note_times(&state);
    strike(&mut state, &mut app, notes[0] - EARLY_BY_US);
    assert_eq!(state.session.judge().total_judged(), 1);
    assert_eq!(app.shared.config.judge.offset_ms, 3);
}

/// A run the game plays, and one that reproduces a replay, take no key of the player's and so move
/// no offset, whatever the row says.
#[test]
fn a_run_that_plays_itself_or_a_replay_moves_no_offset() {
    let mut autoplayed = app_on("adjust-autoplay", adjusting(3));
    autoplayed.shared.config.play.autoplay = true;
    let mut state = run_of(&chart(SLOW_BPM), SessionOptions { autoplay: true, auto_calibration: true, ..SessionOptions::default() });
    let notes = note_times(&state);
    strike(&mut state, &mut autoplayed, notes[0] - EARLY_BY_US);
    state.session.tick(rbms_play::SessionClock::at(notes[1] + EARLY_BY_US), &mut NullSink);
    assert_eq!(state.session.judge().total_judged(), 2, "the chart played itself");
    assert_eq!((autoplayed.shared.config.judge.offset_ms, state.session.judge_offset_ms()), (3, 3));

    let mut watched = app_on("adjust-replay", adjusting(3));
    let events = vec![rbms_store::ReplayEvent { t: notes[0] - EARLY_BY_US, lane: FIRST_LANE, press: true, backward: false }];
    let replay = rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 3,
        scratch_auto: false,
        gauge: String::new(),
        judge: rbms_store::ReplayJudge::default(),
        events,
    };
    watched.shared.replay = Some(replay.clone());
    let options = SessionOptions { replay: Some(replay), judge_offset_us: 3_000, auto_calibration: true, ..SessionOptions::default() };
    let mut state = run_of(&chart(SLOW_BPM), options);
    strike(&mut state, &mut watched, notes[1] - EARLY_BY_US);
    state.session.tick(rbms_play::SessionClock::at(notes[0]), &mut NullSink);
    assert_eq!(state.session.judge().total_judged(), 1, "the replay's own input was judged");
    assert_eq!((watched.shared.config.judge.offset_ms, state.session.judge_offset_ms()), (3, 3));
}

/// The offset a run was abandoned at is the offset the settings file holds afterwards: the
/// reference keeps each move in the player's configuration the moment it is made, so a run that is
/// left early keeps what it had learnt.
#[test]
fn an_abandoned_run_leaves_its_offset_in_the_settings_file() {
    let mut app = app_on("adjust-abandoned", adjusting(0));
    let settings = app.shared.settings_path.clone();
    let mut state = run_judged_for(&app);
    let notes = note_times(&state);
    strike(&mut state, &mut app, notes[0] - EARLY_BY_US);
    assert_eq!(app.shared.config.judge.offset_ms, -1);
    let left = state.handle_key(
        &mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 },
        KeyInput { code: KeyCode::Escape, pressed: true, released: false, text: None },
    );
    assert!(!matches!(left, Transition::Stay), "Escape did not leave the run");
    let saved = rbms_config::load(&settings).expect("the settings file reads back").config;
    assert_eq!(saved.judge.offset_ms, -1);
    assert!(saved.judge.auto_offset);
}

/// A run that reaches its result, cleared or failed, writes the offset out too. The replay it saves
/// states no offset, because every input was judged where it landed, and comes out as the run
/// whatever the offset of whoever watches it is.
#[test]
fn a_run_that_reaches_its_result_saves_its_offset_and_a_replay_judged_where_it_was_played() {
    let mut app = app_on("adjust-result", adjusting(0));
    let settings = app.shared.settings_path.clone();
    let mut state = run_judged_for(&app);
    let notes = note_times(&state);
    strike(&mut state, &mut app, notes[0] - EARLY_BY_US);
    strike(&mut state, &mut app, notes[1] - EARLY_BY_US);
    assert_eq!(app.shared.config.judge.offset_ms, -2);
    let summary = state.session.summary();
    assert!(matches!(enter_result(&mut state, &mut app.shared), Transition::To(Stage::Result(_))), "a finished run moves to the result screen");
    assert_eq!(app.shared.config.judge.offset_ms, -2, "the result applied a second correction on top of the run's own");
    assert_eq!(rbms_config::load(&settings).expect("the settings file reads back").config.judge.offset_ms, -2);

    let saved = std::fs::read_dir(&app.shared.replay_dir).expect("the run saved a replay").filter_map(Result::ok).map(|entry| entry.path()).next();
    let replay = rbms_store::Replay::load(&saved.expect("one replay was saved")).expect("the replay reads back");
    assert_eq!(replay.offset_ms, 0, "the inputs were judged where they landed");
    assert!(replay.judge.offset_steps.is_empty(), "{:?}", replay.judge.offset_steps);

    for watched_at_us in [0, -2_000, 120_000] {
        let options = SessionOptions { replay: Some(replay.clone()), judge_offset_us: watched_at_us, ..SessionOptions::default() };
        let mut again = run_of(&chart(SLOW_BPM), options);
        again.session.tick(rbms_play::SessionClock::at(notes[1] + HELD_FOR_US), &mut NullSink);
        let reproduced = again.session.summary();
        assert_eq!((reproduced.counts, reproduced.fast, reproduced.early), (summary.counts, summary.fast, summary.early), "watched at {watched_at_us} us");
    }
}
