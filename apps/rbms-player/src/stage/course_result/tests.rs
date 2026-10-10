//! The course result screen: first as the built-in screen it is without a skin, then drawn by a
//! small skin of its own, whose scene runs as the single chart's result scene does and always
//! leaves for the browser.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rbms_course::{Course, CourseChart, CourseRun, StageResult};
use rbms_model::Mode;
use rbms_play::{NullSink, PlayRecord, PlaySession, SessionClock, SessionOptions};
use rbms_skin::timer::timer_id;

use super::*;
use crate::stage::capture::{app_in, settings_of};
use crate::stage::render_tests::{FRAME_DT, app};
use crate::stage::result::next_course_gauge_type;
use crate::stage::scene_life::takes_input;
use crate::stage::select::tests::{press, release};
use crate::stage::{HeadlessCanvas, Stage, StageId};
use crate::{App, CH, Config};

/// The times of the fixture skin.
const INPUT_MS: i64 = 2000;
const FADEOUT_MS: i64 = 4000;

/// The first moment the scene listens.
const LISTENING_MS: i64 = INPUT_MS + 1;

/// The keys the shipped seven-key layout plays the graph key and the seventh key on.
const GRAPH_KEY: KeyCode = KeyCode::KeyF;
const AGAIN_KEY: KeyCode = KeyCode::KeyV;

/// Frames a skin with no files is given to be read and compiled.
const SKIN_FRAMES: usize = 240;

/// The fixture skin: a course result skin that paints the screen one colour, a square for each timer
/// the scene switches, and a square for each fact about the course it is told.
const SKIN: &str = include_str!("fixture/course.luaskin");

/// What the fixture paints, and where: the ground, listening and fading in the top corners, the three
/// score timers and then the facts along the bottom.
const GROUND: Color = Color::rgb(20, 40, 80);
const LISTENING_MARK: Color = Color::rgb(0, 255, 0);
const FADING_MARK: Color = Color::rgb(255, 0, 0);
const SCORE_MARKS: [(u32, Color); 3] = [(10, Color::rgb(255, 255, 0)), (110, Color::rgb(255, 0, 255)), (210, Color::rgb(0, 255, 255))];
const TITLES_MARK: (u32, Color) = (310, Color::rgb(255, 128, 0));
const NOTES_MARK: (u32, Color) = (410, Color::rgb(128, 255, 0));
const CLEARED_MARK: (u32, Color) = (510, Color::rgb(0, 128, 255));
const FAILED_MARK: (u32, Color) = (610, Color::rgb(255, 0, 128));
const SENDING_MARK: (u32, Color) = (710, Color::rgb(0, 0, 255));
const SENT_MARK: (u32, Color) = (810, Color::rgb(128, 0, 255));
const LISTENING_MARK_AT: (u32, u32) = (10, 10);
const FADING_MARK_AT: (u32, u32) = (CW - 10, 10);
const GROUND_AT: (u32, u32) = (CW / 2, 100);
const FACT_MARK_ROW: u32 = CH - 10;

/// Where the fixture draws its gauge graph, in pixels: the left edge, its width, and the rows it
/// covers.
const GRAPH_LEFT: u32 = 140;
const GRAPH_WIDTH: u32 = 1000;
const GRAPH_ROWS: std::ops::Range<u32> = 220..420;

/// How many rows of the boundary's column show the line. The history's own lines are drawn over
/// it, where the gauge moves through that column, so only a stretch of it is bare.
const MIN_LINE_ROWS: usize = 20;

/// The line a graph draws where one stage of a course ends.
const SECTION_LINE: Color = Color::rgb(255, 255, 255);

/// Four notes on one key lane, which is all a run needs to leave a record behind.
const RECORD_CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n";
const RECORD_FRAME_US: i64 = 10_000;
const RECORD_RUN_US: i64 = 9_000_000;

/// An autoplayed run of the chart above, every note a PGREAT.
fn record() -> PlayRecord {
    let model = rbms_chart::to_model(&rbms_parser::parse(RECORD_CHART), Mode::BEAT_7K);
    let mut session = PlaySession::new(model, SessionOptions { autoplay: true, ..SessionOptions::default() });
    for frame in 0..=RECORD_RUN_US / RECORD_FRAME_US {
        session.tick(SessionClock::at(frame * RECORD_FRAME_US), &mut NullSink);
    }
    session.record()
}

/// A course of `stages` charts, played to the end or failed on its last stage.
fn run_of(stages: usize, failed: bool) -> CourseRun {
    run_ending(stages, failed.then_some(stages))
}

/// A course of `stages` charts, played to the end or failed on the stage numbered `failing`, counted
/// from one, with the stages after it never reached.
fn run_ending(stages: usize, failing: Option<usize>) -> CourseRun {
    let mut course = Course {
        name: "TEST COURSE".to_string(),
        charts: (0..stages).map(|i| CourseChart { md5: format!("{i:032x}"), sha256: String::new(), title: format!("stage {i}") }).collect(),
        ..Course::default()
    };
    assert!(course.validate(), "the fixture course is valid");
    let mut run = CourseRun::new(course, 100.0);
    for stage in 1..=failing.unwrap_or(stages) as u32 {
        let survived = failing != Some(stage as usize);
        let carried = 4 * stage;
        run.advance(&StageResult {
            ex_score: 8,
            max_ex_score: 8,
            notes: 4,
            counts: [4, 0, 0, 0, 0, 0],
            max_combo: carried,
            combo_at_end: carried,
            gauge_value: if survived { 100.0 } else { 0.0 },
            clear: if survived { 5 } else { 1 },
            survived,
            ..StageResult::default()
        });
    }
    run
}

/// Put the finished course in `app` the way the application leaves it when the last stage ends: the
/// run alive and one record for each stage that was played.
fn finish_course(mut app: App, run: CourseRun) -> (App, CourseRun) {
    let record = record();
    let played = run.failed_at.map_or(run.index, |failed| failed + 1);
    app.shared.run_records = vec![record; played];
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    app.shared.course_run = Some(run.clone());
    (app, run)
}

/// An app with no skin at all.
fn plain_app() -> App {
    app()
}

/// An app whose skin pack is one folder holding `skin` as its course result skin.
fn skinned_app(tag: &str, skin: &str) -> App {
    let settings = settings_of(&format!("course-result-{tag}"));
    let pack: PathBuf = settings.with_file_name("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join("course.luaskin"), skin).expect("the skin is written");
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    app_in(settings, config)
}

/// Enter the course result screen the way the application does.
fn enter(app: &mut App, run: &CourseRun) {
    let state = CourseResultState::of(run, &app.shared);
    app.switch(Transition::To(Stage::CourseResult(Box::new(state))));
    assert_eq!(app.stage.id(), StageId::CourseResult);
}

fn course_screen(app: &App) -> &CourseResultState {
    let Stage::CourseResult(state) = &app.stage else {
        panic!("the {:?} screen is up, not the course result screen", app.stage);
    };
    state
}

/// One frame of the screen that is up: its update, and its draw unless the update asked to leave.
fn frame(app: &mut App, pixels: &mut HeadlessCanvas) -> Transition {
    let now = Instant::now();
    let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
    if matches!(transition, Transition::Stay) {
        app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, &mut Canvas::Headless(pixels));
    }
    transition
}

/// Run frames until the skin has drawn and the scene has begun.
fn run_until_begun(app: &mut App, pixels: &mut HeadlessCanvas) {
    for _ in 0..SKIN_FRAMES {
        assert!(matches!(frame(app, pixels), Transition::Stay), "the screen left before its scene began");
        if course_screen(app).scene.begun {
            return;
        }
    }
    panic!("the course result skin never drew: {:?}", app.shared.skin_failure(SKIN_TYPE_COURSE_RESULT));
}

/// Move the scene clock on by `millis` and run one frame.
fn frame_after_ms(app: &mut App, pixels: &mut HeadlessCanvas, millis: i64) -> Transition {
    app.shared.age_skin_scene(Duration::from_millis(u64::try_from(millis).expect("a scene only moves forward")));
    frame(app, pixels)
}

fn key(app: &mut App, key: KeyInput<'_>) -> Transition {
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key)
}

/// A course result drawn by the fixture skin whose scene has begun and is listening.
fn listening_screen(tag: &str, stages: usize, failed: bool) -> (App, HeadlessCanvas) {
    let (mut app, run) = finish_course(skinned_app(tag, SKIN), run_of(stages, failed));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    run_until_begun(&mut app, &mut pixels);
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));
    assert!(takes_input(&app.shared.skin_timers), "the scene is not listening past its input time");
    (app, pixels)
}

fn mark_at(pixels: &HeadlessCanvas, (x, color): (u32, Color)) -> bool {
    pixels.pixel_at(x, FACT_MARK_ROW) == color
}

#[test]
fn a_course_that_reached_the_end_reads_as_cleared() {
    let (app, run) = finish_course(plain_app(), run_of(2, false));
    let state = CourseResultState::of(&run, &app.shared);

    assert!(state.cleared());
    assert!(state.rows().iter().any(|(label, _)| label == "EX SCORE"));
}

#[test]
fn a_course_that_ended_early_names_the_stage_it_failed_on() {
    let (app, run) = finish_course(plain_app(), run_of(3, true));
    let state = CourseResultState::of(&run, &app.shared);

    assert!(!state.cleared());
    assert_eq!(state.rows().iter().find(|(label, _)| label == "FAILED AT").map(|(_, value)| value.as_str()), Some("STAGE 3"));
}

/// With no skin chosen nothing of a skin is built: the screen is the built-in one, which acts on its
/// keys at once and leaves for the browser with the player's own settings back.
#[test]
fn without_a_skin_the_screen_is_the_built_in_one() {
    let (mut app, run) = finish_course(plain_app(), run_of(2, false));
    app.shared.course_settings_backup = Some(app.shared.config.play.clone());
    enter(&mut app, &run);
    let state = course_screen(&app);
    assert!(state.skin.is_none() && state.status == SkinStatus::Gone, "a skin's data was built with no skin to draw");
    assert!(state.announced, "the arrival cue was not raised on entry");

    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(matches!(key(&mut app, press(KeyCode::KeyR)), Transition::Stay), "the run-again key did something on a course result");
    assert!(matches!(key(&mut app, press(KeyCode::KeyN)), Transition::Stay), "the next-chart key did something on a course result");
    assert!(app.shared.course_run.is_some());

    assert!(matches!(key(&mut app, press(KeyCode::Enter)), Transition::Back));
    assert!(app.shared.course_run.is_none() && app.shared.course_settings_backup.is_none(), "the course was not ended");
}

/// The built-in screen is the same screen it was before a skin could draw it: the rows on their
/// plates over the theme's background.
#[test]
fn the_built_in_screen_draws_its_rows() {
    let (mut app, run) = finish_course(plain_app(), run_of(2, false));
    enter(&mut app, &run);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));

    let th = rbms_render::theme();
    let x = (CW as f32 - PANEL_W) * 0.5 + 1.0;
    assert_eq!(pixels.pixel_at(2, 2), th.bg, "the background was not cleared");
    assert_eq!(pixels.pixel_at(x as u32, ROW_TOP as u32 + 1), th.panel, "the first row's plate is not where it was");
}

/// The course is taken as one run of every stage: its notes, its judgements, the titles of its
/// charts, and the gauge it opens its graph on.
#[test]
fn the_course_is_taken_as_one_run_of_every_stage() {
    let (app, run) = finish_course(skinned_app("one-run", SKIN), run_of(2, false));
    let state = CourseResultState::of(&run, &app.shared);
    let skin = state.skin.as_ref().expect("a skin is chosen for the screen");

    let snapshot = &skin.snapshot;
    assert_eq!((snapshot.score.sheet.notes, snapshot.score.sheet.count(0), snapshot.score.sheet.max_combo), (8, 8, 8));
    assert_eq!(snapshot.course_titles, ["stage 0", "stage 1"]);
    assert!(snapshot.cleared());
    assert_eq!(skin.chart().notes, Some(8), "the chart cluster is told the notes of the whole course");
    assert_eq!(skin.chart().heading, Some("TEST COURSE"));
    assert_eq!(state.scene.gauge_type, skin.finished_gauge());
}

/// A failed course is charged the stages it never reached: those stages are not in the library, so
/// they count for nothing here, but the course is failed and says so everywhere.
#[test]
fn a_failed_course_reads_as_failed_to_its_skin() {
    let (app, run) = finish_course(skinned_app("failed-run", SKIN), run_of(2, true));
    let state = CourseResultState::of(&run, &app.shared);
    let skin = state.skin.as_ref().expect("a skin is chosen for the screen");

    assert!(!skin.snapshot.cleared());
    assert_eq!(skin.snapshot.score.sheet.clear, rbms_judge::clear_type_id(rbms_judge::ClearType::Failed));
}

/// The gauge bar of a course reads the end of the course's last stage, which for a stage the course
/// never reached is the floor that stage was filled with; the gauge number beside it still reads
/// where the last stage played ended (`SkinGauge.prepare`, `createGrooveGaugeProperty`).
#[test]
fn the_gauge_bar_of_a_course_that_stopped_short_is_empty() {
    let (app, run) = finish_course(skinned_app("stopped-short", SKIN), run_ending(3, Some(2)));
    assert_eq!(app.shared.run_records.len(), 2, "the stage after the failure was played");
    let state = CourseResultState::of(&run, &app.shared);
    let skin = state.skin.as_ref().expect("a skin is chosen for the screen");
    let shown = skin.finished_gauge();

    assert_eq!(skin.gauge(shown).value, Some(0.0), "the bar shows a stage the course did reach");
    assert_eq!(skin.snapshot.gauge.value(shown), record().gauge_log.last(record().finished_gauge), "the number reads the last stage played");

    let (app, run) = finish_course(skinned_app("ran-through", SKIN), run_of(2, false));
    let state = CourseResultState::of(&run, &app.shared);
    let skin = state.skin.as_ref().expect("a skin is chosen for the screen");
    assert_eq!(skin.gauge(shown).value, record().gauge_log.last(record().finished_gauge), "a course played to its end shows its last stage");
}

/// The score server's timers follow the submission on a course result as they do on a chart's: one
/// when a score is sent, and one for how that ended.
#[test]
fn the_submission_switches_the_score_servers_timers() {
    use crate::ir_outcome::{IrReport, IrStatus};

    let (mut app, mut pixels) = listening_screen("submission", 2, false);
    assert!(!app.shared.skin_timers.is_on(timer_id::IR_CONNECT_BEGIN), "a course that was never sent started the connection timer");
    assert!(!mark_at(&pixels, SENDING_MARK) && !mark_at(&pixels, SENT_MARK));

    app.shared.ir_status = IrStatus::Sending;
    frame(&mut app, &mut pixels);
    assert!(app.shared.skin_timers.is_on(timer_id::IR_CONNECT_BEGIN) && !app.shared.skin_timers.is_on(timer_id::IR_CONNECT_SUCCESS));
    assert!(mark_at(&pixels, SENDING_MARK) && !mark_at(&pixels, SENT_MARK), "the skin does not show the submission under way");

    app.shared.ir_status = IrStatus::Reported(IrReport { score: "IR 1/1".to_string(), new_best: false, replay: None, failed: false });
    frame(&mut app, &mut pixels);
    assert!(app.shared.skin_timers.is_on(timer_id::IR_CONNECT_SUCCESS) && !app.shared.skin_timers.is_on(timer_id::IR_CONNECT_FAIL));
    assert!(mark_at(&pixels, SENT_MARK), "the skin does not show the submission answered");
}

/// The frame the skin first draws on is the scene's first moment: the clock is back at zero, the
/// three score timers are on from it, the skin is told the course, and listening and fading are not
/// shown yet.
#[test]
fn the_scene_begins_on_the_frame_its_skin_first_draws() {
    const WAITED: Duration = Duration::from_secs(30);
    let (mut app, run) = finish_course(skinned_app("begins", SKIN), run_of(2, false));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    assert_eq!(course_screen(&app).status, SkinStatus::Reading);
    app.shared.age_skin_scene(WAITED);
    run_until_begun(&mut app, &mut pixels);

    let now_us = app.shared.skin_now_us();
    assert!(now_us < WAITED.as_micros() as i64 / 2, "the scene clock was not started with the skin: {now_us} us");
    for timer in [timer_id::RESULTGRAPH_BEGIN, timer_id::RESULTGRAPH_END, timer_id::RESULT_UPDATESCORE] {
        assert!(app.shared.skin_timers.is_on(timer), "{timer:?} is not on");
        assert!(app.shared.skin_timers.value_us(timer) <= now_us, "{timer:?} was switched on against the old clock");
    }
    assert_eq!(pixels.pixel_at(GROUND_AT.0, GROUND_AT.1), GROUND, "the skin did not draw the screen");
    for mark in SCORE_MARKS {
        assert!(mark_at(&pixels, mark), "a score timer's mark is missing at {}", mark.0);
    }
    assert_ne!(pixels.pixel_at(LISTENING_MARK_AT.0, LISTENING_MARK_AT.1), LISTENING_MARK, "the scene listens from its first frame");
    assert_ne!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), FADING_MARK);
}

/// What the skin is told of the course reaches it through the clusters: the titles of the charts, the
/// notes of the whole course, and that it cleared.
#[test]
fn the_skin_is_told_the_course_by_the_clusters() {
    let (mut app, run) = finish_course(skinned_app("told", SKIN), run_of(2, false));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    run_until_begun(&mut app, &mut pixels);

    assert!(mark_at(&pixels, TITLES_MARK), "the course's titles did not reach the skin, or the tenth title was not empty");
    assert!(mark_at(&pixels, NOTES_MARK), "the skin was not told the notes of the whole course");
    assert!(mark_at(&pixels, CLEARED_MARK), "a cleared course does not read as cleared");
    assert!(!mark_at(&pixels, FAILED_MARK), "a cleared course reads as failed");
}

#[test]
fn a_failed_course_is_shown_as_failed() {
    let (mut app, run) = finish_course(skinned_app("failed-shown", SKIN), run_of(2, true));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    run_until_begun(&mut app, &mut pixels);

    assert!(mark_at(&pixels, FAILED_MARK), "a failed course does not read as failed");
    assert!(!mark_at(&pixels, CLEARED_MARK), "a failed course reads as cleared");
}

/// The stages' gauge histories are drawn end to end with a white line where a stage ends.
#[test]
fn the_gauge_graph_marks_where_each_stage_ends() {
    let each = record().gauge_log.len() as u32;
    let boundary = GRAPH_LEFT + GRAPH_WIDTH * (each - 1) / (each * 2);
    let white_rows = |pixels: &HeadlessCanvas| GRAPH_ROWS.filter(|row| pixels.pixel_at(boundary, *row) == SECTION_LINE).count();

    let (_, pixels) = listening_screen("sections", 2, false);
    assert!(white_rows(&pixels) >= MIN_LINE_ROWS, "no line marks the end of the first stage");

    let (_, pixels) = listening_screen("one-stage", 1, false);
    assert_eq!(white_rows(&pixels), 0, "a line was drawn in a course of one stage");
}

/// A confirming key tapped while the scene waits out its input time does nothing; the same key once
/// it listens starts the fade, and the screen leaves for the browser when the fade has run its time.
#[test]
fn a_confirming_key_fades_the_scene_out_to_the_browser() {
    let (mut app, run) = finish_course(skinned_app("confirm", SKIN), run_of(2, false));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    run_until_begun(&mut app, &mut pixels);

    for code in [KeyCode::Enter, KeyCode::Escape] {
        assert!(matches!(key(&mut app, press(code)), Transition::Stay), "{code:?} left the screen at once");
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
        key(&mut app, release(code));
    }
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "a key acted while the scene was not listening");

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));
    assert_eq!(pixels.pixel_at(LISTENING_MARK_AT.0, LISTENING_MARK_AT.1), LISTENING_MARK);
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "a key tapped during the wait was acted on afterwards");

    key(&mut app, press(KeyCode::Enter));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "the screen left without fading");
    assert_eq!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), FADING_MARK, "Enter did not start the fade");
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, FADEOUT_MS / 2), Transition::Stay), "the screen left before its fade was over");
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, FADEOUT_MS), Transition::Back), "the faded screen did not go back to the browser");
    assert!(app.shared.course_run.is_none(), "the course was not ended on the way out");
}

/// The scene leaves on its own once its length has passed, for the browser as well.
#[test]
fn a_scene_left_alone_fades_out_on_its_own() {
    const SCENE_MS: i64 = 3_600_000;
    let (mut app, run) = finish_course(skinned_app("alone", SKIN), run_of(2, false));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    run_until_begun(&mut app, &mut pixels);

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, SCENE_MS + 1), Transition::Stay));
    assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT));
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, FADEOUT_MS + 1), Transition::Back));
}

/// The sixth key turns the gauge by the course's own expression: from the gauge the last stage ended
/// on, a normal gauge, it goes to the first of the three course gauges, where a chart's result would
/// go to the next of its six.
#[test]
fn the_graph_key_turns_the_gauge_by_the_courses_expression() {
    let (mut app, mut pixels) = listening_screen("graph-key", 2, false);
    let shown = course_screen(&app).scene.gauge_type;
    assert_eq!(shown, record().finished_gauge.index(), "the graph opens on the gauge the last stage ended on");
    assert_eq!(shown, 2, "a run that chose nothing ends on the normal gauge");

    app.shared.note_key(&press(GRAPH_KEY));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    app.shared.note_key(&release(GRAPH_KEY));
    assert_eq!(course_screen(&app).scene.gauge_type, next_course_gauge_type(shown));
    assert_eq!(course_screen(&app).scene.gauge_type, 6);
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "turning the gauge closed the screen");
}

/// The reference's expression, `(g - 5) % 3 + 6` in signed arithmetic: the three course gauges turn
/// among themselves, and a chart's gauge is sent into the upper ones.
#[test]
fn the_courses_expression_is_the_references_signed_one() {
    assert_eq!([6, 7, 8].map(next_course_gauge_type), [7, 8, 6]);
    assert_eq!([0, 1, 2, 3, 4, 5].map(next_course_gauge_type), [4, 5, 6, 4, 5, 6], "(g - 5) % 3 + 6 with the sign of the dividend");
}

/// Nothing is offered to run again: a run-again key held through the fade and the R and N keys all
/// leave for the browser, as the reference's course result has no such branch.
#[test]
fn a_course_result_always_leaves_for_the_browser() {
    let (mut app, mut pixels) = listening_screen("browser-only", 2, false);
    key(&mut app, press(KeyCode::KeyR));
    key(&mut app, press(KeyCode::KeyN));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "R or N confirmed a course result");

    app.shared.note_key(&press(AGAIN_KEY));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT), "a lane key did not confirm");
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, FADEOUT_MS + 1), Transition::Back), "a run-again key held through the fade left for a run");
    assert!(app.shared.retry_seed.is_none());
}

/// A screen still waiting for its skin takes Escape alone, which leaves at once.
#[test]
fn escape_leaves_a_screen_still_waiting_for_its_skin() {
    let (mut app, run) = finish_course(skinned_app("waiting", SKIN), run_of(2, false));
    enter(&mut app, &run);
    assert_eq!(course_screen(&app).status, SkinStatus::Reading);

    assert!(matches!(key(&mut app, press(KeyCode::Enter)), Transition::Stay), "Enter acted before there was a scene");
    assert!(matches!(key(&mut app, press(KeyCode::Escape)), Transition::Back));
    assert!(app.shared.course_run.is_none());
}

/// A skin that turns out not to load hands the screen to the built-in layout.
#[test]
fn a_skin_that_cannot_be_read_leaves_the_built_in_screen() {
    const BROKEN: &str = "if skin_config then error(\"no such asset\") end\nreturn { type = 15, name = \"Broken\", w = 1280, h = 720 }\n";
    let (mut app, run) = finish_course(skinned_app("broken", BROKEN), run_of(2, false));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, &run);
    for _ in 0..SKIN_FRAMES {
        frame(&mut app, &mut pixels);
        if course_screen(&app).status == SkinStatus::Gone {
            break;
        }
    }
    assert_eq!(course_screen(&app).status, SkinStatus::Gone);
    assert!(course_screen(&app).announced, "the built-in screen did not raise its cue");
    assert!(matches!(key(&mut app, press(KeyCode::Enter)), Transition::Back), "the built-in screen does not act on a key at once");
}
