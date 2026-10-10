//! The trace of a run against a real judge engine: what it reports as the engine judges, and what
//! a skin's note field and judge object read back from it.

use rbms_judge::{Judge, JudgeEngine};
use rbms_model::Model;
use rbms_render::skin_render::frame::{JudgeHit, NOTE_UNJUDGED, NoteStates};

use super::*;

/// A seven-key chart with two notes in its first key, one beat apart, and one in its second.
const CHART: &str = "#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n#00112:01\n";

/// A chart with one long note in its first key, written as a hell charge note.
const HELL_CHART: &str = "#PLAYER 1\n#BPM 120\n#LNTYPE 1\n#LNMODE 3\n#WAV01 a.wav\n#00151:0101\n";

/// The same long note written as a plain one.
const LONG_CHART: &str = "#PLAYER 1\n#BPM 120\n#LNTYPE 1\n#LNMODE 1\n#WAV01 a.wav\n#00151:0101\n";

/// Far enough past a note for the engine to count it as missed.
const WELL_PAST_US: i64 = 2_000_000;

/// The state a note that took a perfect great and one that went by unhit are left in.
const PERFECT_STATE: u8 = 1;
const MISSED_STATE: u8 = 5;

fn model_of(chart: &str) -> Model {
    let src = rbms_parser::parse_with(chart.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "trace.bms");
    let mut model = rbms_chart::to_model(&src, mode);
    rbms_chart::resolve_long_note_flavour(&mut model, rbms_model::LnKind::Ln);
    model
}

/// The chart times of the notes in `lane`, in order.
fn note_times(engine: &JudgeEngine, lane: usize) -> Vec<i64> {
    engine.note_marks().filter(|mark| mark.lane == lane && !mark.long_end).map(|mark| mark.time_us).collect()
}

/// The first lane of `engine` that has a note.
fn first_lane(engine: &JudgeEngine) -> usize {
    engine.note_marks().map(|mark| mark.lane).min().expect("the chart has a note")
}

/// A trace that has taken its first look at `engine`.
fn looking_at(engine: &JudgeEngine) -> RunTrace {
    let mut trace = RunTrace::new();
    assert!(trace.take_judgements(engine).is_empty(), "the first look at a run reported a judgement");
    trace
}

/// The first look takes the states in and reports nothing, even of a run that is already under way.
#[test]
fn the_first_look_at_a_run_reports_nothing_it_did_before() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lane = first_lane(&engine);
    let first = note_times(&engine, lane)[0];
    engine.press(lane, first);

    let mut trace = RunTrace::new();
    assert!(trace.take_judgements(&engine).is_empty());
    assert_eq!(trace.state(lane, first), PERFECT_STATE, "the state of a note judged before the first look was not taken in");
    assert!(trace.take_judgements(&engine).is_empty(), "a judgement that was already there was reported later");
}

/// A note the player hits is reported once, in its own lane, with the judgement and the timing it
/// took, and the note field reads the note as judged from then on.
#[test]
fn a_note_that_is_hit_is_reported_once_with_its_lane_and_its_timing() {
    const EARLY_US: i64 = 3_000;
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lane = first_lane(&engine);
    let times = note_times(&engine, lane);
    let mut trace = looking_at(&engine);
    assert_eq!(trace.state(lane, times[0]), NOTE_UNJUDGED);

    engine.press(lane, times[0] - EARLY_US);
    assert_eq!(trace.take_judgements(&engine), vec![Seen { lane, judge: Judge::PerfectGreat, fast_us: EARLY_US, combo: 1 }]);
    assert_eq!(trace.state(lane, times[0]), PERFECT_STATE);
    assert_eq!(trace.state(lane, times[1]), NOTE_UNJUDGED, "the next note of the lane was read as judged");
    assert!(trace.take_judgements(&engine).is_empty(), "one judgement was reported twice");
}

/// A note nobody hits is reported when the engine counts it as missed, though no input ever named
/// it: that is the judgement a skin's timers would otherwise never hear of.
#[test]
fn a_note_that_goes_by_unhit_is_reported_as_a_poor_in_its_lane() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lanes: Vec<usize> = engine.note_marks().map(|mark| mark.lane).collect();
    let last = engine.note_marks().map(|mark| mark.time_us).max().expect("the chart has a note");
    let mut trace = looking_at(&engine);

    engine.update(last + WELL_PAST_US);
    let seen = trace.take_judgements(&engine);
    assert_eq!(seen.len(), lanes.len(), "not every missed note was reported: {seen:?}");
    assert!(seen.iter().all(|seen| seen.judge == Judge::Poor && seen.combo == 0));
    assert_eq!(seen.iter().map(|seen| seen.lane).collect::<Vec<_>>(), lanes, "the misses are not reported in their own lanes, as the engine counted them");
    assert!(lanes.iter().all(|lane| note_times(&engine, *lane).iter().all(|time_us| trace.state(*lane, *time_us) == MISSED_STATE)));
}

/// A press that takes no note leaves no note behind to say where it was, and is still reported in
/// the lane that was pressed: the engine says which.
#[test]
fn a_press_that_takes_no_note_is_reported_in_the_lane_that_was_pressed() {
    const JUST_OUTSIDE_US: i64 = 400_000;
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lane = first_lane(&engine) + 1;
    let first = note_times(&engine, lane)[0];
    let mut trace = looking_at(&engine);

    let before = engine.counts[Judge::Miss as usize];
    engine.press(lane, first - JUST_OUTSIDE_US);
    if engine.counts[Judge::Miss as usize] == before {
        return;
    }
    let seen = trace.take_judgements(&engine);
    assert_eq!(seen.iter().map(|seen| (seen.lane, seen.judge)).collect::<Vec<_>>(), [(lane, Judge::Miss)]);
    assert_eq!(trace.state(lane, first), NOTE_UNJUDGED, "an empty POOR consumed the note it was pressed near");
}

/// A trace that is reset has seen nothing: its next look takes the run in afresh.
#[test]
fn a_reset_trace_takes_the_run_in_afresh_and_shows_no_judgement() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lane = first_lane(&engine);
    let first = note_times(&engine, lane)[0];
    let mut trace = looking_at(&engine);
    engine.press(lane, first);
    let seen = trace.take_judgements(&engine);
    trace.note_judgement(seen[0], 8, 1, 0);
    assert!(trace.regions().region(0).is_some());

    trace.reset();
    assert!(trace.regions().region(0).is_none(), "a judgement stayed on show after the run was put elsewhere");
    assert_eq!(trace.sides(), [None; PLAYER_SIDES]);
    assert!(trace.take_judgements(&engine).is_empty(), "the look after a reset reported what the run had done before");
    assert_eq!(trace.state(lane, first), PERFECT_STATE);
}

/// A lane's key going down is told from one that is still down by when the engine lit its beam.
#[test]
fn a_key_is_reported_as_pressed_when_its_beam_is_lit_anew() {
    let mut trace = RunTrace::new();
    assert!(trace.take_presses(&[i64::MIN, i64::MIN]).is_empty(), "a lane that is up was reported as pressed");
    assert_eq!(trace.take_presses(&[i64::MIN, 1_000]), vec![1]);
    assert!(trace.take_presses(&[i64::MIN, 1_000]).is_empty(), "a key that is still down was pressed again");
    assert_eq!(trace.take_presses(&[2_000, 3_000]), vec![0, 1], "a key pressed again while down, and a new one");
    assert!(trace.take_presses(&[i64::MIN, 3_000]).is_empty(), "a key coming up was reported as a press");
}

/// A judgement is noted in the region its lane belongs to: one region takes every lane, and two
/// split the lanes between the two fields. A skin with no judge object notes nothing.
#[test]
fn a_judgement_is_noted_in_the_region_of_its_lane() {
    const DOUBLE_LANES: usize = 16;
    const LATE_US: i64 = -12_700;
    let late = Seen { lane: 12, judge: Judge::Great, fast_us: LATE_US, combo: 7 };

    let mut single = RunTrace::new();
    single.note_judgement(late, DOUBLE_LANES, 1, 5_000);
    assert_eq!(single.regions().region(0), Some(JudgeHit { judgement: 1, combo: 7, at_us: 5_000 }));
    assert_eq!(single.sides()[0], Some(ShownJudgement { judge: 1, timing_ms: -12 }), "the timing is cut towards zero to whole milliseconds");

    let mut double = RunTrace::new();
    double.note_judgement(late, DOUBLE_LANES, 2, 5_000);
    assert!(double.regions().region(0).is_none(), "a judgement in the right-hand field was shown in the left");
    assert!(double.regions().region(1).is_some());
    assert_eq!(double.sides()[1].map(|side| side.judge), Some(1));

    let mut none = RunTrace::new();
    none.note_judgement(late, DOUBLE_LANES, 0, 5_000);
    assert_eq!(none.sides(), [None; PLAYER_SIDES], "a skin with no judge object has no region to show a judgement in");
}

/// Two judgements counted between two looks are reported in the order they were counted, each with
/// the combo it left, so the two fields of a double chart do not show the same count for a chord.
#[test]
fn the_judgements_of_one_frame_keep_their_order_and_their_own_combos() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lane = first_lane(&engine);
    let other = lane + 1;
    let (first, chord) = (note_times(&engine, lane)[0], note_times(&engine, other)[0]);
    let mut trace = looking_at(&engine);

    engine.press(other, chord);
    engine.press(lane, first);
    let seen = trace.take_judgements(&engine);
    assert_eq!(seen.iter().map(|seen| (seen.lane, seen.combo)).collect::<Vec<_>>(), [(other, 1), (lane, 2)]);

    const DOUBLE_LANES: usize = 2;
    for judgement in seen {
        trace.note_judgement(Seen { lane: judgement.lane - lane, ..judgement }, DOUBLE_LANES, DOUBLE_LANES, 0);
    }
    let shown = |region: i32| trace.regions().region(region).map(|hit| hit.combo);
    assert_eq!((shown(0), shown(1)), (Some(2), Some(1)), "the two regions show the combo of the frame rather than their own");
}

/// The combo of a chart alone leaves out what a course carried into it, until the combo breaks in
/// the chart: from then on the two are the same count.
#[test]
fn the_combo_of_a_chart_alone_leaves_out_what_a_course_carried_in() {
    const CARRIED: u32 = 120;
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    engine.combo = CARRIED;
    let lane = first_lane(&engine);
    let times = note_times(&engine, lane);
    let mut trace = looking_at(&engine);
    assert_eq!(trace.stage_combo(engine.combo, CARRIED), 0);

    engine.press(lane, times[0]);
    trace.take_judgements(&engine);
    assert_eq!((engine.combo, trace.stage_combo(engine.combo, CARRIED)), (CARRIED + 1, 1));

    engine.update(times[1] + WELL_PAST_US);
    trace.take_judgements(&engine);
    assert_eq!(trace.stage_combo(engine.combo, CARRIED), 0, "a broken combo is nothing");
    assert_eq!(trace.stage_combo(CARRIED - 1, CARRIED), CARRIED - 1, "a combo counted after the break still had the carried count taken off");

    let mut joined = RunTrace::new();
    joined.take_judgements(&engine);
    assert_eq!(joined.stage_combo(3, CARRIED), 3, "a trace that joins a run after its combo broke takes the carried count off");
}

/// The one long note of a chart, as the play screen keeps it.
fn span_of(engine: &JudgeEngine) -> (usize, LongSpan) {
    let head = engine.note_marks().find(|mark| !mark.long_end).expect("the chart has a long note");
    let end = engine.note_marks().find(|mark| mark.long_end).expect("the long note has an end");
    (head.lane, LongSpan { head_us: head.time_us, end_us: end.time_us })
}

/// How a lane stands with the long notes the engine says it has in hand, on a run played by hand.
fn hand_of(engine: &JudgeEngine, lane: usize, down: bool) -> LaneHand {
    LaneHand { held: engine.long_note(lane), down, autoplay: false }
}

/// A long note is in hand from the press that takes its head, which may come ahead of the head's
/// own time, and until the note is resolved. A lane that is merely down holds nothing.
#[test]
fn a_long_note_is_in_hand_from_the_press_that_takes_it() {
    const EARLY_US: i64 = 15_000;
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(LONG_CHART));
    let (lane, span) = span_of(&engine);
    let trace = looking_at(&engine);
    assert_eq!(lane_long(&[span], &trace, lane, hand_of(&engine, lane, true)), LaneLong::default(), "a lane that is down with no note taken holds one");

    engine.press(lane, span.head_us - EARLY_US);
    engine.update(span.head_us - EARLY_US);
    assert_eq!(lane_long(&[span], &trace, lane, hand_of(&engine, lane, true)).processing, Some(span.head_us), "a note taken ahead of its head is not held yet");

    engine.release(lane, span.end_us);
    assert_eq!(lane_long(&[span], &trace, lane, hand_of(&engine, lane, false)), LaneLong::default(), "a note that is over is still in hand");
}

/// A long note the engine has counted as missed is not in hand, however long the lane stays down.
#[test]
fn a_long_note_that_was_missed_is_not_in_hand() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(LONG_CHART));
    let (lane, span) = span_of(&engine);
    let mut trace = looking_at(&engine);
    engine.update(span.end_us + WELL_PAST_US);
    trace.take_judgements(&engine);
    assert_eq!(lane_long(&[span], &trace, lane, hand_of(&engine, lane, true)).processing, None);
}

/// A hell charge note the play head is inside is going by whether or not it was hit, and reads as
/// not judged until something judges it, which is what keeps both of its timers off till then.
#[test]
fn a_hell_charge_note_goes_by_unjudged_until_something_judges_it() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(HELL_CHART));
    let (lane, span) = span_of(&engine);
    let mut trace = looking_at(&engine);
    let middle = (span.head_us + span.end_us) / 2;
    assert_eq!(lane_long(&[span], &trace, lane, hand_of(&engine, lane, false)).passing, None, "a note the play head has not reached is going by");

    engine.update(span.head_us + 1);
    trace.take_judgements(&engine);
    let untouched = lane_long(&[span], &trace, lane, hand_of(&engine, lane, false));
    assert_eq!((untouched.processing, untouched.passing, untouched.increasing), (None, Some(span.head_us), false));
    assert!(!trace.is_judged(lane, span.head_us), "a note nothing has judged reads as judged");

    engine.update(middle);
    trace.take_judgements(&engine);
    assert!(trace.is_judged(lane, span.head_us), "a note the engine counted as missed still reads as not judged");
    let missed = lane_long(&[span], &trace, lane, hand_of(&engine, lane, false));
    assert_eq!((missed.passing, missed.increasing), (Some(span.head_us), false), "a missed note that nobody holds is gaining");
}

/// A hell charge note going by gains while its lane is down, while the game plays the chart by
/// itself, and once its end has been taken, and drains otherwise.
#[test]
fn a_hell_charge_note_gains_while_it_is_held_or_its_end_was_taken() {
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(HELL_CHART));
    let (lane, span) = span_of(&engine);
    let mut trace = looking_at(&engine);
    let middle = (span.head_us + span.end_us) / 2;
    engine.press(lane, span.head_us);
    engine.update(middle);
    trace.take_judgements(&engine);

    let held = lane_long(&[span], &trace, lane, hand_of(&engine, lane, true));
    assert_eq!((held.processing, held.passing, held.increasing), (Some(span.head_us), Some(span.head_us), true));
    assert!(!lane_long(&[span], &trace, lane, hand_of(&engine, lane, false)).increasing, "a note nobody holds is gaining");
    assert!(lane_long(&[span], &trace, lane, LaneHand { autoplay: true, ..hand_of(&engine, lane, false) }).increasing, "a note the game plays is draining");

    const EARLY_US: i64 = 10_000;
    engine.release(lane, span.end_us - EARLY_US);
    engine.update(span.end_us - EARLY_US);
    trace.take_judgements(&engine);
    let let_go = lane_long(&[span], &trace, lane, hand_of(&engine, lane, false));
    assert_eq!(let_go.processing, None);
    assert_eq!((let_go.passing, let_go.increasing), (Some(span.head_us), true), "a note whose end was taken cleanly drains until the play head leaves it");
}

/// The second the fixture chart's first notes fall in, and the second its last one does.
const FIRST_NOTES_SECOND: usize = 2;
const LAST_NOTE_SECOND: usize = 3;

/// The class of the tables a note that has not been judged is counted in.
const UNJUDGED_CLASS: usize = 0;

/// How far past its early class a judgement's late class is in the early and late table.
const LATE_CLASS_AFTER: usize = 4;

/// The tables of the chart's seconds start with every note counted as not judged, in the second its
/// own time falls in, and follow each note as it is judged: under its judgement in one table, and
/// under its judgement and its side in the other. A perfect great has no side.
#[test]
fn the_seconds_of_the_chart_follow_the_notes_as_they_are_judged() {
    const EARLY_US: i64 = 3_000;
    const LATE_US: i64 = 40_000;
    let mut engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let lane = first_lane(&engine);
    let first = note_times(&engine, lane)[0];
    let mut trace = looking_at(&engine);
    {
        let (by_judge, by_timing) = trace.seconds();
        assert_eq!((by_judge[FIRST_NOTES_SECOND][UNJUDGED_CLASS], by_judge[LAST_NOTE_SECOND][UNJUDGED_CLASS]), (2, 1));
        assert_eq!(by_timing[FIRST_NOTES_SECOND][UNJUDGED_CLASS], 2);
        assert_eq!(by_judge.len(), LAST_NOTE_SECOND + 1, "the tables end with the second of the last note until they are told the chart runs on");
    }

    engine.press(lane, first - EARLY_US);
    trace.take_judgements(&engine);
    let (by_judge, by_timing) = trace.seconds();
    let perfect = usize::from(PERFECT_STATE);
    assert_eq!((by_judge[FIRST_NOTES_SECOND][UNJUDGED_CLASS], by_judge[FIRST_NOTES_SECOND][perfect]), (1, 1));
    assert_eq!(by_timing[FIRST_NOTES_SECOND][perfect], 1);

    let other = lane + 1;
    let hit = engine.press(other, note_times(&engine, other)[0] + LATE_US).expect("a press forty milliseconds late takes its note");
    let judged = hit.judge as usize + 1;
    assert!(judged > perfect, "forty milliseconds late is not a perfect great");
    trace.take_judgements(&engine);
    let (by_judge, by_timing) = trace.seconds();
    assert_eq!(by_judge[FIRST_NOTES_SECOND], {
        let mut row = [0; 6];
        row[perfect] = 1;
        row[judged] = 1;
        row
    });
    assert_eq!(by_timing[FIRST_NOTES_SECOND][judged + LATE_CLASS_AFTER], 1, "a late judgement is not counted on the late side");
    assert_eq!(by_timing[FIRST_NOTES_SECOND][judged], 0);
    assert_eq!(by_judge[LAST_NOTE_SECOND][UNJUDGED_CLASS], 1, "a note of another second moved");
}

/// The tables are given a row for every second of a chart that runs on past its last note, and keep
/// their length when the run is put somewhere else in the chart.
#[test]
fn the_tables_reach_the_end_of_the_chart_and_keep_their_length_over_a_reset() {
    const CHART_SECONDS: usize = 9;
    let engine = JudgeEngine::from_model_for_mode(&model_of(CHART));
    let mut trace = looking_at(&engine);
    trace.reach(CHART_SECONDS);
    assert_eq!(trace.seconds().0.len(), CHART_SECONDS);
    assert_eq!(trace.seconds().1.len(), CHART_SECONDS);

    trace.reset();
    trace.take_judgements(&engine);
    let (by_judge, _) = trace.seconds();
    assert_eq!(by_judge.len(), CHART_SECONDS);
    assert_eq!(by_judge.iter().map(|row| row[UNJUDGED_CLASS]).sum::<u32>(), 3, "the notes were counted twice, or not again, after a reset");
}
