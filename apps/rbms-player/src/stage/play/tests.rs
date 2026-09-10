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
