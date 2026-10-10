//! The record a skin's result screen reads, checked against the reference's own definitions on
//! charts written for the purpose.

use rbms_chart::to_model;
use rbms_judge::GaugeKind;
use rbms_judge::gauge::{GaugeAutoShift, GaugeIndex};
use rbms_model::{Mode, Model};
use rbms_parser::parse;
use rbms_store::{Replay, ReplayEvent};

use super::*;
use crate::session::{JudgeSetup, NullSink, PlaySession, SessionClock, SessionOptions};

/// The frame step the fixture runs are driven at.
const FRAME_US: i64 = 10_000;

/// Song time of the first note of the fixtures: bar 1 at 120 BPM.
const FIRST_NOTE_US: i64 = 2_000_000;

/// Spacing of the four-to-a-bar fixture notes.
const QUARTER_US: i64 = 500_000;

/// How close two computed statistics have to be to count as the same number.
const STAT_TOLERANCE: f32 = 1e-3;

fn model(bms: &[u8]) -> Model {
    to_model(&parse(bms), Mode::BEAT_7K)
}

/// Four notes on one key lane at 2.0, 2.5, 3.0 and 3.5 seconds, judged at the mode's stock widths.
fn four_notes() -> Model {
    model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n")
}

/// Eight notes over two bars, 2.0 to 5.5 seconds.
fn eight_notes() -> Model {
    model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00211:01010101\r\n")
}

/// Thirty-two notes over two bars, enough unhit to empty a HARD gauge well before the end.
fn dense_notes() -> Model {
    model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101010101010101010101010101\r\n#00211:01010101010101010101010101010101\r\n")
}

fn autoplay(model: Model) -> PlaySession {
    PlaySession::new(model, SessionOptions { autoplay: true, ..Default::default() })
}

/// Drive `session` frame by frame from the start of the song to `until_us`, feeding `inputs`
/// (`(time, lane, press)`, in time order) at the frame that reaches them, the way a live run does.
fn run(session: &mut PlaySession, inputs: &[(i64, usize, bool)], until_us: i64) {
    let mut next = 0;
    let mut frame_us = 0;
    while frame_us <= until_us {
        while let Some(&(at_us, lane, press)) = inputs.get(next).filter(|input| input.0 <= frame_us) {
            if press {
                session.press(lane, at_us, &mut NullSink);
            } else {
                session.release(lane, at_us);
            }
            next += 1;
        }
        session.tick(SessionClock::at(frame_us), &mut NullSink);
        frame_us += FRAME_US;
    }
}

/// How many samples a run driven every frame from zero holds once it has reached `song_ms`.
fn samples_by(song_ms: i64) -> usize {
    (song_ms / GAUGE_LOG_INTERVAL_MS + 1) as usize
}

fn gauge_values(session: &PlaySession) -> [f32; GaugeIndex::COUNT] {
    GaugeIndex::ALL.map(|index| session.judge().gauge.value_at(index))
}

fn sample_of(log: &GaugeLog, at: usize) -> [f32; GaugeIndex::COUNT] {
    GaugeIndex::ALL.map(|index| log.of(index)[at])
}

#[test]
fn the_play_time_waits_for_the_last_note_or_for_the_last_sound() {
    let chart = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00301:01\r\n");
    assert_eq!(last_note_time_ms(&chart), 3_500, "the last note is the fourth quarter of bar 1");
    assert_eq!(last_event_time_ms(&chart), 6_000, "a BGM sound in bar 3 is an event but not a note");
    assert_eq!(play_time_ms(&chart, false), 3_500 + PLAY_TIME_MARGIN_MS, "BMSPlayer.java:198 waits for the last note of a played run");
    assert_eq!(play_time_ms(&chart, true), 6_000 + PLAY_TIME_MARGIN_MS, "and for the last sound of an autoplay one");
    assert_eq!(PlaySession::new(chart, SessionOptions::default()).play_time_ms(), 8_500);
}

#[test]
fn an_autoplay_run_logs_every_gauge_on_the_half_second_grid_until_the_play_time() {
    let mut probe = autoplay(eight_notes());
    let initial = gauge_values(&probe);
    probe.tick(SessionClock::at(FIRST_NOTE_US), &mut NullSink);
    let after_one = gauge_values(&probe);
    assert_ne!(after_one[GaugeIndex::Normal.index()], initial[GaugeIndex::Normal.index()], "one PGREAT moves the NORMAL gauge");

    let mut session = autoplay(eight_notes());
    let play_time_ms = session.play_time_ms();
    assert_eq!(play_time_ms, 5_500 + PLAY_TIME_MARGIN_MS);
    run(&mut session, &[], (play_time_ms + 2_000) * 1_000);
    let record = session.record();
    let log = &record.gauge_log;

    assert_eq!(log.len(), samples_by(play_time_ms), "one sample per 500 ms from zero to the play time, and none after it");
    for index in GaugeIndex::ALL {
        assert_eq!(log.of(index).len(), log.len(), "{index:?} is logged alongside the selected gauge");
        assert!(log.of(index).windows(2).all(|pair| pair[0] <= pair[1]), "{index:?} never falls on an all-PGREAT run");
    }
    assert_eq!(sample_of(log, 0), initial, "the first sample is every gauge's starting value");
    assert_eq!(sample_of(log, samples_by(FIRST_NOTE_US / 1_000) - 2), initial, "nothing moves before the first note");
    assert_eq!(sample_of(log, samples_by(FIRST_NOTE_US / 1_000) - 1), after_one, "the sample at 2.0 s is taken after that frame's judgement");
    assert_eq!(sample_of(log, log.len() - 1), gauge_values(&session), "the last sample is where every gauge ended");
    for index in GaugeIndex::ALL {
        assert_eq!(log.last(index), Some(session.judge().gauge.value_at(index)));
    }
    assert!(log.of(GaugeIndex::Hard).iter().all(|value| *value == 100.0), "a HARD gauge starts full and stays there");
    assert!(log.of(GaugeIndex::Normal).first() < log.of(GaugeIndex::Normal).last());
    assert_eq!(record.failed_at_ms, None);
    assert_eq!(record.finished_gauge, GaugeIndex::Normal);
}

#[test]
fn the_shadow_gauges_do_not_change_what_the_played_gauge_does() {
    let mut session = autoplay(eight_notes());
    let mut plain = crate::Player::new(eight_notes(), true);
    let end_us = (session.play_time_ms() + 1_000) * 1_000;
    run(&mut session, &[], end_us);
    plain.update(end_us, |_| {});
    assert_eq!(session.judge().gauge.value(), plain.judge().gauge.value());
    assert_eq!(session.summary().clear_lamp, plain.judge().clear_lamp());
    assert_eq!(session.judge().counts, plain.judge().counts);
}

#[test]
fn a_frame_takes_at_most_one_sample_and_none_before_the_song_starts() {
    let mut session = autoplay(eight_notes());
    session.tick(SessionClock::at(-FRAME_US), &mut NullSink);
    assert!(session.record().gauge_log.is_empty(), "the playing state has not begun before the song clock reaches zero");
    session.tick(SessionClock::at(3_000_000), &mut NullSink);
    assert_eq!(session.record().gauge_log.len(), 1, "BMSPlayer.java:632 adds one sample a frame however far the clock jumped");
    session.tick(SessionClock::at(3_000_000 + FRAME_US), &mut NullSink);
    session.tick(SessionClock::at(3_000_000 + 2 * FRAME_US), &mut NullSink);
    assert_eq!(session.record().gauge_log.len(), 3, "and catches up one frame at a time");
}

#[test]
fn a_failed_run_stops_logging_and_is_padded_with_zeroes_to_the_end_of_the_chart() {
    let mut session = PlaySession::new(dense_notes(), SessionOptions { gauge: GaugeKind::Hard, ..Default::default() });
    let play_time_ms = session.play_time_ms();
    let mut frame_us = 0;
    while !session.is_failed() {
        session.tick(SessionClock::at(frame_us), &mut NullSink);
        frame_us += FRAME_US;
        assert!(frame_us < play_time_ms * 1_000, "an unplayed HARD run fails long before the chart ends");
    }
    let failed_at_ms = (frame_us - FRAME_US) / 1_000;
    let live = samples_by(failed_at_ms);
    for _ in 0..200 {
        session.tick(SessionClock::at(frame_us), &mut NullSink);
        frame_us += FRAME_US;
    }

    let record = session.record();
    let log = &record.gauge_log;
    assert_eq!(record.failed_at_ms, Some(failed_at_ms));
    assert!(record.summary.failed);
    let mut zeroes = 0;
    let mut at_ms = failed_at_ms;
    while at_ms < play_time_ms + GAUGE_LOG_INTERVAL_MS {
        zeroes += 1;
        at_ms += GAUGE_LOG_INTERVAL_MS;
    }
    assert!(zeroes > 1, "the fixture fails with most of the chart still ahead");
    assert_eq!(log.len(), live + zeroes, "what was sampled up to the failure, then BMSPlayer.java:724-728's zeroes; later frames add nothing");
    for index in GaugeIndex::ALL {
        assert!(log.of(index)[live..].iter().all(|value| *value == 0.0), "{index:?} is zero-filled whether or not it had emptied");
    }
    assert!(log.of(GaugeIndex::Normal)[live - 1] > 0.0, "a NORMAL gauge never empties, so its own samples stay above the floor");
    assert_eq!(log.of(GaugeIndex::Hard)[0], 100.0);
    assert_eq!(session.judge().gauge.value_at(GaugeIndex::Hard), 0.0);
    assert_eq!(log.last(GaugeIndex::Normal), Some(0.0), "the result reads the padded tail as the final value");
}

#[test]
fn a_run_the_gauge_shift_rescues_keeps_logging() {
    let mut session = PlaySession::new(dense_notes(), SessionOptions { gauge: GaugeKind::Hard, ..Default::default() });
    session.set_judge_setup(JudgeSetup { gauge_auto_shift: GaugeAutoShift::Continue, ..JudgeSetup::default() });
    let play_time_ms = session.play_time_ms();
    run(&mut session, &[], (play_time_ms + 1_000) * 1_000);
    let record = session.record();
    assert_eq!(record.failed_at_ms, None, "CONTINUE leaves the run going on an empty gauge");
    assert_eq!(record.gauge_log.len(), samples_by(play_time_ms));
    assert_eq!(record.gauge_log.last(GaugeIndex::Hard), Some(0.0), "the samples are the dead gauge's own value, not padding");
    assert!(record.gauge_log.last(GaugeIndex::Normal) > Some(0.0));
}

/// One input of each kind on [`four_notes`]: an empty POOR well ahead of the first note, an early
/// PGREAT, a late GREAT, an early GOOD, and the fourth note left to the sweep.
fn mixed_inputs() -> Vec<(i64, usize, bool)> {
    let notes = [FIRST_NOTE_US, FIRST_NOTE_US + QUARTER_US, FIRST_NOTE_US + 2 * QUARTER_US];
    vec![
        (notes[0] - 400_000, 0, true),
        (notes[0] - 390_000, 0, false),
        (notes[0] - 10_000, 0, true),
        (notes[0] + 20_000, 0, false),
        (notes[1] + 40_000, 0, true),
        (notes[1] + 70_000, 0, false),
        (notes[2] - 100_000, 0, true),
        (notes[2] - 70_000, 0, false),
    ]
}

fn mixed_run() -> PlaySession {
    let mut session = PlaySession::new(four_notes(), SessionOptions::default());
    let end_us = (session.play_time_ms() + 1_000) * 1_000;
    run(&mut session, &mixed_inputs(), end_us);
    session
}

#[test]
fn the_early_late_split_counts_every_judgement_and_its_totals_leave_pgreat_out() {
    let session = mixed_run();
    let record = session.record();
    assert_eq!(record.summary.counts, [1, 1, 1, 0, 1, 1]);
    assert_eq!(record.summary.early, [1, 0, 1, 0, 0, 1], "early PGREAT, early GOOD, early empty POOR");
    assert_eq!(record.summary.late, [0, 1, 0, 0, 1, 0], "late GREAT, and a swept POOR is always late");
    assert_eq!(record.total_early(), 2, "IntegerPropertyFactory.java:449 sums judges 1 to 5: the GOOD and the empty POOR, not the PGREAT");
    assert_eq!(record.total_late(), 2, "the GREAT and the swept POOR");
    assert_eq!((session.judge().fast, session.judge().slow), (2, 1), "the engine's own FAST/SLOW count PGREAT in and POOR out");
    assert_eq!(record.min_bp_with_unreached(), record.summary.min_bp, "every note was reached");
}

#[test]
fn the_timing_distribution_is_taken_from_the_notes_in_whole_milliseconds() {
    let record = mixed_run().record();
    let timing = &record.timing;
    assert_eq!(timing.buckets().len(), TIMING_DISTRIBUTION_BINS);
    assert_eq!(timing.center(), 150);
    assert_eq!(timing.buckets()[timing.center() + 10], 1, "the PGREAT came 10 ms early");
    assert_eq!(timing.buckets()[timing.center() - 40], 1, "the GREAT came 40 ms late");
    assert_eq!(timing.buckets()[timing.center() + 100], 1, "the GOOD came 100 ms early");
    assert_eq!(timing.buckets().iter().sum::<u32>(), 3, "the swept POOR is further out than the range and is dropped");
    let mean = (10.0 - 40.0 + 100.0) / 3.0f32;
    let variance = ((10.0 - mean).powi(2) + (-40.0 - mean).powi(2) + (100.0 - mean).powi(2)) / 3.0;
    assert!((timing.average() - mean).abs() < STAT_TOLERANCE, "{}", timing.average());
    assert!((timing.std_dev() - variance.sqrt()).abs() < STAT_TOLERANCE, "{}", timing.std_dev());
}

#[test]
fn the_average_duration_charges_a_missed_note_a_full_second() {
    let record = mixed_run().record();
    assert_eq!(record.total_duration_us, 10_000 + 40_000 + 100_000 + UNJUDGED_DURATION_US, "BMSPlayer.java:923");
    assert_eq!(record.avg_duration_us, record.total_duration_us / 4);
    assert_ne!(record.avg_duration_us, record.summary.avg_judge_us, "the engine's mean is signed and over the hits alone");
}

#[test]
fn an_untouched_chart_has_no_timing_statistics() {
    let session = PlaySession::new(four_notes(), SessionOptions::default());
    let record = session.record();
    assert_eq!(record.timing.buckets().iter().sum::<u32>(), 0);
    assert_eq!(record.timing.average(), TIMING_AVERAGE_ABSENT);
    assert_eq!(record.timing.std_dev(), TIMING_STD_DEV_ABSENT);
    assert_eq!(record.avg_duration_us, UNJUDGED_DURATION_US, "every note still counts as a miss");
    assert!(record.gauge_log.is_empty());
    assert_eq!(record.seconds.by_judge.iter().map(|row| row[0]).sum::<u32>(), 4, "all four notes sit in the unjudged column");
}

#[test]
fn the_judge_seconds_sort_each_note_by_its_own_second_and_split_great_and_below_by_side() {
    let record = mixed_run().record();
    let seconds = &record.seconds;
    assert_eq!(seconds.by_judge.len(), 4, "one row per second up to the last event at 3.5 s");
    assert_eq!(seconds.by_timing.len(), 4);
    assert_eq!(seconds.by_judge[0], [0; JUDGE_SECOND_KINDS]);
    assert_eq!(seconds.by_judge[2], [0, 1, 1, 0, 0, 0], "the PGREAT at 2.0 s and the GREAT at 2.5 s");
    assert_eq!(seconds.by_judge[3], [0, 0, 0, 1, 0, 1], "the GOOD at 3.0 s and the POOR at 3.5 s");
    assert_eq!(seconds.by_timing[2], [0, 1, 0, 0, 0, 0, 1, 0, 0, 0], "a PGREAT has one bucket; the late GREAT is bucket 2 + 4");
    assert_eq!(seconds.by_timing[3], [0, 0, 0, 1, 0, 0, 0, 0, 0, 1], "the early GOOD stays in bucket 3; the late POOR is bucket 5 + 4");
}

#[test]
fn a_plain_long_note_is_one_object_and_a_charge_note_is_two() {
    let mut plain = autoplay(model(b"#BPM 120\r\n#LNMODE 1\r\n#WAV01 a.wav\r\n#00151:01000001\r\n"));
    let mut charge = autoplay(model(b"#BPM 120\r\n#LNMODE 2\r\n#WAV01 a.wav\r\n#00151:01000001\r\n"));
    for session in [&mut plain, &mut charge] {
        let end_us = (session.play_time_ms() + 1_000) * 1_000;
        run(session, &[], end_us);
    }
    let (plain, charge) = (plain.record(), charge.record());
    assert_eq!(plain.timing.buckets().iter().sum::<u32>(), 1, "the end of a plain long note is left out, as MusicResult.java:355 leaves it");
    assert_eq!(charge.timing.buckets().iter().sum::<u32>(), 2, "a charge note's end is judged and counted");
    assert_eq!(plain.seconds.by_judge.iter().map(|row| row.iter().sum::<u32>()).sum::<u32>(), 1);
    assert_eq!(charge.seconds.by_judge[2][1], 1, "the head at 2.0 s");
    assert_eq!(charge.seconds.by_judge[3][1], 1, "the end at 3.5 s");
    assert_eq!((plain.avg_duration_us, charge.avg_duration_us), (0, 0), "autoplay lands on the note");
    assert_eq!(plain.timing.average(), 0.0);
}

#[test]
fn rates_are_measured_against_the_notes_that_have_gone_by() {
    let mut session = PlaySession::new(four_notes(), SessionOptions::default());
    let untouched = session.score_progress();
    assert_eq!((untouched.pass_notes, untouched.pass_max_ex(), untouched.max_ex()), (0, 0, 8));
    assert_eq!(untouched.now_rate(), 1.0, "ScoreDataProperty.java:97 rates a run nothing has passed at 100%");
    assert!(untouched.qualifies_now_rank(RANK_STEP_COUNT - 1), "so it stands at the top step until a note says otherwise");
    assert!(!untouched.qualifies_rank(1), "while over the whole chart it has secured nothing");

    let inputs = mixed_inputs();
    run(&mut session, &inputs[..6], FIRST_NOTE_US + QUARTER_US + 100_000);
    let midway = session.score_progress();
    assert_eq!(midway, ScoreProgress { ex_score: 3, pass_notes: 2, total_notes: 4 }, "an empty POOR passes no note");
    assert_eq!(midway.pass_max_ex(), 4);
    assert_eq!(midway.now_rate(), 0.75);
    assert_eq!(midway.rate(), 0.375);
    assert!(midway.qualifies_now_rank(18) && !midway.qualifies_now_rank(21), "3 of 4 so far is an A, short of AA");
    assert!(midway.qualifies_rank(9) && !midway.qualifies_rank(12), "3 of 8 over the chart has only secured a D");
    assert_eq!(midway.paced(8), 4, "a best of 8 over four notes stood at 4 after two");
    assert_eq!(midway.paced(7), 3, "ScoreDataProperty.java:119 truncates");
    assert_eq!(session.record().progress(), midway);
    assert_eq!(session.record().min_bp_with_unreached(), 1 + 2, "the empty POOR, and BMSPlayer.java:909's two notes not yet reached");

    let finished = mixed_run().record().progress();
    assert_eq!(finished.pass_notes, finished.total_notes);
    assert_eq!(finished.now_rate(), finished.rate(), "once every note has gone by the two rates agree");
}

#[test]
fn a_chart_with_no_notes_rates_as_complete_and_ranks_as_nothing() {
    let empty = ScoreProgress { ex_score: 0, pass_notes: 0, total_notes: 0 };
    assert_eq!((empty.now_rate(), empty.rate()), (1.0, 1.0));
    assert!(!empty.qualifies_now_rank(0) && !empty.qualifies_rank(0), "ScoreDataProperty.java:102, 111 gate every step on the note count");
    assert_eq!(empty.paced(100), 0);
}

#[test]
fn the_gauge_bounds_come_from_the_gauges_the_run_was_played_on() {
    let session = autoplay(eight_notes());
    let record = session.record();
    for index in GaugeIndex::ALL {
        let gauge = session.judge().gauge.gauge_at(index);
        assert_eq!(record.gauge_bounds[index.index()], GaugeBounds { min: gauge.min(), max: gauge.max(), border: gauge.border() }, "{index:?}");
    }
    assert_eq!(record.gauge_bounds[GaugeIndex::Normal.index()].border, 80.0);
}

/// A replay file as it was written before the judge settings and the scratch direction were
/// recorded: no `judge` block, no `scratch_auto`, no `gauge`, and events without `backward`.
fn legacy_replay_text(inputs: &[(i64, usize, bool)]) -> String {
    let events: Vec<String> = inputs.iter().map(|(at_us, lane, press)| format!("(t:{at_us},lane:{lane},press:{press})")).collect();
    format!("(chart_path:\"old.bms\",md5:\"abc\",mode:\"BEAT_7K\",random:\"OFF\",seed:0,offset_ms:0,events:[{}])", events.join(","))
}

#[test]
fn a_replay_file_from_before_the_record_existed_reproduces_the_record() {
    let inputs = mixed_inputs();
    let dir = std::env::temp_dir().join(format!("rbms-play-record-legacy-replay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("legacy.ron");
    std::fs::write(&path, legacy_replay_text(&inputs)).unwrap();
    let replay = Replay::load(&path).expect("a replay written before the judge block still loads");
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(replay.events.len(), inputs.len());
    assert_eq!(replay.events[0], ReplayEvent { t: inputs[0].0, lane: 0, press: true, backward: false });

    let live = mixed_run();
    let mut replayed = PlaySession::new(four_notes(), SessionOptions { replay: Some(replay), ..Default::default() });
    let end_us = (replayed.play_time_ms() + 1_000) * 1_000;
    run(&mut replayed, &[], end_us);
    assert_eq!(replayed.summary().counts, [1, 1, 1, 0, 1, 1]);
    assert_eq!(replayed.record(), live.record(), "the judgements are reproduced, so everything read back off them is too");
}

#[test]
fn scrubbing_a_replay_starts_the_gauge_history_again() {
    let replay = Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    };
    let mut session = PlaySession::new(four_notes(), SessionOptions { replay: Some(replay), ..Default::default() });
    run(&mut session, &[], 1_000_000);
    assert_eq!(session.record().gauge_log.len(), samples_by(1_000));
    session.seek(500_000);
    assert!(session.record().gauge_log.is_empty(), "the history belongs to the run the scrub just replaced");
}
