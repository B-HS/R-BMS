//! How a course stage's gauge and long notes are set up, checked on stages set up the way the chart
//! load sets one up ([`AppShared::stage_judge_setup`]) and handed to the app the way the play screen
//! hands one over: the table the COURSE GAUGE setting picks, the course gauge the GAUGE setting maps
//! to, every gauge carried from the stage before, and the long-note flavour the course forces.

use rbms_judge::GaugeKind;
use rbms_judge::gauge::GaugeAutoShift;
use rbms_judge::gauge_tables::{CourseGaugeMode, GaugeSetId};
use rbms_judge::ln::LnMode;

use super::*;

/// The library a course of the recorded chart is played from: the one chart, so every stage of it
/// resolves.
fn library_of_the_recorded_chart() -> Library {
    let model = recorded_model();
    Library::from_songs(vec![rbms_library::SongEntry {
        path: PathBuf::from(RECORDED_CHART_NAME),
        title: model.meta.title.clone(),
        subtitle: String::new(),
        artist: model.meta.artist.clone(),
        genre: String::new(),
        maker: String::new(),
        level: model.meta.play_level.clone(),
        difficulty: 0,
        init_bpm: model.init_bpm,
        rank: 2,
        total: model.meta.total,
        mode: model.mode,
        md5: model.md5.clone(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }])
}

/// Begin the recorded chart's course of `stages` stages under `constraints` from the browser, as the
/// player starts one.
fn course_started(shared: &mut AppShared, stages: usize, constraints: Vec<rbms_course::CourseConstraint>) {
    shared.library = library_of_the_recorded_chart();
    let mut course = one_stage_course(recorded_model().md5).course;
    course.charts = course.charts.iter().cycle().take(stages).cloned().collect();
    course.constraints = constraints;
    let entry = crate::course_ui::CourseEntry { course, missing: Vec::new() };
    crate::start_course(shared, &entry);
    assert!(shared.course_run.is_some(), "the course started");
}

/// The recorded chart set up as the chart load sets up the stage the app stands on: judged as
/// [`AppShared::stage_judge_setup`] says, and starting on the course gauge, the carried gauges and
/// the carried combo it answers with.
fn loaded_stage(shared: &AppShared) -> PlaySession {
    let model = recorded_model();
    let (judge_setup, course_stage) = shared.stage_judge_setup(&model.mode);
    let options = SessionOptions {
        gauge: shared.config.play.gauge,
        initial_gauge: course_stage.map(|stage| (stage.start, stage.value)),
        initial_combo: course_stage.and(shared.course_run.as_ref()).map_or(0, |run| run.totals.combo_carry),
        carried_gauges: course_stage.and_then(|stage| stage.carried),
        ..SessionOptions::default()
    };
    let mut session = PlaySession::new(model, options);
    session.set_judge_setup(judge_setup);
    session
}

/// [`loaded_stage`] played by hand to its end.
fn played_stage(shared: &AppShared) -> PlaySession {
    replay_inputs(loaded_stage(shared), &inputs_by_hand(&recorded_model(), &HAND_OFFSETS_US, false))
}

/// An LN course plays its long notes as LN whatever LN MODE says, and reports them so, while the
/// setting stays the player's: it is what the settings file holds during the course and after it.
#[test]
fn an_ln_course_plays_long_notes_whatever_the_ln_mode_setting() {
    let mut shared = signed_out("course-ln-forced");
    shared.config.judge.ln_mode = LnMode::ChargeNote;
    course_started(&mut shared, 1, vec![rbms_course::CourseConstraint::Ln]);

    let (judge_setup, _) = shared.stage_judge_setup(&recorded_model().mode);
    assert_eq!(judge_setup.ln_mode, LnMode::LongNote, "the stage is judged on the course's flavour");
    assert_eq!(loaded_stage(&shared).ln_mode(), LnMode::LongNote);
    assert_eq!(crate::judge_setup::run_lntype(0, judge_setup.ln_mode), LnMode::LongNote.id() as i32, "the stage reports LN");
    assert_eq!(shared.settings_to_save().judge.ln_mode, LnMode::ChargeNote, "the settings file keeps the player's LN MODE");

    crate::end_course(&mut shared);
    assert_eq!(shared.config.judge.ln_mode, LnMode::ChargeNote);
    let (single_chart, _) = shared.stage_judge_setup(&recorded_model().mode);
    assert_eq!(single_chart.ln_mode, LnMode::ChargeNote, "a chart played after the course is on the setting again");
}

/// A course stage plays on the table the COURSE GAUGE setting picks, ahead of the course's own gauge
/// constraint, while a single chart keeps the table the mode picks.
#[test]
fn a_stage_on_iidx_rule_plays_on_the_iidx_table() {
    let mut shared = signed_out("course-iidx-table");
    shared.config.judge.course_gauge = CourseGaugeMode::Iidx;
    assert_eq!(loaded_stage(&shared).gauge_set(), GaugeSetId::SevenKeys, "a single chart is not a course stage");
    course_started(&mut shared, 1, vec![rbms_course::CourseConstraint::GaugeLr2]);
    let stage = loaded_stage(&shared);
    assert_eq!(stage.gauge_set(), GaugeSetId::Iidx);
    assert_eq!(stage.judge().gauge.selected_index(), GaugeIndex::Class);
}

/// The stage after one that moved its selection with BEST CLEAR starts every one of its gauges where
/// the stage before left it, not only the one the stage ended on.
#[test]
fn the_next_stage_starts_on_every_gauge_the_last_one_left() {
    let mut shared = signed_out("course-carry-every-gauge");
    shared.config.judge.gauge_auto_shift = GaugeAutoShift::BestClear;
    course_started(&mut shared, 2, Vec::new());

    let first = played_stage(&shared);
    let finished_on = first.record().finished_gauge;
    assert_ne!(finished_on, GaugeIndex::Class, "the fixture stage ends on a stronger course gauge than it started on");
    let left = first.gauge_values();
    assert_ne!(left[GaugeIndex::Class.index()], left[finished_on.index()], "the fixture stage leaves the gauges apart");
    let (entered, _) = end_run(&mut shared, first);
    assert!(matches!(entered, Transition::To(Stage::Loading(_))), "the course goes on to its second stage");

    let second = loaded_stage(&shared);
    assert_eq!(second.gauge_values(), left, "every gauge starts where the first stage left it");
    assert_eq!(second.judge().gauge.selected_index(), GaugeIndex::Class, "the stage starts on the course gauge the setting maps to");
}

/// A player whose GAUGE setting is HARD plays a course on EXCLASS, and each stage is stored and sent
/// on it (`GrooveGauge.java:164`).
#[test]
fn a_hard_player_records_course_stages_on_exclass() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("course-hard-exclass", server.clone());
    shared.config.play.gauge = GaugeKind::Hard;
    course_started(&mut shared, 1, Vec::new());

    let stage = played_stage(&shared);
    assert_eq!(stage.record().finished_gauge, GaugeIndex::ExClass, "the stage ran on EXCLASS");
    end_run(&mut shared, stage);
    shared.submit_rx.take().expect("the stage was handed to the score server").recv().expect("the submission came back");
    assert_eq!(server.gauges.lock().expect("the gauge log").as_slice(), [rbms_ir::GaugeType::ExClass], "the stage is sent on EXCLASS");
    let (logs, _) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one stage was played and {} rows were stored", logs.len());
    };
    assert_eq!(log.gauge, rbms_store::scoredb::gauge_id("exclass"), "the row names EXCLASS");
}

/// A GRADE MIRROR course started from the browser plays its stages on MIRROR or as written and on
/// nothing else, which is the arrangement its stages are loaded and reported with, and the player's
/// own arrangement is back once it ends (`MusicSelector.java:483-491`).
#[test]
fn a_grade_mirror_course_plays_its_stages_on_mirror_or_as_written() {
    for chosen in rbms_chart::shuffle::NoteOption::ALL {
        let mut shared = signed_out(&format!("course-grade-mirror-{chosen:?}"));
        shared.config.play.random = chosen;
        course_started(&mut shared, 1, vec![rbms_course::CourseConstraint::Mirror]);
        let played_on = if chosen == rbms_chart::shuffle::NoteOption::Mirror { chosen } else { rbms_chart::shuffle::NoteOption::Off };
        assert_eq!(shared.config.play.random, played_on, "{chosen:?}");
        crate::end_course(&mut shared);
        assert_eq!(shared.config.play.random, chosen, "{chosen:?} is the player's again");
    }
}
