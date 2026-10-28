//! Where the judge offset puts the three instants of one note: when it sounds, when it is drawn on
//! the judge line, and when a press of it is on time.
//!
//! The reference adds its `judgetiming` to the time its note field is drawn at and to nothing else
//! (`LaneRenderer.java:300-301`; `JudgeManager` and the keysounds never read it). So at an offset
//! of +N ms:
//!
//! | instant | where it is |
//! |---|---|
//! | the note's sound, and every sound of the chart | at the note's own time |
//! | the middle of the window a press is a PGREAT in | at the note's own time |
//! | the note drawn on the judge line | N ms before it |
//!
//! Each is measured here rather than worked out: the sound off what an autoplayed run books, the
//! line by walking the clock a millisecond at a time until the field is drawn with the note on it,
//! and the window by pressing at every millisecond around the note and keeping the presses that
//! were a PGREAT. Before the offset was the reference's it was added to the instant a press was
//! judged at, and the same measurement read the other way: the line on the note's time and the
//! window N ms before it, which is a press on the beat judged N ms late.

use rbms_play::{SessionClock, SessionOptions, SoundRequest, SoundSink, SoundTime};

use super::hold_tests::{app_on, note_times, run_of, song_at};
use super::*;
use crate::stage::KeyInput;
use crate::{App, Config};

/// The tempo of the chart that is measured.
const BPM: u32 = 150;

/// The offsets the three instants are measured at, in milliseconds.
const OFFSETS_MS: [i32; 5] = [0, 30, -30, 100, -100];

/// How far either side of the note the clock is walked and the key is pressed, in microseconds:
/// past the largest offset by more than the widest window of a seven-key chart.
const WALKED_US: i64 = 260_000;

/// How finely, which is the unit the offset is counted in.
const WALK_STEP_US: i64 = 1_000;

/// How far either side of its middle a press is a PGREAT on a chart that states no rank, in
/// microseconds.
const PERFECT_WINDOW_US: i64 = 20_000;

/// The first key lane.
const FIRST_LANE: usize = 0;

/// The judgements of a run, in the order the engine counts them.
const PERFECT: usize = 0;

/// The PLAYBACK RATE of a run played at double speed, in percent, and how much more of the chart
/// goes by in a length of the player's time at it.
const DOUBLE_SPEED_PERCENT: i32 = 200;
const DOUBLE_SPEED_FACTOR: i64 = 2;

/// The speed of a practice slice played at half speed, in percent, and how much of the chart goes
/// by in a length of the player's time at it.
const HALF_SPEED_PERCENT: i32 = 50;
const HALF_SPEED_DIVISOR: i64 = 2;

/// The chart a practice slice is cut from, as its panel is told of it: how long it is in
/// milliseconds and its TOTAL, and where the slice begins.
const PRACTISED_CHART_MS: i32 = 60_000;
const PRACTISED_TOTAL: f64 = 300.0;
const PRACTICE_START_MS: i32 = 30_000;

/// A one-measure seven-key chart with two notes in its first key.
fn chart() -> String {
    format!("#PLAYER 1\n#TITLE t\n#BPM {BPM}\n#WAV01 a.wav\n#00111:0101\n")
}

/// Everything a run asked to be sounded.
#[derive(Default)]
struct Heard(Vec<SoundRequest>);

impl SoundSink for Heard {
    fn play(&mut self, request: SoundRequest) {
        self.0.push(request);
    }

    fn stop_all(&mut self) {}
}

fn offset_config(offset_ms: i32) -> Config {
    let mut config = Config::default();
    config.judge.offset_ms = offset_ms;
    config
}

/// A run set up the way the application sets one up for `app`.
fn run_for(app: &App) -> PlayState {
    run_of(&chart(), SessionOptions { judge_offset_us: app.shared.offset_us(), ..SessionOptions::default() })
}

/// Put the first lane's key down through the screen with the song clock at `at_us`.
fn press(state: &mut PlayState, app: &mut App, at_us: i64) {
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    song_at(app, at_us);
    let key = KeyInput { code: lane_key, pressed: true, released: false, text: None };
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, key);
}

/// When the first note of the chart is sounded by a run that plays itself, on the clock sounds are
/// booked on.
fn sounded_at(app: &App) -> SoundTime {
    let mut run = run_of(&chart(), SessionOptions { autoplay: true, judge_offset_us: app.shared.offset_us(), ..SessionOptions::default() });
    let note_us = note_times(&run)[0];
    let mut heard = Heard::default();
    run.session.tick(SessionClock { audible_us: 0, scheduled_us: note_us }, &mut heard);
    heard.0.first().expect("the note has a sound").at
}

/// Every millisecond around `note_us`, earliest first.
fn walk(note_us: i64) -> impl Iterator<Item = i64> {
    (-WALKED_US / WALK_STEP_US..=WALKED_US / WALK_STEP_US).map(move |step| note_us + step * WALK_STEP_US)
}

/// The first moment of the song a skin's note field is drawn with the note at `note_us` on its
/// judge line: the play timer is put where a frame at each moment would have it, and the time the
/// field is handed is read back.
fn on_the_line_of_a_skin_at(state: &PlayState, app: &mut App, note_us: i64) -> i64 {
    app.shared.stop_scene_clock();
    let now_us = app.shared.skin_now_us();
    walk(note_us)
        .find(|song_us| {
            app.shared.skin_timers.set_on(timer_id::PLAY, now_us - song_us);
            state.drawn_chart_us(&app.shared) >= note_us
        })
        .expect("the note reaches the line inside the walk")
}

/// The same for the built-in field, which is drawn off the song position of the frame.
fn on_the_line_of_the_built_in_field_at(state: &mut PlayState, app: &App, note_us: i64) -> i64 {
    walk(note_us)
        .find(|song_us| {
            state.song_us = *song_us;
            state.built_in_field_us(&app.shared) >= note_us
        })
        .expect("the note reaches the line inside the walk")
}

/// The first and the last moment a press of the note at `note_us` is a PGREAT, a fresh run for
/// every press.
fn perfect_between(app: &mut App, note_us: i64) -> (i64, i64) {
    let perfect: Vec<i64> = walk(note_us)
        .filter(|pressed_us| {
            let mut state = run_for(app);
            press(&mut state, app, *pressed_us);
            state.session.judge().counts[PERFECT] == 1
        })
        .collect();
    (*perfect.first().expect("some press is a PGREAT"), *perfect.last().expect("some press is a PGREAT"))
}

/// The table at the head of this file, measured at five offsets: the sound and the window stay on
/// the note, and the note is on the line the offset ahead of it.
#[test]
fn the_offset_moves_the_note_on_the_line_and_neither_the_sound_nor_the_window() {
    for offset_ms in OFFSETS_MS {
        let mut app = app_on(&format!("timing-{offset_ms}"), offset_config(offset_ms));
        let mut state = run_for(&app);
        let note_us = note_times(&state)[0];
        let ahead_us = i64::from(offset_ms) * MICROS_PER_MILLI;

        assert_eq!(sounded_at(&app), SoundTime::Scheduled(note_us), "the sound of the note at {offset_ms:+} ms");
        assert_eq!(on_the_line_of_a_skin_at(&state, &mut app, note_us), note_us - ahead_us, "a skin's field at {offset_ms:+} ms");
        assert_eq!(on_the_line_of_the_built_in_field_at(&mut state, &app, note_us), note_us - ahead_us, "the built-in field at {offset_ms:+} ms");
        let window = perfect_between(&mut app, note_us);
        assert_eq!(window, (note_us - PERFECT_WINDOW_US, note_us + PERFECT_WINDOW_US), "the PGREAT window at {offset_ms:+} ms");
    }
}

/// A press on the beat is on time at every offset, and its keysound is asked for at once: nothing
/// of what the player hears is moved.
#[test]
fn a_press_on_the_beat_is_on_time_and_sounds_at_once_at_every_offset() {
    for offset_ms in OFFSETS_MS {
        let mut run = run_of(&chart(), SessionOptions { judge_offset_us: i64::from(offset_ms) * MICROS_PER_MILLI, ..SessionOptions::default() });
        let note_us = note_times(&run)[0];
        let mut heard = Heard::default();
        let hit = run.session.press(FIRST_LANE, note_us, &mut heard).expect("the note is hit");
        assert_eq!(hit.delta_us, 0, "a press on the beat at {offset_ms:+} ms");
        assert_eq!(heard.0.iter().map(|request| request.at).collect::<Vec<_>>(), [SoundTime::Immediate], "its keysound at {offset_ms:+} ms");
    }
}

/// The field is drawn the offset ahead before the chart starts as well, as the reference draws it
/// (`LaneRenderer.java:300-301` adds its timing whichever timer is on, and with neither on).
#[test]
fn the_field_is_ahead_before_the_chart_starts_too() {
    let app = app_on("timing-before", offset_config(100));
    let state = run_for(&app);
    assert!(!app.shared.skin_timers.is_on(timer_id::PLAY));
    assert_eq!(state.drawn_chart_us(&app.shared), 100 * MICROS_PER_MILLI);
}

/// The offset is a length of the player's time. A practice slice at half speed goes through half
/// as much of the chart in it, so the note is still on the line that long before its sound.
#[test]
fn a_practice_slice_at_another_speed_is_ahead_by_the_same_length_of_the_players_time() {
    let app = app_on("timing-practice", offset_config(100));
    let mut state = run_for(&app);
    let whole = state.field_lead_us(&app.shared);
    assert_eq!(whole, 100 * MICROS_PER_MILLI);
    let mut panel = crate::practice::PracticePanel::new("md5".to_string(), Mode::BEAT_7K, PRACTISED_CHART_MS, None, PRACTISED_TOTAL);
    panel.property.start_ms = PRACTICE_START_MS;
    panel.property.freq = HALF_SPEED_PERCENT;
    state.set_practice(panel.start());
    assert_eq!(state.field_lead_us(&app.shared), whole / HALF_SPEED_DIVISOR);
    assert_eq!(state.drawn_chart_us(&app.shared), i64::from(PRACTICE_START_MS) * MICROS_PER_MILLI + whole / HALF_SPEED_DIVISOR, "from where the slice begins");
}

/// A run at another PLAYBACK RATE is judged in the player's own time (the reference's practice
/// FREQUENCY, `BMSPlayer.java:554-568`): at double speed the note comes at half its time of the song
/// clock, and the window a press of it is a PGREAT in is as long as it is at the chart's own speed,
/// not half as long. The judge offset is a length of the player's time too.
#[test]
fn a_run_at_another_playback_rate_is_judged_in_the_players_own_time() {
    let mut app = app_on("timing-rate", offset_config(100));
    let note_us = note_times(&run_for(&app))[0];
    let heard_us = note_us / DOUBLE_SPEED_FACTOR;
    let perfect: Vec<i64> = walk(heard_us)
        .filter(|pressed_us| {
            let mut state = run_for(&app);
            state.set_playback_rate(DOUBLE_SPEED_PERCENT);
            press(&mut state, &mut app, *pressed_us);
            state.session.judge().counts[PERFECT] == 1
        })
        .collect();
    let window = (*perfect.first().expect("some press is a PGREAT"), *perfect.last().expect("some press is a PGREAT"));
    assert_eq!(window, (heard_us - PERFECT_WINDOW_US, heard_us + PERFECT_WINDOW_US), "the PGREAT window of the player's own time");

    let mut state = run_for(&app);
    let whole = state.field_lead_us(&app.shared);
    state.set_playback_rate(DOUBLE_SPEED_PERCENT);
    assert_eq!(state.session.judge().playback_rate(), DOUBLE_SPEED_PERCENT, "the judge is told the rate");
    assert_eq!(state.field_lead_us(&app.shared), whole * DOUBLE_SPEED_FACTOR, "the offset is twice as much of the chart");
    assert_eq!(state.changed_playback_percent(), Some(DOUBLE_SPEED_PERCENT));
    assert_eq!(state.play_speed_percent(), DOUBLE_SPEED_PERCENT, "a skin's rhythm timer runs at the tempo the player hears");
}

/// A practice slice judges and draws at its own FREQUENCY whatever PLAYBACK RATE the run was set to.
#[test]
fn a_practice_slice_plays_at_its_own_frequency_over_the_playback_rate() {
    let app = app_on("timing-rate-practice", offset_config(100));
    let mut state = run_for(&app);
    let whole = state.field_lead_us(&app.shared);
    state.set_playback_rate(DOUBLE_SPEED_PERCENT);
    let mut panel = crate::practice::PracticePanel::new("md5".to_string(), Mode::BEAT_7K, PRACTISED_CHART_MS, None, PRACTISED_TOTAL);
    panel.property.freq = HALF_SPEED_PERCENT;
    state.set_practice(panel.start());
    assert_eq!(state.field_lead_us(&app.shared), whole / HALF_SPEED_DIVISOR);
    assert_eq!(state.session.judge().playback_rate(), HALF_SPEED_PERCENT);
    assert_eq!(state.play_speed_percent(), HALF_SPEED_PERCENT);
    assert_eq!(state.changed_playback_percent(), None, "a slice is no run at another PLAYBACK RATE");
}
