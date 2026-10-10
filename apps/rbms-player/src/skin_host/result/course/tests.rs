//! The course taken as one run: the judgements of the stages added up, the stages never reached
//! charged to a course that failed, and the gauge histories joined end to end.

use rbms_course::{Course, CourseChart, StageResult};
use rbms_play::{NullSink, PlaySession, SessionClock, SessionOptions};

use super::*;

/// The frame step a fixture run is driven at, and how long it is driven: past the last note and
/// the margin the reference keeps a chart playing after it.
const FRAME_US: i64 = 10_000;
const RUN_US: i64 = 9_000_000;

/// A chart of four notes on one key lane, 2.0 to 3.5 seconds in.
const FOUR_NOTES: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n";

/// How many notes [`FOUR_NOTES`] has, and how long its gauge history is once it has been played.
const FOUR_NOTES_COUNT: u32 = 4;

/// The stage a course that was never reached stands for: 120 notes, the last at four seconds.
const SKIPPED: UnplayedStage = UnplayedStage { notes: 120, last_note_ms: 4_000 };

/// An autoplayed run of [`FOUR_NOTES`], every note a PGREAT.
fn autoplayed_record() -> PlayRecord {
    let model = rbms_chart::to_model(&rbms_parser::parse(FOUR_NOTES), Mode::BEAT_7K);
    let mut session = PlaySession::new(model, SessionOptions { autoplay: true, ..SessionOptions::default() });
    let mut frame_us = 0;
    while frame_us <= RUN_US {
        session.tick(SessionClock::at(frame_us), &mut NullSink);
        frame_us += FRAME_US;
    }
    session.record()
}

fn course_of(stages: usize) -> Course {
    let mut course = Course {
        name: "TEST COURSE".to_string(),
        charts: (0..stages).map(|i| CourseChart { md5: format!("{i:032x}"), sha256: String::new(), title: format!("stage {i}") }).collect(),
        ..Course::default()
    };
    assert!(course.validate(), "the fixture course is valid");
    course
}

/// What an autoplayed stage hands the course: four PGREATs and an unbroken combo.
fn perfect_stage(record: &PlayRecord) -> StageResult {
    StageResult {
        ex_score: record.summary.ex_score,
        max_ex_score: record.summary.max_ex_score,
        notes: record.summary.total_notes,
        counts: record.summary.counts,
        max_combo: record.summary.max_combo,
        combo_at_end: record.summary.max_combo,
        gauge_value: 100.0,
        clear: clear_type_id(ClearType::Normal),
        survived: true,
        ..StageResult::default()
    }
}

fn cleared_course(stages: usize, record: &PlayRecord) -> CourseRun {
    let mut run = CourseRun::new(course_of(stages), 100.0);
    for stage in 1..=stages as u32 {
        let carried = record.summary.max_combo * stage;
        run.advance(&StageResult { max_combo: carried, combo_at_end: carried, ..perfect_stage(record) });
    }
    run
}

#[test]
fn the_stages_judgements_are_added_into_one_score() {
    let record = autoplayed_record();
    let run = cleared_course(2, &record);
    let tally = CourseTally::of(&run, &[record.clone(), record.clone()], &[], Mode::BEAT_7K);

    let sheet = &tally.input.sheet;
    assert_eq!(sheet.notes, FOUR_NOTES_COUNT * 2);
    assert_eq!(sheet.count(0), 8, "two stages of four PGREATs");
    assert_eq!(sheet.ex_score(), record.summary.ex_score * 2);
    assert_eq!((sheet.min_bp, sheet.max_combo, sheet.family), (0, 8, PointFamily::Beat7));
    assert_eq!(tally.notes, 8);
}

#[test]
fn the_courses_titles_are_its_charts_in_order() {
    let record = autoplayed_record();
    let tally = CourseTally::of(&cleared_course(2, &record), &[record.clone(), record], &[], Mode::BEAT_7K);

    assert_eq!(tally.input.course_titles, ["stage 0", "stage 1"]);
}

#[test]
fn the_stages_gauge_histories_are_joined_and_each_stages_end_is_marked() {
    let record = autoplayed_record();
    let each = record.gauge_log.len();
    assert!(each > 0, "the fixture run sampled its gauge");
    let tally = CourseTally::of(&cleared_course(2, &record), &[record.clone(), record.clone()], &[], Mode::BEAT_7K);

    assert_eq!(tally.gauges.len(), GAUGE_TYPES);
    assert!(tally.gauges.iter().all(|history| history.len() == each * 2));
    assert_eq!(tally.sections, [each, each * 2]);
    for index in GaugeIndex::ALL {
        let joined = &tally.gauges[index.index()];
        assert_eq!(&joined[..each], record.gauge_log.of(index));
        assert_eq!(&joined[each..], record.gauge_log.of(index));
    }
}

#[test]
fn a_stage_never_reached_is_flat_zeroes_as_long_as_the_stage_would_have_been() {
    assert_eq!(SKIPPED.gauge_samples(), 9, "(4000 + 500) / 500");
    assert_eq!(UnplayedStage::default().gauge_samples(), 1, "even an empty chart has the one sample");

    let record = autoplayed_record();
    let each = record.gauge_log.len();
    let mut run = CourseRun::new(course_of(2), 100.0);
    run.advance(&StageResult { survived: false, ..perfect_stage(&record) });
    let tally = CourseTally::of(&run, std::slice::from_ref(&record), &[SKIPPED], Mode::BEAT_7K);

    assert_eq!(tally.sections, [each, each + SKIPPED.gauge_samples()]);
    assert!(tally.gauges.iter().all(|history| history.len() == each + SKIPPED.gauge_samples()));
    assert!(tally.gauges.iter().all(|history| history[each..].iter().all(|value| *value == 0.0)));
}

/// The reference charges the stages a failed course never reached twice: once in
/// `MusicResult.updateScoreDatabase` and once more in `MusicResult.render`.
#[test]
fn a_failed_course_charges_the_notes_it_never_reached_as_bad_poor_twice_over() {
    let record = autoplayed_record();
    let mut run = CourseRun::new(course_of(2), 100.0);
    run.advance(&StageResult { survived: false, ..perfect_stage(&record) });
    let tally = CourseTally::of(&run, std::slice::from_ref(&record), &[SKIPPED], Mode::BEAT_7K);

    assert_eq!(tally.input.sheet.notes, FOUR_NOTES_COUNT + SKIPPED.notes, "the course's notes are every stage's");
    assert_eq!(tally.input.sheet.min_bp, record.min_bp_with_unreached() + SKIPPED.notes * 2);
    assert_eq!(tally.input.sheet.clear, clear_type_id(ClearType::Failed));
    assert_eq!(tally.input.course_clear, Some(clear_type_id(ClearType::Failed)));
}

#[test]
fn a_course_that_was_cleared_charges_nothing_for_stages_it_has_no_record_of() {
    let record = autoplayed_record();
    let run = cleared_course(1, &record);
    let tally = CourseTally::of(&run, std::slice::from_ref(&record), &[SKIPPED], Mode::BEAT_7K);

    assert_eq!(tally.input.sheet.min_bp, record.min_bp_with_unreached());
}

#[test]
fn the_graph_opens_on_the_gauge_the_last_stage_ended_on_and_reads_its_bounds() {
    let record = autoplayed_record();
    let tally = CourseTally::of(&cleared_course(1, &record), std::slice::from_ref(&record), &[], Mode::BEAT_7K);

    assert_eq!(tally.finished_gauge, record.finished_gauge.index());
    let bounds = record.gauge_bounds[record.finished_gauge.index()];
    assert_eq!(tally.scales[record.finished_gauge.index()], GaugeScale::new(bounds.min, bounds.max, bounds.border));
    assert_eq!(tally.input.gauge.value(tally.finished_gauge), record.gauge_log.last(record.finished_gauge), "the gauge number reads the last stage played");
}

/// A course whose combo ran unbroken through every note is a full combo, a perfect or a max by what
/// it was given besides PGREATs; one that broke it keeps the lamp of its gauge.
#[test]
fn an_unbroken_combo_earns_the_full_combo_family_by_its_judgements() {
    let record = autoplayed_record();
    let lamp = |counts: [u32; 6], max_combo: u32| {
        let mut run = CourseRun::new(course_of(1), 100.0);
        run.advance(&StageResult { counts, max_combo, notes: 4, ..perfect_stage(&record) });
        CourseTally::of(&run, std::slice::from_ref(&record), &[], Mode::BEAT_7K).input.sheet.clear
    };

    assert_eq!(lamp([4, 0, 0, 0, 0, 0], 4), clear_type_id(ClearType::Max));
    assert_eq!(lamp([3, 1, 0, 0, 0, 0], 4), clear_type_id(ClearType::Perfect));
    assert_eq!(lamp([2, 1, 1, 0, 0, 0], 4), clear_type_id(ClearType::FullCombo));
    assert_eq!(lamp([4, 0, 0, 0, 0, 0], 3), clear_type_id(lamp_for_gauge(record.finished_gauge)), "a broken combo keeps the gauge's lamp");
}

#[test]
fn an_assisted_course_keeps_its_assist_lamp() {
    let record = autoplayed_record();
    let mut run = CourseRun::new(course_of(1), 100.0);
    run.advance(&StageResult { clear: clear_type_id(ClearType::AssistEasy), ..perfect_stage(&record) });

    assert_eq!(CourseTally::of(&run, &[record], &[], Mode::BEAT_7K).input.sheet.clear, clear_type_id(ClearType::AssistEasy));
}

#[test]
fn a_course_with_no_record_reads_as_a_course_of_nothing() {
    let run = CourseRun::new(course_of(2), 100.0);
    let tally = CourseTally::of(&run, &[], &[], Mode::BEAT_7K);

    assert_eq!((tally.notes, tally.input.sheet.min_bp), (0, 0));
    assert!(tally.gauges.iter().all(Vec::is_empty) && tally.sections.is_empty());
    assert_eq!(tally.scales, [UNKNOWN_SCALE; GAUGE_TYPES]);
    assert_eq!(tally.input.gauge.value(tally.finished_gauge), None);
}
