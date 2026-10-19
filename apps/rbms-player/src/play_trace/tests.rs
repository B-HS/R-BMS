//! The trace of a run played by hand, checked against the judgments the run is scored by: what
//! each object's state says adds up to the counts the summary holds, tier by tier and early against
//! late, on a finished run, a run left part-way, a run on RANDOM, a run SCRATCH AUTO played the
//! turntable of, and runs of charge and hell-charge notes.

use rbms_chart::shuffle::NoteOption;
use rbms_ir::{
    PlayTrace, TRACE_KIND_CHARGE_END, TRACE_KIND_LONG_END, TRACE_KIND_LONG_HEAD, TRACE_KIND_NORMAL, TRACE_MAX_SPEED_CHANGES, TRACE_MAX_TEMPO_POINTS, TraceSpeed,
};
use rbms_judge::Judge;
use rbms_model::{LnKind, Mode, NoteKind};
use rbms_play::{NullSink, PlaySession, PlaySummary, SessionClock, SessionOptions};

use super::*;

/// A seven-key chart of four measures: eighth notes on the first key, quarter notes on the second,
/// a turntable note in each measure, a long note on the third key in each of the last two, which the
/// chart leaves to LN MODE to say the flavour of, and a last note on the seventh key.
const TRACE_CHART: &[u8] = b"#PLAYER 1\n#BPM 150\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n#LNTYPE 1\n\
#00111:0101010101010101\n#00112:01010101\n#00116:01000000\n\
#00211:0101010101010101\n#00212:01010101\n#00216:00000100\n\
#00311:0101010101010101\n#00316:01000000\n#00353:01000100\n\
#00411:01010101\n#00416:00010000\n#00453:0001000100000000\n#00419:00000001\n";

/// The file name the chart is parsed under.
const TRACE_CHART_NAME: &str = "trace.bms";

/// How far from its time each press lands, by the note's place among the notes pressed: on time,
/// early and late inside PGREAT, GREAT both ways, a late GOOD nearer its own note than the next, and
/// one note let pass.
const OFFSETS_US: [Option<i64>; 7] = [Some(0), Some(8_000), Some(-12_000), Some(40_000), Some(-40_000), Some(-80_000), None];

/// How long a normal note's key stays down.
const KEY_HELD_US: i64 = 50_000;

/// How far ahead of the first note of the first key a press lands that takes no note: inside the
/// 空POOR band and outside BAD.
const EMPTY_POOR_LEAD_US: i64 = 300_000;

/// The step the engine is run at, and how far past the chart's own playing time it is run.
const RUN_FRAME_US: i64 = 10_000;
const RUN_MARGIN_US: i64 = 1_000_000;

/// Where the left run is let go: into the third measure, with the long notes still to come.
const LEFT_AT_US: i64 = 3_500_000;

/// The seed the RANDOM run is laid out with.
const LAYOUT_SEED: u64 = 0x5eed_1234;

/// Microseconds in a millisecond, which the run's frames are counted in.
const MICROS_PER_MILLI: i64 = 1_000;

/// One input: when it lands, its lane, and whether the key goes down.
type Input = (i64, usize, bool);

fn chart(flavour: LnKind, option: NoteOption) -> rbms_model::Model {
    let source = rbms_parser::parse_with(TRACE_CHART, Default::default());
    let mut model = rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, TRACE_CHART_NAME));
    rbms_chart::resolve_long_note_flavour(&mut model, flavour);
    rbms_chart::shuffle::apply(&mut model, option, LAYOUT_SEED);
    model
}

/// The presses and releases of the notes of `model` played by hand, each [`OFFSETS_US`] off its
/// time by its place among them; a long note is held from its head's press to its end, released
/// on time. The turntable is left alone when `scratch_auto` plays it. An empty POOR comes first
/// when `with_empty_poor`.
fn inputs(model: &rbms_model::Model, scratch_auto: bool, with_empty_poor: bool) -> Vec<Input> {
    let mut inputs = Vec::new();
    let mut pressed = 0;
    let mut first_key_note = None;
    for line in &model.timelines {
        for (lane, note) in line.notes.iter().enumerate() {
            let Some(note) = note else {
                continue;
            };
            if scratch_auto && model.mode.is_scratch(lane) {
                continue;
            }
            match note.kind {
                NoteKind::Normal | NoteKind::LongStart { .. } => {
                    pressed += 1;
                    let Some(offset_us) = OFFSETS_US[(pressed - 1) % OFFSETS_US.len()] else {
                        continue;
                    };
                    first_key_note = first_key_note.or((!model.mode.is_scratch(lane)).then_some((line.time_us, lane)));
                    inputs.push((line.time_us + offset_us, lane, true));
                    if matches!(note.kind, NoteKind::Normal) {
                        inputs.push((line.time_us + offset_us + KEY_HELD_US, lane, false));
                    }
                }
                NoteKind::LongEnd { .. } => inputs.push((line.time_us, lane, false)),
                NoteKind::Mine { .. } => {}
            }
        }
    }
    if let Some((time_us, lane)) = first_key_note.filter(|_| with_empty_poor) {
        inputs.push((time_us - EMPTY_POOR_LEAD_US, lane, true));
        inputs.push((time_us - EMPTY_POOR_LEAD_US + KEY_HELD_US, lane, false));
    }
    inputs.sort_unstable();
    inputs
}

/// Run `model` with `inputs` from its first moment until `until_us`.
fn run(model: rbms_model::Model, scratch_auto: bool, inputs: &[Input], until_us: Option<i64>) -> PlaySession {
    let mode = model.mode;
    let auto_lanes = if scratch_auto { (0..mode.key).map(|lane| mode.is_scratch(lane)).collect() } else { Vec::new() };
    let mut session = PlaySession::new(model, SessionOptions { auto_lanes, seed: LAYOUT_SEED, ..SessionOptions::default() });
    let end_us = until_us.unwrap_or(session.play_time_ms() * MICROS_PER_MILLI + RUN_MARGIN_US);
    let mut next = 0;
    for frame in 0..=end_us / RUN_FRAME_US {
        let now_us = frame * RUN_FRAME_US;
        while let Some((at_us, lane, press)) = inputs.get(next).copied().filter(|(at_us, _, _)| *at_us <= now_us) {
            if press {
                session.press(lane, at_us, &mut NullSink);
            } else {
                session.release(lane, at_us);
            }
            next += 1;
        }
        session.tick(SessionClock::at(now_us), &mut NullSink);
    }
    session
}

fn trace_of(session: &PlaySession, random: NoteOption) -> PlayTrace {
    let speed = SpeedLog::default();
    let run =
        TraceRun { mode: session.model().mode.name, lntype: 0, random: crate::ir_map::ir_random(random), speed: &speed, speed_end: TraceSpeed::default() };
    TraceSource::of(session, &session.record(), run).trace()
}

/// What the states of a trace add up to: per tier from PGREAT to POOR, early and late of PGREAT to
/// BAD, and the judged objects nothing consumed (a plain long note's end aside).
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    counts: [u32; 5],
    early: [u32; 4],
    late: [u32; 4],
    unreached: u32,
    judged_objects: u32,
}

fn tally(trace: &PlayTrace) -> Tally {
    let notes = &trace.notes;
    let mut tally = Tally::default();
    for at in 0..notes.state.len() {
        let state = usize::from(notes.state[at]);
        if notes.kind[at] == TRACE_KIND_LONG_END {
            assert_eq!(state, 0, "a plain long note is judged on its head");
            continue;
        }
        tally.judged_objects += 1;
        if state == 0 {
            tally.unreached += 1;
            assert_eq!(notes.err_us[at], 0, "an object nothing consumed carries no timing");
            continue;
        }
        tally.counts[state - 1] += 1;
        if state <= tally.early.len() {
            if notes.err_us[at] >= 0 {
                tally.early[state - 1] += 1;
            } else {
                tally.late[state - 1] += 1;
            }
        }
    }
    tally
}

/// Hold `trace` to `summary`, the summary the run is recorded with.
fn holds_to(trace: &PlayTrace, summary: &PlaySummary) {
    let tally = tally(trace);
    let poor = Judge::Poor as usize;
    let mut counts = tally.counts;
    counts[poor] += tally.unreached;
    assert_eq!(counts[..], summary.counts[..=poor], "each tier, with every object never reached a POOR let pass");
    assert_eq!(tally.early[..], summary.early[..poor], "early of PGREAT to BAD");
    assert_eq!(tally.late[..], summary.late[..poor], "late of PGREAT to BAD, the notes never reached aside");
    assert_eq!(trace.empty_poors.time_us.len() as u32, summary.empty_poor);
    assert_eq!(trace.empty_poors.lane.len(), trace.empty_poors.time_us.len());
    assert_eq!(tally.judged_objects, summary.total_notes, "one judged object for each note the run is scored out of");
    let order: Vec<(i64, u32)> = trace.notes.time_us.iter().copied().zip(trace.notes.lane.iter().copied()).collect();
    assert!(order.is_sorted(), "the notes are in chart order and, at one time, in lane order");
}

#[test]
fn a_finished_run_adds_up_to_its_summary() {
    let model = chart(LnKind::Ln, NoteOption::Off);
    let session = run(model.clone(), false, &inputs(&model, false, true), None);
    let summary = session.summary();
    assert!(summary.counts[..3].iter().all(|count| *count > 0) && summary.counts[4] > 0, "the fixture run has PGREAT to GOOD and POOR: {summary:?}");
    assert_eq!(summary.empty_poor, 1, "the fixture run has its one empty POOR: {summary:?}");
    assert_eq!(summary.total_judged, summary.total_notes, "the fixture run is played to its end");
    let trace = trace_of(&session, NoteOption::Off);
    holds_to(&trace, &summary);
    assert_eq!(tally(&trace).unreached, 0);
    assert!(trace.auto_lanes.is_empty(), "nothing was played for the player");
    assert_eq!(trace.gauge.series.len(), rbms_judge::gauge::GaugeIndex::COUNT, "every gauge's history");
    assert_eq!(trace.gauge_bounds.len(), rbms_judge::gauge::GaugeIndex::COUNT);
    assert!(trace.gauge.series.iter().all(|series| series.len() == trace.gauge.series[0].len() && !series.is_empty()));
    assert_eq!((trace.format.as_str(), trace.score_id.as_deref()), (rbms_ir::TRACE_FORMAT, None));
}

#[test]
fn an_empty_poor_is_traced_at_the_press_that_drew_it() {
    let model = chart(LnKind::Ln, NoteOption::Off);
    let presses = inputs(&model, false, true);
    let session = run(model.clone(), false, &presses, None);
    let trace = trace_of(&session, NoteOption::Off);
    let first = model.timelines.iter().find_map(|line| line.notes[0].as_ref().map(|_| line.time_us)).expect("the first key has a note");
    assert_eq!(trace.empty_poors.time_us, [first - EMPTY_POOR_LEAD_US]);
    assert_eq!(trace.empty_poors.lane, [0]);
}

#[test]
fn a_run_left_part_way_adds_up_to_the_summary_it_is_kept_with() {
    let model = chart(LnKind::Ln, NoteOption::Off);
    let session = run(model.clone(), false, &inputs(&model, false, false), Some(LEFT_AT_US));
    let summary = session.summary();
    assert!(summary.total_judged > 0 && summary.total_judged < summary.total_notes, "the fixture run is left part-way: {summary:?}");
    let left = summary.left_unfinished();
    let trace = trace_of(&session, NoteOption::Off);
    holds_to(&trace, &left);
    assert_eq!(tally(&trace).unreached, summary.total_notes - summary.total_judged, "the notes never reached are state 0, not POOR");
}

#[test]
fn a_random_run_is_traced_as_it_was_laid_out() {
    let model = chart(LnKind::Ln, NoteOption::Random);
    let original = chart(LnKind::Ln, NoteOption::Off);
    let session = run(model.clone(), false, &inputs(&model, false, false), None);
    let trace = trace_of(&session, NoteOption::Random);
    holds_to(&trace, &session.summary());
    assert_eq!(trace.random, rbms_ir::RandomOption::Random);
    let lanes_of = |model: &rbms_model::Model| {
        let mut lanes: Vec<(i64, usize)> = model
            .timelines
            .iter()
            .flat_map(|line| line.notes.iter().enumerate().filter(|(_, n)| n.is_some()).map(move |(lane, _)| (line.time_us, lane)))
            .collect();
        lanes.sort_unstable();
        lanes
    };
    assert_ne!(lanes_of(&model), lanes_of(&original), "the seed moves the keys");
    let traced_heads: Vec<(i64, usize)> = trace
        .notes
        .time_us
        .iter()
        .zip(&trace.notes.lane)
        .zip(&trace.notes.kind)
        .filter(|(_, kind)| **kind != TRACE_KIND_LONG_END)
        .map(|((time_us, lane), _)| (*time_us, *lane as usize))
        .collect();
    let laid_out_heads: Vec<(i64, usize)> = model
        .timelines
        .iter()
        .flat_map(|line| {
            line.notes.iter().enumerate().filter_map(move |(lane, note)| match note.as_ref().map(|note| &note.kind) {
                Some(NoteKind::Normal | NoteKind::LongStart { .. }) => Some((line.time_us, lane)),
                _ => None,
            })
        })
        .collect();
    let mut laid_out_heads = laid_out_heads;
    laid_out_heads.sort_unstable();
    assert_eq!(traced_heads, laid_out_heads, "the trace holds the lanes the run was played on");
}

#[test]
fn a_scratch_auto_run_leaves_the_turntable_out() {
    let model = chart(LnKind::Ln, NoteOption::Off);
    let session = run(model.clone(), true, &inputs(&model, true, false), None);
    assert!(session.has_auto_played_notes(), "SCRATCH AUTO played the turntable");
    let trace = trace_of(&session, NoteOption::Off);
    holds_to(&trace, &session.summary());
    assert_eq!(model.mode, Mode::BEAT_7K);
    let scratch = (0..model.mode.key).find(|lane| model.mode.is_scratch(*lane)).expect("seven keys have a turntable") as u32;
    assert!(!trace.notes.lane.contains(&scratch), "no turntable note is in the trace");
    assert!(trace.auto_lanes[scratch as usize] && trace.auto_lanes.iter().filter(|auto| **auto).count() == 1);
}

#[test]
fn charge_and_hell_charge_runs_judge_both_ends_and_add_up() {
    for flavour in [LnKind::Cn, LnKind::Hcn] {
        let model = chart(flavour, NoteOption::Off);
        let session = run(model.clone(), false, &inputs(&model, false, false), None);
        let trace = trace_of(&session, NoteOption::Off);
        holds_to(&trace, &session.summary());
        let heads = trace.notes.kind.iter().filter(|kind| **kind == TRACE_KIND_LONG_HEAD).count();
        let ends = trace.notes.kind.iter().filter(|kind| **kind == TRACE_KIND_CHARGE_END).count();
        assert!(heads > 0 && heads == ends, "{flavour:?}: every head has a charge end ({heads} heads, {ends} ends)");
        assert!(!trace.notes.kind.contains(&TRACE_KIND_LONG_END), "{flavour:?}: no plain end");
        let left = run(model.clone(), false, &inputs(&model, false, false), Some(LEFT_AT_US));
        holds_to(&trace_of(&left, NoteOption::Off), &left.summary().left_unfinished());
    }
}

#[test]
fn a_plain_long_note_is_a_head_judged_once_and_an_end_left_at_state_zero() {
    let model = chart(LnKind::Ln, NoteOption::Off);
    let trace = trace_of(&run(model.clone(), false, &inputs(&model, false, false), None), NoteOption::Off);
    let kinds = &trace.notes.kind;
    assert_eq!(kinds.iter().filter(|kind| **kind == TRACE_KIND_LONG_HEAD).count(), kinds.iter().filter(|kind| **kind == TRACE_KIND_LONG_END).count());
    assert!(kinds.iter().filter(|kind| **kind == TRACE_KIND_LONG_HEAD).count() > 0);
    assert!(kinds.contains(&TRACE_KIND_NORMAL));
}

#[test]
fn the_tempo_names_where_the_chart_opens_and_nothing_it_does_not_change() {
    let model = chart(LnKind::Ln, NoteOption::Off);
    let trace = trace_of(&run(model.clone(), false, &[], None), NoteOption::Off);
    assert_eq!(trace.tempo.len(), 1, "a chart of one tempo has only its opening: {:?}", trace.tempo);
    assert_eq!((trace.tempo[0].t_us, trace.tempo[0].bpm, trace.tempo[0].stop_us), (0, model.init_bpm, 0));
}

fn speed(hispeed: f64, offset_ms: i32) -> TraceSpeed {
    TraceSpeed { hispeed, offset_ms, ..TraceSpeed::default() }
}

#[test]
fn the_speed_log_keeps_the_start_the_end_and_each_change_between() {
    let mut log = SpeedLog::default();
    log.observe(speed(2.0, 0), -500);
    log.observe(speed(2.0, 0), 0);
    log.observe(speed(2.5, 0), 1_200);
    log.observe(speed(2.5, -3), 1_250);
    let speeds = log.finished(speed(2.5, -4));
    assert_eq!(speeds.start, speed(2.0, 0), "the run started on what was first seen");
    assert_eq!(speeds.end, speed(2.5, -4));
    let changes: Vec<(i64, &str, f64)> = speeds.changes.iter().map(|change| (change.t_ms, change.field.as_str(), change.value)).collect();
    assert_eq!(changes, [(1_200, "hispeed", 2.5), (1_250, "offset_ms", -3.0), (1_250, "offset_ms", -4.0)], "the end is counted at the last look");
    let never = SpeedLog::default().finished(speed(3.0, 1));
    assert_eq!((never.start, never.end, never.changes.len()), (speed(3.0, 1), speed(3.0, 1), 0));
}

#[test]
fn a_run_that_moves_its_settings_past_the_cap_keeps_its_first_and_its_last_changes() {
    let mut log = SpeedLog::default();
    let looks = TRACE_MAX_SPEED_CHANGES as i32 * 2;
    for look in 0..=looks {
        log.observe(speed(2.0, look), i64::from(look));
    }
    let speeds = log.finished(speed(2.0, looks));
    assert_eq!(speeds.changes.len(), TRACE_MAX_SPEED_CHANGES, "the changes stop at the cap");
    assert_eq!((speeds.start, speeds.end), (speed(2.0, 0), speed(2.0, looks)), "the start and the end are whole");
    let seen: Vec<i64> = speeds.changes.iter().map(|change| change.t_ms).collect();
    let first = SPEED_CHANGES_FROM_THE_START as i64;
    let last_from = i64::from(looks) - SPEED_CHANGES_TO_THE_END as i64 + 1;
    let kept: Vec<i64> = (1..=first).chain(last_from..=i64::from(looks)).collect();
    assert_eq!(seen, kept, "the first changes and the last ones, in the order they came");
}

/// How many tempo changes each measure of [`chart_of_tempos`] makes, and the most tempos channel 03
/// spells.
const TEMPO_CHANGES_PER_MEASURE: usize = 16;
const TEMPO_VALUES: usize = 254;

/// A chart of `measures` measures, its tempo changing [`TEMPO_CHANGES_PER_MEASURE`] times in each,
/// every change to a tempo other than the one before.
fn chart_of_tempos(measures: usize) -> rbms_model::Model {
    let mut text = String::from("#PLAYER 1\n#BPM 150\n#WAV01 a.wav\n#00111:01\n");
    for measure in 1..=measures {
        let values: String =
            (0..TEMPO_CHANGES_PER_MEASURE).map(|at| format!("{:02X}", 1 + (measure * TEMPO_CHANGES_PER_MEASURE + at) % TEMPO_VALUES)).collect();
        text.push_str(&format!("#{measure:03}03:{values}\n"));
    }
    let source = rbms_parser::parse_with(text.as_bytes(), Default::default());
    rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, TRACE_CHART_NAME))
}

#[test]
fn a_chart_of_more_tempo_points_than_the_cap_keeps_its_first_and_its_last() {
    let model = chart_of_tempos(TRACE_MAX_TEMPO_POINTS * 2 / TEMPO_CHANGES_PER_MEASURE);
    assert!(model.timelines.len() > TRACE_MAX_TEMPO_POINTS, "the fixture is past the cap");
    let tempo = tempo_of(&model);
    assert_eq!(tempo.len(), TRACE_MAX_TEMPO_POINTS);
    assert_eq!((tempo[0].t_us, tempo[0].bpm), (0, model.init_bpm), "the opening tempo stays");
    let last = model.timelines.last().expect("the chart has lines");
    assert_eq!(tempo.last().map(|point| (point.t_us, point.bpm)), Some((last.time_us, last.bpm)), "the last change stays");
    assert!(tempo.windows(2).all(|pair| pair[0].t_us <= pair[1].t_us), "in chart order");
}
