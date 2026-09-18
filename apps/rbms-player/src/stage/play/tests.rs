//! What the PLAY screen does with a run: the escape modes, the in-play controls, the timing split
//! it keeps for the HUD, and the summary it hands to the result screen.
//!
//! What the screen *paints* is pinned next door in `render_tests_play.rs`; what a keystroke does to
//! the run in progress is here.
#![allow(clippy::wildcard_imports)]

use super::*;
use crate::app_result::reported_gauge;
use crate::stage::KeyInput;
use crate::{App, Config, LaunchOptions};
use rbms_play::SessionOptions;

/// A one-measure 7-key chart, enough for a session with real lanes.
const CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n";

/// How far before a note a test press lands, well inside the widest window that still counts.
const EARLY_BY_US: i64 = 4_000;

/// How far after a note a test press lands.
const LATE_BY_US: i64 = 4_000;

/// One frame of the clock a replay is fed against.
const REPLAY_STEP_US: i64 = 16_000;

/// How far past the last note a replay is run, so every recorded input is reached.
const REPLAY_TAIL_US: i64 = 1_000_000;

/// An app with no window, no audio and no server, set to interactive play so the lane path is
/// live. `App::new` never opens an output stream, so `shared.audio` is the "no device" case.
fn app() -> App {
    let dir = std::env::temp_dir().join(format!("rbms-play-stage-tests-{}", std::process::id()));
    let mut config = Config::default();
    config.play.autoplay = false;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.replay = None;
    app
}

fn play_state() -> PlayState {
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "t.bms");
    let model = rbms_chart::to_model(&src, mode);
    PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// The same chart set up to reproduce `events` rather than to take input, which is what a replay
/// and an autoplay run both look like to this screen.
fn replay_state(events: Vec<rbms_store::ReplayEvent>) -> PlayState {
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "t.bms");
    let model = rbms_chart::to_model(&src, mode);
    let replay = rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: mode.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: rbms_store::ReplayJudge::default(),
        events,
    };
    let options = SessionOptions { replay: Some(replay), ..SessionOptions::default() };
    PlayState::new(PlaySession::new(model, options), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

fn key(code: KeyCode, pressed: bool) -> KeyInput<'static> {
    KeyInput { code, pressed, released: !pressed, text: None }
}

/// Send one key to a running chart at `now` and report where it left the app.
fn press_at(state: &mut PlayState, app: &mut App, code: KeyCode, pressed: bool, now: Instant) -> Transition {
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key(code, pressed))
}

/// Whether a transition leaves the chart, which is what abandoning a run looks like.
fn leaves_the_chart(transition: &Transition) -> bool {
    !matches!(transition, Transition::Stay)
}

/// What Escape has always done, and what a fresh install still holds.
#[test]
fn escape_abandons_a_run_on_the_first_press_by_default() {
    let mut app = app();
    assert_eq!(app.shared.config.play.play_escape, PlayEscape::Immediate, "the shipped setting is the one the key has always had");
    let mut state = play_state();
    let now = Instant::now();
    assert!(leaves_the_chart(&press_at(&mut state, &mut app, KeyCode::Escape, true, now)));
}

/// Every other screen acts on the press. A release carried in from the screen before — the Escape
/// that left it — must not take the run away the moment it starts.
#[test]
fn escape_letting_go_never_abandons_a_run_under_any_setting() {
    for mode in [PlayEscape::Immediate, PlayEscape::Hold, PlayEscape::Double] {
        let mut app = app();
        app.shared.config.play.play_escape = mode;
        let mut state = play_state();
        let now = Instant::now();
        assert!(!leaves_the_chart(&press_at(&mut state, &mut app, KeyCode::Escape, false, now)), "{mode:?} abandoned the run on a release");
    }
}

/// Holding is the guard for a player who keeps hitting the key by accident: a press that is let
/// go of again leaves the run alone.
#[test]
fn escape_held_briefly_leaves_the_run_alone() {
    let mut app = app();
    app.shared.config.play.play_escape = PlayEscape::Hold;
    let mut state = play_state();
    let now = Instant::now();
    assert!(!leaves_the_chart(&press_at(&mut state, &mut app, KeyCode::Escape, true, now)), "the press alone must not end the run");
    assert!(!state.escape_hold_elapsed(now + ESC_HOLD / 2), "and neither does half the hold");
    press_at(&mut state, &mut app, KeyCode::Escape, false, now + ESC_HOLD / 2);
    assert!(!state.escape_hold_elapsed(now + ESC_HOLD * 2), "letting go cancels the hold outright");
}

#[test]
fn escape_held_long_enough_abandons_the_run() {
    let mut app = app();
    app.shared.config.play.play_escape = PlayEscape::Hold;
    let mut state = play_state();
    let now = Instant::now();
    press_at(&mut state, &mut app, KeyCode::Escape, true, now);
    assert!(state.escape_hold_elapsed(now + ESC_HOLD), "the hold is up");
    let moved = state.update(&mut FrameCtx { shared: &mut app.shared, now: now + ESC_HOLD, dt: 0.0 });
    assert!(leaves_the_chart(&moved), "the frame the hold completes on leaves the chart");
}

#[test]
fn escape_pressed_twice_in_time_abandons_the_run_and_once_does_not() {
    let mut app = app();
    app.shared.config.play.play_escape = PlayEscape::Double;
    let mut state = play_state();
    let now = Instant::now();
    assert!(!leaves_the_chart(&press_at(&mut state, &mut app, KeyCode::Escape, true, now)), "one press arms the pair");
    let late = now + ESC_DOUBLE * 2;
    assert!(!leaves_the_chart(&press_at(&mut state, &mut app, KeyCode::Escape, true, late)), "a press too late arms a new pair rather than leaving");
    assert!(leaves_the_chart(&press_at(&mut state, &mut app, KeyCode::Escape, true, late + ESC_DOUBLE / 2)), "the second half of a pair leaves");
}

/// A run with nothing left to hit is finished, not abandoned, so it reaches the result screen
/// on one press whatever guard the setting asks for.
#[test]
fn a_finished_run_reaches_the_result_screen_under_every_escape_setting() {
    for mode in [PlayEscape::Immediate, PlayEscape::Hold, PlayEscape::Double] {
        let mut app = app();
        app.shared.config.play.play_escape = mode;
        let mut state = play_state();
        state.session.tick(SessionClock::at(state.session.last_time_us() + 1_000_000), &mut NullSink);
        assert!(state.session.all_notes_resolved(), "the fixture run is over");
        let moved = press_at(&mut state, &mut app, KeyCode::Escape, true, Instant::now());
        assert!(matches!(moved, Transition::To(Stage::Result(_))), "{mode:?} did not reach the result screen");
    }
}

#[test]
fn a_practice_slice_uses_absolute_chart_time_for_clock_schedule_and_end() {
    let mut panel = crate::practice::PracticePanel::new("md5".to_string(), Mode::BEAT_7K, 60_000, None, 300.0);
    panel.property.start_ms = 30_000;
    panel.property.end_ms = 45_000;
    panel.property.freq = 200;
    let mut state = play_state();
    state.set_practice(panel.start());
    let clock = state.practice_clock();

    assert_eq!(clock.chart_time_us(0), 30_000_000);
    assert_eq!(clock.chart_time_us(10_000_000), 50_000_000);
    let scheduled_engine_us = schedule_position_us(10_000_000, 2_000_000, 3_000_000, false);
    assert_eq!(clock.chart_time_us(scheduled_engine_us), 60_000_000);
    assert!(!state.run_is_over(44_999_999));
    assert!(state.run_is_over(45_000_000));
}

#[test]
fn a_press_is_recorded_for_the_replay_even_with_no_output_device() {
    let mut app = app();
    assert!(app.shared.audio.is_none(), "the fixture is the audio-unavailable case");
    let lane_key = app.shared.active_keys.first().map(|(code, _)| *code).expect("the default key config binds lane 0");

    let mut state = play_state();
    let now = std::time::Instant::now();
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key(lane_key, true));
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key(lane_key, false));

    let events = state.session.recorded_events();
    assert_eq!(events.len(), 2, "a run with no device still records what was played: {events:?}");
    assert!(events[0].press, "the press is recorded first");
    assert!(!events[1].press, "and the release after it — never a release on its own");
}

/// A run whose gauge is empty under GAUGE AUTO SHIFT = NONE is over: the reference moves to
/// `STATE_FAILED` and stops judging (`BMSPlayer.java:653-661, 694`), so the frame loop has to
/// leave the play screen rather than keep tallying a dead run to the end of the song.
#[test]
fn an_emptied_gauge_under_no_auto_shift_leaves_the_play_screen() {
    let mut app = app();
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "t.bms"));
    let last_us = model.timelines.last().map(|t| t.time_us).unwrap_or(0);
    let session = PlaySession::new(model, SessionOptions { gauge: rbms_judge::GaugeKind::Hazard, ..SessionOptions::default() });
    let mut state = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());

    let sweep_us = last_us + 1_000_000;
    state.session.tick(SessionClock::at(sweep_us), &mut NullSink);
    assert!(state.session.is_failed(), "a HAZARD gauge is empty after the first missed note");
    assert!(!state.session.is_finished(sweep_us), "and the song itself is not over yet, so only the failure can end the run");
    assert!(state.run_is_over(sweep_us));

    let now = std::time::Instant::now();
    let moved = state.update(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert!(matches!(moved, Transition::To(Stage::Result(_))), "the failed run has to reach the result screen");
}

/// A run the gauge auto-shift moved is recorded under the gauge that decided its clear, not
/// under the one the settings still hold, or the record would pair a NORMAL gauge with a HARD
/// lamp.
/// Scrubbing a replay is the player driving the clock, so a gauge that empties along the way
/// must not throw the screen to the result — the same reason `is_finished` stands down.
#[test]
fn a_replay_being_scrubbed_does_not_end_on_a_failed_gauge() {
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "t.bms"));
    let last_us = model.timelines.last().map(|t| t.time_us).unwrap_or(0);
    let replay = rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: String::new(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    };
    let options = SessionOptions { gauge: rbms_judge::GaugeKind::Hazard, analysis: true, replay: Some(replay), ..SessionOptions::default() };
    let mut state = PlayState::new(PlaySession::new(model, options), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());

    let sweep_us = last_us + 1_000_000;
    state.session.tick(SessionClock::at(sweep_us), &mut NullSink);
    assert!(state.session.is_failed(), "the gauge really did empty");
    assert!(!state.run_is_over(sweep_us), "an analysis run is scrubbed, not ended");
}

#[test]
fn a_shifted_run_is_recorded_under_the_gauge_it_finished_on() {
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "t.bms"));
    let mut session = PlaySession::new(model, SessionOptions { gauge: rbms_judge::GaugeKind::Normal, ..SessionOptions::default() });
    session.set_judge_setup(rbms_play::JudgeSetup { gauge_auto_shift: rbms_judge::gauge::GaugeAutoShift::BestClear, ..Default::default() });
    session.tick(SessionClock::at(0), &mut NullSink);

    let summary = session.summary();
    assert!(summary.gauge_shifted, "BEST CLEAR re-picks every frame and climbs off NORMAL at once");
    assert_ne!(reported_gauge(&summary, rbms_judge::GaugeKind::Normal), rbms_judge::GaugeKind::Normal, "the record follows the shift");
    assert_eq!(reported_gauge(&summary, rbms_judge::GaugeKind::Normal), summary.finished_gauge.expect("a selectable gauge decided the clear"));
}

/// A run nothing shifted still reports the gauge it was played on.
#[test]
fn an_unshifted_run_is_recorded_under_the_gauge_that_was_chosen() {
    let state = play_state();
    let summary = state.session.summary();
    assert!(!summary.gauge_shifted);
    assert_eq!(reported_gauge(&summary, rbms_judge::GaugeKind::Normal), rbms_judge::GaugeKind::Normal);
}

#[test]
fn a_submission_reports_the_candidate_policy_the_run_actually_used() {
    let state = play_state();
    let reported = state.session.judge().algorithm();
    assert_eq!(reported.name(), rbms_judge::JudgeAlgorithm::default().name(), "a fresh run uses the engine default");
    assert_eq!(reported.name(), "Combo", "and the default is the reference's own (JudgeAlgorithm.java:42 lists Combo first)");
}

/// How far two travel times may sit apart and still count as the same one — see the same
/// constant next to the arithmetic itself.
const GREEN_TOLERANCE_MS: f64 = 1e-3;

/// A chart whose tempo changes, so the four tempos are not all the same number.
const VARIABLE_CHART: &str =
    "#PLAYER 1\n#TITLE t\n#BPM 120\n#WAV01 a.wav\n#BPM01 240\n#BPM02 90\n#00108:01\n#00111:0101\n#00208:02\n#00211:0101\n#00311:01010101\n";

fn variable_state() -> PlayState {
    let src = rbms_parser::parse_with(VARIABLE_CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "t.bms");
    let model = rbms_chart::to_model(&src, mode);
    PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// The four tempos are what the SPEED FIX row picks between, so each has to be read off the
/// chart rather than guessed (`LaneRenderer.java:127-144`).
#[test]
fn the_four_tempos_of_a_chart_are_read_off_it() {
    let state = variable_state();
    assert_eq!(state.bpm.start, 120.0, "the chart opens at its header tempo");
    assert_eq!(state.bpm.min, 90.0);
    assert_eq!(state.bpm.max, 240.0);
    assert_eq!(state.bpm.main, 90.0, "the last section carries the most notes");
}

/// A chart that never changes tempo has the same answer four ways, and no pinning at all when
/// the row is off.
#[test]
fn a_single_tempo_chart_pins_to_that_tempo_whichever_way_the_row_points() {
    let state = play_state();
    assert_eq!(state.bpm.target(FixHiSpeed::Off), None, "OFF pins to nothing at all");
    for fix in [FixHiSpeed::StartBpm, FixHiSpeed::MinBpm, FixHiSpeed::MaxBpm, FixHiSpeed::MainBpm] {
        assert_eq!(state.bpm.target(fix), Some(120.0), "{fix:?}");
    }
}

#[test]
fn each_speed_fix_choice_names_the_tempo_it_says_it_does() {
    let state = variable_state();
    assert_eq!(state.bpm.target(FixHiSpeed::StartBpm), Some(120.0));
    assert_eq!(state.bpm.target(FixHiSpeed::MinBpm), Some(90.0));
    assert_eq!(state.bpm.target(FixHiSpeed::MaxBpm), Some(240.0));
    assert_eq!(state.bpm.target(FixHiSpeed::MainBpm), Some(90.0));
}

/// The point of pinning a green number: raising the cover mid-run leaves the number the player
/// reads where it was, by moving the scroll speed instead.
#[test]
fn raising_the_cover_on_a_pinned_run_holds_the_travel_time() {
    let mut app = app();
    app.shared.config.play.fix_hispeed = FixHiSpeed::MainBpm;
    app.shared.config.play.enable_cover = true;
    app.shared.config.play.cover = 0.1;
    app.shared.config.play.hispeed = 3.0;
    let mut state = play_state();
    let before = green_for_hispeed(state.bpm.main, app.shared.config.play.hispeed, app.shared.effective_cover());
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    let after = green_for_hispeed(state.bpm.main, app.shared.config.play.hispeed, app.shared.effective_cover());
    assert!((after - before).abs() < GREEN_TOLERANCE_MS, "the travel time moved from {before} to {after}");
    assert!(app.shared.config.play.hispeed < 3.0, "the scroll speed is what moved instead");
}

/// A run with the row off leaves the speed alone when the cover moves, which is what the screen
/// did before the row existed.
#[test]
fn raising_the_cover_on_an_unpinned_run_leaves_the_speed_alone() {
    let mut app = app();
    app.shared.config.play.fix_hispeed = FixHiSpeed::Off;
    app.shared.config.play.hispeed = 3.0;
    let mut state = play_state();
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    assert_eq!(app.shared.config.play.hispeed, 3.0);
}

/// Choosing a new speed by hand is the player choosing a new travel time, so the next cover
/// change has to hold the new one rather than the one the run started with.
#[test]
fn changing_the_speed_by_hand_re_reads_the_travel_time_that_is_then_held() {
    let mut app = app();
    app.shared.config.play.fix_hispeed = FixHiSpeed::MainBpm;
    app.shared.config.play.enable_cover = true;
    app.shared.config.play.cover = 0.1;
    app.shared.config.play.hispeed = 3.0;
    let mut state = play_state();
    state.in_play_control(&mut app.shared, ControlAction::HiSpeedUp);
    let chosen = green_for_hispeed(state.bpm.main, app.shared.config.play.hispeed, app.shared.effective_cover());
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    let held = green_for_hispeed(state.bpm.main, app.shared.config.play.hispeed, app.shared.effective_cover());
    assert!((held - chosen).abs() < GREEN_TOLERANCE_MS, "the run held {held} rather than the {chosen} the player chose");
}

/// The modifier must never take a key the chart needs: on the shipped seven-key layout the left
/// shift is the turntable, so only the free one switches the step.
#[test]
fn a_shift_key_bound_to_a_lane_is_not_the_fine_modifier() {
    let app = app();
    let scratch = app.shared.lane_input_for(KeyCode::ShiftLeft);
    assert!(scratch.is_some(), "the fixture layout binds the left shift to a lane");
    assert!(!PlayState::is_fine_modifier(&app.shared, KeyCode::ShiftLeft), "a key that plays a lane keeps that job");
    assert!(PlayState::is_fine_modifier(&app.shared, KeyCode::ShiftRight), "the free shift is the modifier");
    assert!(!PlayState::is_fine_modifier(&app.shared, KeyCode::KeyQ), "and nothing else is");
}

/// Holding the modifier switches the lane shades to their finer step for as long as it is down.
#[test]
fn holding_the_modifier_switches_the_shades_to_the_finer_step() {
    let mut app = app();
    let mut state = play_state();
    let now = Instant::now();
    press_at(&mut state, &mut app, KeyCode::ShiftRight, true, now);
    assert!(state.fine_held);
    app.shared.config.play.cover = 0.5;
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    let fine = app.shared.config.play.cover - 0.5;
    press_at(&mut state, &mut app, KeyCode::ShiftRight, false, now);
    assert!(!state.fine_held);
    app.shared.config.play.cover = 0.5;
    state.in_play_control(&mut app.shared, ControlAction::CoverUp);
    let coarse = app.shared.config.play.cover - 0.5;
    assert!(fine < coarse, "the modifier did not reach the step: {fine} against {coarse}");
}

/// When this chart's notes fall, read off the model so a change to the fixture cannot leave a test
/// pressing at nothing.
fn note_times(state: &PlayState) -> Vec<i64> {
    state.session.model().timelines.iter().filter(|tl| tl.notes.iter().flatten().next().is_some()).map(|tl| tl.time_us).collect()
}

/// Play one lane at `at_us` through the session, which is the path a key on this screen takes.
fn hit(state: &mut PlayState, lane: usize, at_us: i64) {
    state.session.press(lane, at_us, &mut rbms_play::NullSink);
    state.session.release(lane, at_us + 1);
}

/// A note hit early is counted in one column and one late in the other, and the two are kept apart
/// for the keys and for the turntable — a turntable is thrown rather than pressed and drifts in its
/// own direction.
#[test]
fn early_and_late_hits_are_counted_apart_for_the_keys_and_the_turntable() {
    let app = app();
    let mut state = play_state();
    let notes = note_times(&state);
    let scratch = (0..app.shared.mode.key).find(|&lane| app.shared.mode.is_scratch(lane)).expect("the fixture mode has a turntable");
    hit(&mut state, 0, notes[0] - EARLY_BY_US);
    hit(&mut state, 0, notes[1] + LATE_BY_US);
    hit(&mut state, scratch, notes[0]);
    assert_eq!(state.fast()[KEY_LANE_KIND], 1, "the early hit is in the key column");
    assert_eq!(state.slow()[KEY_LANE_KIND], 1, "and the late one beside it");
    assert_eq!(state.fast()[SCRATCH_LANE_KIND] + state.slow()[SCRATCH_LANE_KIND], 0, "the turntable had no note to hit on this chart");
    assert_eq!(
        state.fast().iter().chain(state.slow().iter()).sum::<u32>(),
        state.session.judge().fast + state.session.judge().slow,
        "the columns add up to what the judge engine counted"
    );
}

/// A note nobody hit is swept as a miss, which has no input to be early or late against: the two
/// columns still add up to what the judge engine counted.
#[test]
fn a_judgement_with_no_timing_is_counted_in_neither_column() {
    let mut state = play_state();
    let notes = note_times(&state);
    hit(&mut state, 0, notes[0] - EARLY_BY_US);
    state.session.tick(SessionClock { audible_us: notes[1] + REPLAY_TAIL_US, scheduled_us: notes[1] + REPLAY_TAIL_US }, &mut rbms_play::NullSink);
    let judge = state.session.judge();
    assert!(judge.counts[3..].iter().sum::<u32>() > 0, "the note left alone was not swept");
    assert_eq!(state.fast().iter().chain(state.slow().iter()).sum::<u32>(), judge.fast + judge.slow, "an untimed judgement reached a column");
    assert_eq!(state.fast()[KEY_LANE_KIND] + state.slow()[KEY_LANE_KIND], 1, "only the hit that had timing is counted");
}

/// The split is kept by the session, so a run whose inputs the player is not making — a replay
/// being played back, or an auto-played lane — reports one too. Reading it off this screen's own
/// key handler left every such run at zero.
#[test]
fn a_replayed_run_reports_its_split_as_well_as_an_interactive_one() {
    let mut played = play_state();
    let notes = note_times(&played);
    hit(&mut played, 0, notes[0] - EARLY_BY_US);
    hit(&mut played, 0, notes[1] + LATE_BY_US);
    let expected = (played.fast(), played.slow());
    assert!(expected.0.iter().chain(expected.1.iter()).sum::<u32>() > 0, "the fixture presses did not reach a timed judgement");

    let mut watching = replay_state(played.session.recorded_events().to_vec());
    let last = notes.last().copied().unwrap_or_default() + REPLAY_TAIL_US;
    let mut at = 0;
    while at <= last {
        watching.session.tick(SessionClock { audible_us: at, scheduled_us: at }, &mut rbms_play::NullSink);
        at += REPLAY_STEP_US;
    }
    let judge = watching.session.judge();
    let counted: u32 = watching.fast().iter().chain(watching.slow().iter()).sum();
    assert!(counted > 0, "a replayed run reported no early or late hits at all");
    assert_eq!(counted, judge.fast + judge.slow, "a replayed run reported a different split from the judgements it made");
    assert_eq!((watching.fast(), watching.slow()), expected, "the replay reported a different split from the run it reproduces");
}

/// The result screen reports the run's own split rather than dropping the turntable column, so
/// what the HUD showed while playing is what the summary reports when the run ends.
#[test]
fn the_split_counters_reach_the_result_screen() {
    let mut app = app();
    let mut state = play_state();
    let notes = note_times(&state);
    hit(&mut state, 0, notes[0] - EARLY_BY_US);
    let (fast, slow) = (state.fast(), state.slow());
    assert!(fast.iter().chain(slow.iter()).sum::<u32>() > 0, "the fixture press did not reach a timed judgement");
    let Transition::To(Stage::Result(result)) = enter_result(&mut state, &mut app.shared) else {
        panic!("a finished run moves to the result screen");
    };
    assert_eq!(result.view().fast, fast);
    assert_eq!(result.view().slow, slow);
}

/// A run is paced against a line, not against a total: at the half-way point a target worth 100
/// EX asks for 50, so a run holding 50 there is on the pace rather than 50 behind.
#[test]
fn the_pace_is_the_targets_score_at_this_point_in_the_run() {
    assert_eq!(pace_ex_at(100, 0, 10), 0, "nothing is asked for before the first note");
    assert_eq!(pace_ex_at(100, 5, 10), 50, "half a chart asks for half the target");
    assert_eq!(pace_ex_at(100, 10, 10), 100, "the whole chart asks for the whole target");
    assert_eq!(pace_ex_at(100, 0, 0), 0, "a chart with no notes asks for nothing");
}

/// The HUD paces against the same target the result screen scores the run against, settled once
/// and held for the run.
#[test]
fn the_run_is_paced_against_the_target_the_result_screen_uses() {
    let mut app = app();
    let mut state = play_state();
    state.ensure_pace_target(&app.shared);
    let settled = state.pace_target.clone().expect("a run always has something to pace against");

    let md5 = state.session.model().md5.clone();
    let total_notes = state.session.judge().total_notes();
    let best = app.shared.scores.best_ex_for_md5_in_ln_mode(&md5, &state.ln_mode_key);
    assert_eq!(settled, run_target(&app.shared, &md5, total_notes, best));

    app.shared.config.judge.target = match app.shared.config.judge.target {
        rbms_config::ScoreTarget::Max => rbms_config::ScoreTarget::RateA,
        _ => rbms_config::ScoreTarget::Max,
    };
    state.ensure_pace_target(&app.shared);
    assert_eq!(state.pace_target.as_ref(), Some(&settled), "the target a run is paced against does not move mid-run");
}

/// The layout the run was played on is carried onto the result screen, so the browser, the HUD
/// and the summary all name the same one.
#[test]
fn the_layout_the_run_was_played_on_reaches_the_result_screen() {
    let mut app = app();
    let mut state = play_state();
    let Transition::To(Stage::Result(result)) = enter_result(&mut state, &mut app.shared) else {
        panic!("a finished run moves to the result screen");
    };
    assert_eq!(result.view().mode_label, mode_config_key(app.shared.mode));
}

/// The blocks of the built-in play screen a document can stand in for, in the order the replacement
/// table names them.
const EVERY_BLOCK: &str = r#""field", "gauge", "judge", "score", "counts", "graph", "cover", "frame""#;

/// The objects the replacement table asks for by name, each of which the fixture declares as one
/// plain image: what is being pinned is which blocks step aside, not what replaces them.
const NAMED_OBJECTS: [&str; 15] = [
    "play-gauge-value",
    "play-ex",
    "play-best",
    "play-green",
    "play-count-pg",
    "play-count-gr",
    "play-count-gd",
    "play-count-bd",
    "play-count-pr",
    "play-count-ms",
    "play-fast",
    "play-slow",
    "play-graph-ex",
    "play-graph-best",
    "play-graph-target",
];

/// The six words a judgement pop-up is made of.
const JUDGE_WORDS: [&str; 6] = ["judge-pg", "judge-gr", "judge-gd", "judge-bd", "judge-pr", "judge-ms"];

/// The lane rectangles the fixture states, in the chart's own lane order: the seven keys first and
/// the turntable last, which is how a seven-key chart numbers them. Nothing like the rectangles the
/// shipped layout resolves, so a field taken from the document is unmistakable.
const DOCUMENT_LANES: [(i32, i32); 8] = [(104, 40), (144, 32), (176, 40), (216, 32), (248, 40), (288, 32), (320, 40), (40, 64)];

/// Where the fixture's lanes sit and how tall its notes are, in its own pixels. The document is
/// authored at the canvas size, so these reach the screen unchanged.
const LANE_FOOT: i32 = 220;
const LANE_HEIGHT: i32 = 500;
const NOTE_HEIGHT: f32 = 22.0;

/// Where the screen's first lane starts and how wide the whole field is once the document's lanes
/// have been resolved.
const FIELD_X: f32 = 40.0;
const FIELD_W: f32 = 320.0;

/// A point inside the built-in gauge's empty half, which is a flat fill rather than text and so is
/// the same pixel on every machine. The bar sits ten pixels under the judgement line the document
/// states, and the run's own gauge never reaches this far along it.
const GAUGE_PROBE: (u32, u32) = (350, 515);

/// What the built-in gauge paints that empty half with.
const GAUGE_TROUGH: Color = Color::rgb(28, 28, 36);

/// A layered seven-key document that draws each named block for itself.
///
/// `blocks` is the replacement list it claims and `with_gauge` whether it actually carries the gauge
/// those claims need, which is how a complete claim and one that falls short are told apart.
fn play_document(blocks: &str, with_gauge: bool) -> String {
    let named = NAMED_OBJECTS.iter().filter(|id| with_gauge || **id != "play-gauge-value");
    let every: Vec<&str> = named.chain(JUDGE_WORDS.iter()).chain(["note-cell", "gauge-node", "cover-art"].iter()).copied().collect();
    let images: Vec<String> = every.iter().map(|id| format!(r#"{{ "id": "{id}", "src": "mark", "x": 0, "y": 0, "w": 2, "h": 2 }}"#)).collect();
    let corners: Vec<String> = NAMED_OBJECTS
        .iter()
        .filter(|id| with_gauge || **id != "play-gauge-value")
        .enumerate()
        .map(|(at, id)| format!(r#"{{ "id": "{id}", "dst": [{{ "time": 0, "x": {}, "y": 700, "w": 2, "h": 2 }}] }}"#, 1200 + at as i32 * 4))
        .collect();
    let lanes: Vec<String> = DOCUMENT_LANES.iter().map(|(x, w)| format!(r#"{{ "x": {x}, "y": {LANE_FOOT}, "w": {w}, "h": {LANE_HEIGHT} }}"#)).collect();
    let words: Vec<String> =
        JUDGE_WORDS.iter().map(|id| format!(r#"{{ "id": "{id}", "dst": [{{ "time": 0, "x": 194, "y": 300, "w": 40, "h": 20 }}] }}"#)).collect();
    let gauge = match with_gauge {
        true => r#""gauge": { "id": "gauge", "nodes": ["gauge-node", "gauge-node", "gauge-node", "gauge-node"], "parts": 50 },"#,
        false => "",
    };
    let gauge_dst = match with_gauge {
        true => r#"{ "id": "gauge", "dst": [{ "time": 0, "x": 40, "y": 60, "w": 320, "h": 20 }] },"#,
        false => "",
    };
    format!(
        r#"{{
    "type": 0,
    "composition": "layered",
    "name": "lanes",
    "w": 1280,
    "h": 720,
    "replace": [{blocks}],
    "source": [{{ "id": "mark", "path": "mark.png" }}],
    "image": [{}],
    "note": {{ "id": "notes", "note": [{}], "size": [{}], "dst": [{}] }},
    {gauge}
    "judge": [{{ "id": "judge-1p", "index": 0, "images": [{}] }}],
    "hiddenCover": [{{ "id": "cover", "src": "mark", "x": 0, "y": 0, "w": 2, "h": 2 }}],
    "destination": [
        {{ "id": "notes", "dst": [{{ "time": 0, "x": {FIELD_X}, "y": {LANE_FOOT}, "w": {FIELD_W}, "h": {LANE_HEIGHT} }}] }},
        {gauge_dst}
        {{ "id": "judge-1p", "dst": [{{ "time": 0, "x": 194, "y": 300, "w": 40, "h": 20 }}] }},
        {{ "id": "cover", "dst": [{{ "time": 0, "x": {FIELD_X}, "y": {LANE_FOOT}, "w": {FIELD_W}, "h": {LANE_HEIGHT} }}] }},
        {}
    ]
}}"#,
        images.join(", "),
        vec!["\"note-cell\""; DOCUMENT_LANES.len()].join(", "),
        vec![NOTE_HEIGHT.to_string(); DOCUMENT_LANES.len()].join(", "),
        lanes.join(", "),
        words.join(", "),
        corners.join(", "),
    )
}

/// An app whose settings live in a folder of this test's own, with `document` selected as its
/// seven-key play screen and the source image it draws with written beside it.
fn document_app(tag: &str, document: &str) -> App {
    rbms_render::font::use_embedded_fonts_only();
    let directory = std::env::temp_dir().join(format!("rbms-play-document-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let folder = directory.join(rbms_config::DEFAULT_SKIN_FOLDER);
    std::fs::create_dir_all(&folder).expect("the fixture folder is writable");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 200, 90, 255])).save(folder.join("mark.png")).expect("the source image is written");
    let path = folder.join("lanes.json");
    std::fs::write(&path, document).expect("the document is written");
    let mut config = Config::default();
    config.play.autoplay = false;
    config.skin.select(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS, Some(path.to_string_lossy().into_owned()));
    App::new(String::new(), config, LaunchOptions::default(), directory.join("settings.ron"))
}

/// Draws the play screen until its document has compiled, answering the app and the last frame.
fn play_until_compiled(tag: &str, document: &str) -> (App, crate::stage::HeadlessCanvas) {
    let mut app = document_app(tag, document);
    let mut pixels = crate::stage::HeadlessCanvas::new(CW, CH);
    let compiled = crate::stage::render_tests_skin::render_until_screen_compiled(&mut app, rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS, &mut pixels, || {
        crate::stage::Stage::Play(Box::new(crate::stage::render_tests::play_state()))
    });
    assert!(compiled, "the fixture document never finished compiling: {:?}", app.shared.skin_warnings(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS));
    (app, pixels)
}

#[test]
fn a_play_document_that_draws_its_own_notes_says_where_every_lane_is() {
    let (app, _) = play_until_compiled("lanes", &play_document(EVERY_BLOCK, true));
    let skin = &app.shared.skin;

    for (lane, (x, w)) in DOCUMENT_LANES.iter().enumerate() {
        assert_eq!((skin.x[lane], skin.w[lane]), (*x as f32, *w as f32), "lane {lane} is not where the document put it");
    }
    assert_eq!((skin.top_y, skin.judge_y), (0.0, LANE_HEIGHT as f32), "the field runs from the top of the document's lanes down to their foot");
    assert_eq!(skin.note_height, NOTE_HEIGHT, "a note is as tall as the document's note set says");
    assert_eq!(skin.fields, vec![(FIELD_X, FIELD_W)], "the lanes touch, so they are one field rather than several");
}

#[test]
fn a_complete_replacement_takes_the_built_in_readings_off_the_play_screen() {
    let (app, pixels) = play_until_compiled("replaced", &play_document(EVERY_BLOCK, true));
    let content = app.shared.screen_content(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS).play;

    assert_eq!(
        content,
        rbms_render::content::PlayContent { field: true, gauge: true, judge: true, score: true, counts: true, graph: true, cover: true, frame: true },
        "a document that named every block and compiled every object it needs replaces all of them"
    );
    assert_ne!(pixels.pixel_at(GAUGE_PROBE.0, GAUGE_PROBE.1), GAUGE_TROUGH, "the built-in gauge is still on screen under the document's own");
}

#[test]
fn a_replacement_that_falls_short_leaves_that_block_to_the_built_in_screen() {
    let (app, pixels) = play_until_compiled("short", &play_document(EVERY_BLOCK, false));
    let content = app.shared.screen_content(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS).play;

    assert!(!content.gauge, "a document that claimed the gauge without carrying one does not get it");
    assert!(content.field && content.judge && content.counts, "and the blocks it did carry are replaced all the same");
    assert_eq!(pixels.pixel_at(GAUGE_PROBE.0, GAUGE_PROBE.1), GAUGE_TROUGH, "the built-in gauge stood aside for a document that cannot draw one");
}

/// The lane the fixture chart's first note is on, and where its key bomb burns once that note is
/// hit: the middle of the document's own first lane, at the judgement line the document states.
const BOMB_LANE: usize = 0;
const BOMB_PROBE: (u32, u32) = (124, LANE_HEIGHT as u32);

/// A key bomb belongs to no block of native output: `field` hands over the lane backgrounds, the
/// notes, the judgement line, the key beams and the bar lines, and a document that declares a note
/// field is never asked for a bomb in return. So a document that replaced every block it could still
/// gets one, and it burns on the lanes the document itself laid out.
#[test]
fn a_hit_still_burns_its_key_bomb_over_a_field_the_document_replaced() {
    let (mut app, mut quiet) = play_until_compiled("bomb", &play_document(EVERY_BLOCK, true));
    assert!(app.shared.screen_content(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS).play.field, "the fixture document did not take the field over");

    let notes = note_times(&crate::stage::render_tests::play_state());
    let mut settled = crate::stage::render_tests::play_state();
    settled.song_us = notes[0];
    crate::stage::render_tests::render_into(&mut app, crate::stage::Stage::Play(Box::new(settled)), &mut quiet);

    let mut burning = crate::stage::render_tests::play_state();
    hit(&mut burning, BOMB_LANE, notes[0]);
    burning.song_us = notes[0];
    let mut lit = crate::stage::HeadlessCanvas::new(CW, CH);
    crate::stage::render_tests::render_into(&mut app, crate::stage::Stage::Play(Box::new(burning)), &mut lit);

    let (x, y) = BOMB_PROBE;
    assert_ne!(lit.pixel_at(x, y), quiet.pixel_at(x, y), "the bomb of a note that was hit never reached the judgement line");
}

#[test]
fn a_lane_going_down_starts_the_key_timer_that_lane_publishes() {
    let (mut app, mut pixels) = play_until_compiled("keyon", &play_document(EVERY_BLOCK, true));
    let first_key = rbms_skin::timer::timer_id::KEYON_1P_KEY1;
    assert!(app.shared.skin_timers.is_off(first_key), "no key has been touched yet");

    let mut state = crate::stage::render_tests::play_state();
    let now = Instant::now();
    state.handle_pad(
        &mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 },
        crate::gamepad::PadEvent::Lane { lane: 0, dir: rbms_judge::matcher::ScratchDir::Forward, press: true },
    );
    crate::stage::render_tests::render_into(&mut app, crate::stage::Stage::Play(Box::new(state)), &mut pixels);

    assert!(app.shared.skin_timers.is_on(first_key), "the first key of the left-hand field went down and its timer did not start");
    assert!(app.shared.skin_timers.is_off(rbms_skin::timer::timer_id::KEYOFF_1P_KEY1), "a key that is down is not also reported as released");
    assert!(app.shared.skin_timers.is_off(rbms_skin::timer::timer_id::KEYON_1P_SCRATCH), "and no other lane was touched");
}

/// Where the nudged mark sits in the document, in its own y-up space, and on screen once drawn.
const NUDGED_MARK_DOCUMENT: (i32, i32) = (600, 300);
const NUDGED_MARK_SCREEN: (u32, u32) = (601, 417);
const NUDGE_X: f32 = 50.0;
const NUDGED_MARK_OFFSET_ID: i32 = 40;

#[test]
fn a_play_document_reads_the_nudges_the_player_stored_for_it() {
    let (x, y) = NUDGED_MARK_DOCUMENT;
    let nudged = play_document(EVERY_BLOCK, true)
        .replacen(r#""image": ["#, r#""image": [{ "id": "mark-dot", "src": "mark", "x": 0, "y": 0, "w": 2, "h": 2 }, "#, 1)
        .replacen(
            r#""destination": ["#,
            &format!(
                r#""destination": [{{ "id": "mark-dot", "offset": {NUDGED_MARK_OFFSET_ID}, "dst": [{{ "time": 0, "x": {x}, "y": {y}, "w": 4, "h": 4 }}] }}, "#
            ),
            1,
        );
    let (mut app, mut pixels) = play_until_compiled("nudge", &nudged);
    let mark = Color::rgb(12, 200, 90);
    let (sx, sy) = NUDGED_MARK_SCREEN;
    assert_eq!(pixels.pixel_at(sx, sy), mark, "before any nudge the object sits where the document put it");

    let path = app.shared.config.skin.document(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS).expect("the fixture document is selected").to_owned();
    app.shared
        .config
        .skin
        .customise(&path)
        .offsets
        .insert(NUDGED_MARK_OFFSET_ID, rbms_skin::dst::SkinOffset { x: NUDGE_X, y: 0.0, w: 0.0, h: 0.0, r: 0.0, a: 0.0 });
    let redrawn = crate::stage::render_tests_skin::render_until_screen_compiled(&mut app, rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS, &mut pixels, || {
        crate::stage::Stage::Play(Box::new(crate::stage::render_tests::play_state()))
    });
    assert!(redrawn, "the document stays compiled while a nudge is stored");
    assert_eq!(pixels.pixel_at(sx + NUDGE_X as u32, sy), mark, "the nudge the player stored moved the object on the play screen");
    assert_ne!(pixels.pixel_at(sx, sy), mark, "and it left the place it was drawn at before");
}
