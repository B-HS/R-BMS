//! The snapshot of a finished run: what it takes from a record, what it makes of the best the
//! player had, and what the screen may change in it afterwards.

use rbms_judge::ClearType;
use rbms_judge::gauge::GaugeIndex;
use rbms_model::Mode;
use rbms_play::{NullSink, PlayRecord, PlaySession, SessionClock, SessionOptions};
use rbms_render::result::TargetView;
use rbms_store::{ScoreBook, ScoreRecord};

use super::*;

/// The frame step the fixture run is driven at, and how long it is driven: past the last note and
/// the margin the reference keeps a chart playing after it.
const FRAME_US: i64 = 10_000;
const RUN_US: i64 = 9_000_000;

/// A chart of four notes on one key lane, 2.0 to 3.5 seconds in.
const FOUR_NOTES: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n";

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

#[test]
fn a_record_gives_its_sheet_its_gauges_and_its_timing() {
    let record = autoplayed_record();
    let input = ResultInput::from_record(&record, Mode::BEAT_7K, ClearType::Normal);

    assert_eq!(input.sheet.notes, 4);
    assert_eq!((input.sheet.count(0), input.sheet.ex_score(), input.sheet.max_combo), (4, record.summary.ex_score, 4));
    assert_eq!((input.sheet.min_bp, input.sheet.clear, input.sheet.family), (0, 5, PointFamily::Beat7));
    assert!(input.gauge.ends.iter().all(Option::is_some), "all nine gauges have a history to end on");
    assert_eq!((input.timing.average_ms, input.timing.std_dev_ms), (record.timing.average(), record.timing.std_dev()));
    assert_eq!(input.timing.avg_duration_us, record.avg_duration_us);
    assert_eq!(input.timing.center_ms, 150);
    assert!(input.updates_score && input.target.is_none() && input.course_titles.is_empty());
}

#[test]
fn a_snapshot_holds_the_run_against_the_best_and_the_target_it_was_given() {
    let record = autoplayed_record();
    let snapshot = ResultSnapshot::of(ResultInput {
        previous: PreviousScore { ex_score: 6, max_combo: 3, min_bp: Some(2), clear: 4 },
        target: Some(TargetView { name: "RIVAL".to_string(), ex: 7 }),
        course_titles: vec!["one".to_string()],
        ..ResultInput::from_record(&record, Mode::BEAT_7K, ClearType::Normal)
    });

    let standing = &snapshot.score.standing;
    assert_eq!((standing.now_ex, standing.best_score, standing.rival_score), (8, 6, 7));
    assert_eq!(standing.rate, 1.0, "a run is rated over the chart's notes, not the notes of the best");
    assert_eq!(snapshot.target_name, "RIVAL");
    assert_eq!(snapshot.course_titles, ["one"]);
}

#[test]
fn a_snapshot_without_a_target_or_history_is_held_against_nothing() {
    let snapshot = ResultSnapshot::of(ResultInput::from_record(&autoplayed_record(), Mode::BEAT_7K, ClearType::Normal));

    let standing = &snapshot.score.standing;
    assert_eq!((standing.best_score, standing.rival_score, standing.now_best_score), (0, 0, 0));
    assert_eq!(snapshot.target_name, "");
    assert_eq!(snapshot.previous, PreviousScore::default());
    assert_eq!(snapshot.previous.min_bp, None, "a chart never played has the worst possible bad-poor count");
}

#[test]
fn a_run_clears_unless_it_or_its_course_has_failed() {
    let mut input = ResultInput::from_record(&autoplayed_record(), Mode::BEAT_7K, ClearType::Normal);
    assert!(ResultSnapshot::of(input.clone()).cleared());

    input.course_clear = Some(rbms_judge::clear_type_id(ClearType::Normal));
    assert!(ResultSnapshot::of(input.clone()).cleared(), "a course that is still alive does not fail the stage");

    input.course_clear = Some(rbms_judge::clear_type_id(ClearType::Failed));
    assert!(!ResultSnapshot::of(input.clone()).cleared(), "a stage in a failed course fails with it");

    input.course_clear = None;
    input.sheet.clear = rbms_judge::clear_type_id(ClearType::Failed);
    assert!(!ResultSnapshot::of(input).cleared());
}

#[test]
fn the_gauge_number_reads_the_end_of_the_gauge_the_screen_has_the_graph_on() {
    let mut ends = [None; GaugeIndex::COUNT];
    ends[GaugeIndex::Normal.index()] = Some(72.5);
    ends[GaugeIndex::Hard.index()] = Some(0.0);
    let snapshot =
        ResultSnapshot::of(ResultInput { gauge: GaugeEnds { ends }, ..ResultInput::from_record(&autoplayed_record(), Mode::BEAT_7K, ClearType::Normal) });
    let on = |gauge_type: usize| FinishedRun::new(&snapshot, ResultScene { gauge_type, ..ResultScene::default() }).gauge_reading();

    let normal = on(GaugeIndex::Normal.index()).expect("the normal gauge has a history");
    assert_eq!((normal.value, normal.kind, normal.live), (72.5, 2, false));
    assert_eq!(on(GaugeIndex::Hard.index()).map(|reading| reading.value), Some(0.0), "a failed gauge ends on nothing, which is a value");
    assert_eq!(on(GaugeIndex::Easy.index()), None, "a gauge with no history has nothing to read");
    assert_eq!(on(GaugeIndex::COUNT), None, "and a gauge that does not exist has none either");
}

/// A stored run of a chart, with the figures the previous best is made of.
fn stored(md5: &str, ex_score: u32, max_combo: u32, counts: [u32; 6], clear: u8, assisted: bool, ln_mode: &str) -> ScoreRecord {
    ScoreRecord {
        md5: md5.to_string(),
        title: "chart".to_string(),
        mode: "BEAT_7K".to_string(),
        clear,
        ex_score,
        max_ex: 20,
        counts,
        empty_poor: 0,
        max_combo,
        total_notes: 10,
        gauge: "NORMAL".to_string(),
        gauge_value: 80.0,
        random: "OFF".to_string(),
        played_at: 0,
        replay_file: None,
        rule_version: rbms_store::SCORE_RULE_VERSION,
        ln_mode: ln_mode.to_string(),
        assisted,
    }
}

#[test]
fn the_best_is_the_best_of_each_figure_among_the_runs_that_may_set_one() {
    let mut book = ScoreBook::default();
    book.push(stored("abc", 11, 6, [5, 1, 0, 2, 1, 1], 4, false, "CHART"));
    book.push(stored("abc", 9, 9, [4, 1, 0, 0, 1, 0], 5, false, "CHART"));
    book.push(stored("abc", 19, 10, [9, 1, 0, 0, 0, 0], 8, true, "CHART"));
    book.push(stored("abc", 17, 10, [8, 1, 0, 0, 0, 0], 9, false, "LN"));
    book.push(stored("other", 20, 10, [10, 0, 0, 0, 0, 0], 10, false, "CHART"));

    let previous = PreviousScore::of_book(&book, "abc", "CHART");

    assert_eq!(previous.ex_score, 11, "an assisted run and one under another LN MODE set no best");
    assert_eq!(previous.max_combo, 9);
    assert_eq!(previous.min_bp, Some(1), "bad, poor and miss together: 4, 1 and 0");
    assert_eq!(previous.clear, 8, "an assisted run still raises the lamp");
}

#[test]
fn a_chart_never_played_has_no_best() {
    assert_eq!(PreviousScore::of_book(&ScoreBook::default(), "abc", "CHART"), PreviousScore::default());
}
