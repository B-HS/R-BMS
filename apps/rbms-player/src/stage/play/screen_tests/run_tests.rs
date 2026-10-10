//! What a run hands the skin that draws it as it goes, on the play screen entered the way the
//! application enters it: the chart its note field is shown, the keys and the long notes of its
//! lanes, its background, and what the screen shows while it waits with no skin to draw the wait.

use rbms_render::skin_render::frame::{BgaPick, BgaShow};

use super::*;

/// The colour of the placeholder the note field draws a long note from when the skin gives it no
/// image, which the fixture does not.
const LONG_NOTE: Color = Color::rgb(255, 255, 0);

/// The turntable's lane of a seven-key chart.
const SCRATCH_LANE: usize = 7;

/// Every chart below ends on a note in the sixth key, well after anything a test looks at, which is
/// what makes it a seven-key chart rather than a five-key one.
///
/// A seven-key chart at twice the tempo a CONSTANT field scrolls at, with one note in its first key
/// two seconds in.
const FAST_CHART: &[u8] = b"#PLAYER 1\n#BPM 240\n#WAV01 a.wav\n#00211:01\n#01618:01\n";
const FAST_NOTE_MS: i64 = 2_000;

/// How long a note takes to cross the field on that chart as the chart scrolls, at a speed of one.
const FAST_REGION_MS: f64 = 1_000.0;

/// The scroll speed the fields below are measured at.
const PLAIN_HISPEED: f64 = 1.0;

/// How long START and SELECT have to have been up before a chart that is in is ready.
const START_RELEASE_WAIT_MS: i64 = 1_000;

/// How many frames in a row a lane the run holds is looked at.
const HELD_FRAMES: usize = 3;

/// A seven-key chart with one long note in its first key, from four seconds in to five.
const LONG_CHART: &[u8] = b"#PLAYER 1\n#BPM 120\n#LNTYPE 1\n#WAV01 a.wav\n#00251:0101\n#00818:01\n";
const LONG_HEAD_MS: i64 = 4_000;

/// A seven-key chart with one long note on its turntable, from two seconds in to three.
const TURNTABLE_CHART: &[u8] = b"#PLAYER 1\n#BPM 120\n#LNTYPE 1\n#WAV01 a.wav\n#00156:0101\n#00818:01\n";
const TURNTABLE_HELD_MS: i64 = 2_500;

/// A seven-key chart that puts its first picture up at its very start.
const PICTURE_CHART: &[u8] = b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#BMP01 a.bmp\n#00004:01\n#00818:01\n";
const FIRST_PICTURE: i32 = 1;

/// How far apart the points two frames are held against each other at are, in the pixels the
/// fixture is authored in.
const SAMPLE_STEP: usize = 8;

/// A play skin whose header reads and whose screen cannot be built, so it is found for the mode and
/// found unreadable only once a screen asks for it.
const UNBUILDABLE_SKIN: &str = "local skin = { type = 0, name = \"Unbuildable Play\", w = 1280, h = 720, loadend = 2000, playstart = 1000 }\n\
if skin_config then\n    error(\"this skin cannot be built\")\nend\nreturn skin\n";

/// The gauge a stage of a course is played on, full, as the stage before left it.
const COURSE_GAUGE: (rbms_judge::gauge::GaugeIndex, f32) = (rbms_judge::gauge::GaugeIndex::Class, 100.0);

/// A run of `chart` on `options`, with nothing left to decode.
fn run_of(chart: &[u8], options: SessionOptions) -> PlayState {
    let src = rbms_parser::parse_with(chart, Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, CHART_NAME));
    PlayState::new(PlaySession::new(model, options), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// A replay that presses nothing.
fn idle_replay() -> rbms_store::Replay {
    rbms_store::Replay {
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
    }
}

/// The settings a note field is measured under: one speed, and nothing over the field.
fn plain_field(app: &mut App) {
    let play = &mut app.shared.config.play;
    play.hispeed = PLAIN_HISPEED;
    (play.enable_cover, play.enable_lift, play.enable_hidden) = (false, false, false);
    (play.constant_speed, play.legacy_note) = (false, false);
}

/// The colours of the first lane's column of the note field, top to bottom.
fn first_lane_column(pixels: &HeadlessCanvas) -> Vec<Color> {
    (FIELD_TOP..FIELD_BOTTOM).map(|y| pixel(pixels, (FIRST_LANE_X, y))).collect()
}

/// The play screen that is up and the application state beside it, both to hand.
fn play_and_shared(app: &mut App) -> (&mut PlayState, &mut AppShared) {
    let App { stage, shared, .. } = app;
    let Stage::Play(state) = stage else {
        panic!("the play screen is not up");
    };
    (state, shared)
}

/// CONSTANT scrolls a skin's note field at the built-in field's speed whatever the chart's tempo: a
/// note a second and a half away on a chart at 240 is out of sight as the chart scrolls and three
/// quarters of the way up the field under CONSTANT. The travel time the skin reads follows.
#[test]
fn constant_scrolls_a_skins_note_field_at_one_speed_whatever_the_tempo() {
    const AHEAD_MS: i64 = 1_500;
    for constant in [false, true] {
        let mut app = app_with_play_skin(if constant { "constant-on" } else { "constant-off" });
        let mut pixels = canvas();
        plain_field(&mut app);
        app.shared.config.play.constant_speed = constant;
        started(&mut app, &mut pixels, run_of(FAST_CHART, SessionOptions::default()));
        stay_after(&mut app, &mut pixels, FAST_NOTE_MS - AHEAD_MS);

        let in_sight = first_lane_column(&pixels).contains(&NOTE);
        assert_eq!(in_sight, constant, "with CONSTANT {constant}, a note {AHEAD_MS} ms away at 240 BPM");
        let (state, shared) = play_and_shared(&mut app);
        assert_eq!(state.field.as_ref().map(FieldChart::settings), constant.then_some((true, false)));
        let (bpm, scroll) = state.tempo_at(state.drawn_chart_us(shared));
        let travel = state.field.as_ref().and_then(FieldChart::tempo).unwrap_or((bpm, scroll));
        let region_ms = travel_region_ms(travel.0, PLAIN_HISPEED as f32, travel.1);
        let expected_ms = if constant { rbms_chart::scroll::CONSTANT_GREEN_BASE_MS } else { FAST_REGION_MS };
        assert_eq!(region_ms, expected_ms, "the travel time the skin reads does not follow the field");
    }
}

/// LEGACY NOTE draws a long note on a skin's note field as the plain note at its head: with it off
/// the field shows the long note's body, and with it on a plain note and no body. The run holds the
/// chart as it was written either way.
#[test]
fn legacy_note_draws_a_long_note_on_a_skins_field_as_the_note_at_its_head() {
    const BEFORE_HEAD_MS: i64 = 500;
    for legacy_note in [false, true] {
        let mut app = app_with_play_skin(if legacy_note { "legacy-on" } else { "legacy-off" });
        let mut pixels = canvas();
        plain_field(&mut app);
        app.shared.config.play.legacy_note = legacy_note;
        started(&mut app, &mut pixels, run_of(LONG_CHART, SessionOptions::default()));
        stay_after(&mut app, &mut pixels, LONG_HEAD_MS - BEFORE_HEAD_MS);

        let column = first_lane_column(&pixels);
        assert_eq!(column.contains(&LONG_NOTE), !legacy_note, "with LEGACY NOTE {legacy_note}, the body of a long note");
        assert_eq!(column.contains(&NOTE), legacy_note, "with LEGACY NOTE {legacy_note}, a plain note at the long note's head");
        let judged_as_long = play(&app).session.judge().note_marks().any(|mark| mark.long_end);
        assert!(judged_as_long, "the run lost its long note to a setting that only changes what is drawn");
    }
}

/// A key that comes up once the run is closing is not the run's: the judge has stopped, so the
/// replay ends with the run as it stood and no long note is ended after the fact.
#[test]
fn a_key_that_comes_up_while_the_run_is_closing_is_not_handed_to_the_run() {
    let mut app = app_with_play_skin("closing-release");
    let mut pixels = canvas();
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == 0).map(|(code, _)| *code).expect("the first lane has a key");
    started(&mut app, &mut pixels, unplayed());

    key(&mut app, press(lane_key));
    let recorded = |app: &App| play(app).session.recorded_events().iter().map(|event| event.press).collect::<Vec<_>>();
    assert_eq!(recorded(&app), [true], "a press made while the chart plays was not the run's");
    stay_after(&mut app, &mut pixels, FIRST_NOTE_MS + MISS_AFTER_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Failed));

    key(&mut app, release(lane_key));
    assert_eq!(recorded(&app), [true], "a key coming up while the run closes was handed to the run");
    stay(&mut app, &mut pixels);
    assert!(!app.shared.skin_timers.is_on(timer_id::KEYON_1P_KEY1), "the beam of a key that came up is still lit");
}

/// A lane the run plays for the player holds its long notes and keeps its beam lit as a key would,
/// frame after frame, though the player holds nothing there.
#[test]
fn a_lane_the_run_plays_for_the_player_keeps_its_beam_and_holds_its_long_note() {
    let mut app = app_with_play_skin("auto-lane");
    let mut pixels = canvas();
    let auto_lanes = (0..=SCRATCH_LANE).map(|lane| lane == SCRATCH_LANE).collect();
    started(&mut app, &mut pixels, run_of(TURNTABLE_CHART, SessionOptions { auto_lanes, ..SessionOptions::default() }));
    assert!(!PlayState::plays_back(&app.shared), "the run is played by hand");

    stay_after(&mut app, &mut pixels, TURNTABLE_HELD_MS);
    assert!(play(&app).session.judge().long_note(SCRATCH_LANE).processing.is_some(), "the run is holding the turntable's long note");
    for frame in 0..HELD_FRAMES {
        stay(&mut app, &mut pixels);
        assert!(app.shared.skin_timers.is_on(timer_id::KEYON_1P_SCRATCH), "the turntable's beam went out on frame {frame} of a note the run is holding");
        assert!(app.shared.skin_timers.is_on(timer_id::HOLD_1P_SCRATCH), "the turntable's hold timer is off on frame {frame}");
    }
}

/// A full combo is the chart's own: a later stage of a course begins with the combo the course
/// carried into it, and playing every note of the stage still switches the full combo timer on.
#[test]
fn a_stage_of_a_course_reaches_its_full_combo_whatever_combo_it_began_with() {
    const CARRIED: u32 = 120;
    for carried in [0, CARRIED] {
        let mut app = app_with_play_skin(if carried == 0 { "combo-fresh" } else { "combo-carried" });
        let mut pixels = canvas();
        app.shared.config.play.autoplay = true;
        let initial_gauge = (carried > 0).then_some(COURSE_GAUGE);
        started(&mut app, &mut pixels, run(SessionOptions { autoplay: true, initial_combo: carried, initial_gauge, ..SessionOptions::default() }));
        assert_eq!(play(&app).session.judge().combo, carried);
        assert!(!app.shared.skin_timers.is_on(timer_id::FULLCOMBO_1P));

        stay_after(&mut app, &mut pixels, LAST_NOTE_MS + SLACK_MS);
        let judge = play(&app).session.judge();
        assert_eq!((judge.total_judged(), judge.combo), (judge.total_notes(), carried + judge.total_notes()), "the stage was played whole");
        assert!(app.shared.skin_timers.is_on(timer_id::FULLCOMBO_1P), "a stage begun on a combo of {carried} did not reach its full combo");
    }
}

/// A player who turned backgrounds off is shown the black of a chart with no background, before
/// the chart plays and while it does, rather than nothing at all.
#[test]
fn with_backgrounds_off_a_skins_background_is_black_from_first_to_last() {
    let mut app = app_with_play_skin("bga-off");
    let mut pixels = canvas();
    app.shared.config.display.bga = false;
    let run = autoplayed(&mut app);
    enter(&mut app, run);
    run_until_drawn(&mut app, &mut pixels);
    let shown = |app: &mut App, pixels: &mut HeadlessCanvas| {
        let (state, shared) = play_and_shared(app);
        state.background_frame(shared, &mut Canvas::Headless(pixels), None).show
    };
    assert_eq!(shown(&mut app, &mut pixels), BgaShow::Blank, "before the chart plays");

    stay_after(&mut app, &mut pixels, TIMES.loadend_ms + SLACK_MS);
    stay_after(&mut app, &mut pixels, TIMES.playstart_ms + SLACK_MS);
    stay_after(&mut app, &mut pixels, FIRST_NOTE_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Play));
    assert_eq!(shown(&mut app, &mut pixels), BgaShow::Blank, "while the chart plays");
}

/// A jump of a replay under analysis starts the background over as a chart that is yet to play, so
/// a picture the chart put up at its very start is back on the frame after the jump.
#[test]
fn a_jump_keeps_the_picture_a_chart_put_up_at_its_very_start() {
    let mut app = app_with_play_skin("bga-seek");
    let mut pixels = canvas();
    app.shared.config.display.bga = true;
    let replay = idle_replay();
    app.shared.replay = Some(replay.clone());
    started(&mut app, &mut pixels, run_of(PICTURE_CHART, SessionOptions { analysis: true, replay: Some(replay), ..SessionOptions::default() }));
    stay_after(&mut app, &mut pixels, SLACK_MS);
    let showing = BgaPick::Playing { base: Some(FIRST_PICTURE), layer: None };
    assert_eq!(play(&app).background.pick(), showing, "the chart's first picture is not up");

    assert!(matches!(key(&mut app, press(KeyCode::PageUp)), Transition::Stay));
    stay(&mut app, &mut pixels);
    assert_eq!(play(&app).background.pick(), showing, "the picture at the chart's start was lost to a jump");
}

/// A play skin that cannot be read is not known to be unreadable until the play screen is up and
/// waiting for its chart. The wait is then shown as the built-in LOADING screen shows it, and the
/// built-in field takes over once the chart plays.
#[test]
fn a_play_screen_whose_skin_cannot_be_read_waits_on_the_built_in_loading_display() {
    let mut app = app_with_skins("unreadable", &[("play7.luaskin", UNBUILDABLE_SKIN)]);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(app.shared.has_play_scene(), "a skin nobody has tried to read yet is taken to be readable");
    enter(&mut app, unplayed());
    assert_eq!(phase(&app), Some(PlayPhase::Preload));
    let failed = (0..SKIN_FRAMES).any(|_| {
        stay(&mut app, &mut pixels);
        app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS).is_some()
    });
    assert!(failed, "the unreadable skin was never found to be unreadable");

    stay(&mut app, &mut pixels);
    assert_eq!(phase(&app), Some(PlayPhase::Preload), "the screen is still waiting for START and SELECT to have been up for a second");
    let mut built_in = HeadlessCanvas::new(CW, CH);
    ChartLoads::default().draw(&app.shared, &mut Canvas::Headless(&mut built_in));
    let points = || (0..CH).step_by(SAMPLE_STEP).flat_map(|y| (0..CW).step_by(SAMPLE_STEP).map(move |x| (x, y)));
    let waiting: Vec<Color> = points().map(|(x, y)| pixels.pixel_at(x, y)).collect();
    let expected: Vec<Color> = points().map(|(x, y)| built_in.pixel_at(x, y)).collect();
    assert!(waiting == expected, "a screen waiting with no skin does not show the built-in LOADING display");
    assert!(expected.iter().any(|colour| *colour != expected[0]), "the built-in LOADING display is one flat colour");

    stay_after(&mut app, &mut pixels, START_RELEASE_WAIT_MS + SLACK_MS);
    stay_after(&mut app, &mut pixels, SLACK_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Play), "a chart with no skin to time it did not start once it was in");
    stay(&mut app, &mut pixels);
    let playing: Vec<Color> = points().map(|(x, y)| pixels.pixel_at(x, y)).collect();
    assert!(playing != expected, "the LOADING display stayed up over a chart that is playing");
}
