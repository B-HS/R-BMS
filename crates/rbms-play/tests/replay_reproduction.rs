//! Headless proof that a replay reproduces the run it recorded.
//!
//! A [`PlaySession`] driven against [`NullSink`] needs no audio device and no window, so a whole
//! interactive run can be played, recorded, and then reproduced from its own recording inside a
//! test. The reproduction has to land on exactly the same judgement — same counts, same EX, same
//! early/late split, same gauge — or replays, the analysis screen and every score derived from them
//! are worthless.

use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::GaugeAutoShift;
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_play::{JudgeSetup, NullSink, PlaySession, PlaySummary, SessionClock, SessionOptions};
use rbms_store::{Replay, ReplayEvent};

/// How finely both runs step the song clock. Inputs land on multiples of this, so a recorded input
/// and the live one it came from are judged in the same order relative to the miss sweep.
const STEP_US: i64 = 5_000;

/// How long past the last note both runs keep stepping, so every unhit note is swept to a miss.
const TAIL_US: i64 = 2_500_000;

/// Judge offset both runs are set up with, chosen non-zero so a judgement that depended on it would
/// show up: it moves where the notes are drawn and nothing of how an input is judged.
const JUDGE_OFFSET_US: i64 = 12_000;

/// How far from its inputs a replay recorded before that was judged, in milliseconds, for the test
/// of one: far enough to move this chart's presses across a judge window.
const RECORDED_OFFSET_MS: i32 = 40;

/// Eight notes on lane 0 at 2.0 s .. 5.5 s, with an accompaniment channel so the run also books
/// non-note sounds.
const CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01010101\r\n#00211:01010101\r\n";

/// Presses covering the whole judgement range: dead on, early by enough to fall between the stock
/// PGREAT window and a widened one, late, late by far more, plus one note left untouched so a miss
/// is swept.
const PRESSES_US: &[i64] = &[2_000_000, 2_475_000, 3_040_000, 3_500_000, 4_100_000, 4_500_000, 5_000_000];

/// How long each press is held.
const HOLD_US: i64 = 50_000;

fn model() -> rbms_model::Model {
    rbms_chart::to_model(&rbms_parser::parse(CHART), rbms_model::Mode::BEAT_7K)
}

fn options() -> SessionOptions {
    SessionOptions { judge_offset_us: JUDGE_OFFSET_US, ..Default::default() }
}

fn end_us() -> i64 {
    PlaySession::new(model(), options()).last_time_us() + TAIL_US
}

/// A JUDGE screen set to something other than its defaults on every row that this chart can show:
/// a different candidate algorithm, widened and narrowed timing tiers, a longer release margin, a
/// gauge table no mode selects, and an auto-shift that re-picks every frame.
fn custom_setup() -> JudgeSetup {
    JudgeSetup {
        judge_rate_key: [140, 120, 90],
        judge_rate_scratch: [80, 150, 100],
        longnote_margin_rate: 150,
        algorithm: JudgeAlgorithm::Combo,
        gauge_set: Some(GaugeSetId::Lr2),
        gauge_auto_shift: GaugeAutoShift::SelectToUnder,
        bottom_shiftable_gauge: GaugeKind::Easy,
        ..JudgeSetup::default()
    }
}

/// A session set up the way this run is played, so every path that builds one agrees.
fn session(options: SessionOptions, judge: JudgeSetup) -> PlaySession {
    let mut session = PlaySession::new(model(), options);
    session.set_judge_setup(judge);
    session
}

/// Play the chart by hand and hand back what was judged and what was recorded.
fn play_by_hand() -> (PlaySummary, Vec<ReplayEvent>) {
    play_by_hand_with(JudgeSetup::default())
}

/// Play the chart by hand under `judge`.
fn play_by_hand_with(judge: JudgeSetup) -> (PlaySummary, Vec<ReplayEvent>) {
    let mut session = session(options(), judge);
    let mut song_us = 0;
    while song_us <= end_us() {
        if PRESSES_US.contains(&song_us) {
            session.press(0, song_us, &mut NullSink);
        }
        if PRESSES_US.contains(&(song_us - HOLD_US)) {
            session.release(0, song_us);
        }
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    (session.summary(), session.recorded_events().to_vec())
}

fn replay_of(events: Vec<ReplayEvent>) -> Replay {
    Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: rbms_model::Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events,
    }
}

/// Reproduce a recording, optionally scrubbing to `seek_to` partway through.
fn reproduce(events: Vec<ReplayEvent>, seek_to: Option<i64>) -> PlaySummary {
    reproduce_with(events, seek_to, JudgeSetup::default())
}

/// Reproduce a recording under `judge`.
fn reproduce_with(events: Vec<ReplayEvent>, seek_to: Option<i64>, judge: JudgeSetup) -> PlaySummary {
    let mut session = session(SessionOptions { replay: Some(replay_of(events)), ..options() }, judge);
    if let Some(target_us) = seek_to {
        session.seek(target_us);
    }
    let start_us = seek_to.unwrap_or(0);
    let mut song_us = start_us;
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    session.summary()
}

#[test]
fn a_replay_reproduces_the_run_it_recorded() {
    let (played, recorded) = play_by_hand();
    assert!(played.counts.iter().filter(|count| **count > 0).count() >= 3, "the hand-played run must span several judgements: {played:?}");
    assert!(played.counts[4] > 0, "one note is left unhit so the run also covers a swept miss: {played:?}");
    assert_eq!(recorded.len(), PRESSES_US.len() * 2, "every press and release is recorded");

    let reproduced = reproduce(recorded, None);
    assert_eq!(reproduced, played, "reproducing the recording must land on exactly the same judgement");
}

#[test]
fn reproducing_the_same_replay_twice_gives_the_same_judgement() {
    let (_, recorded) = play_by_hand();
    assert_eq!(reproduce(recorded.clone(), None), reproduce(recorded, None));
}

#[test]
fn scrubbing_into_the_middle_rebuilds_the_same_judgement_as_playing_straight_through() {
    let (played, recorded) = play_by_hand();
    for target_us in [0, 2_600_000, 4_000_000, 5_400_000] {
        assert_eq!(reproduce(recorded.clone(), Some(target_us)), played, "a seek to {target_us} µs must rebuild the run exactly");
    }
}

/// A chart whose judgement is driven by the clock rather than by the inputs: one note, and a mine
/// a second later that only bites while the lane is held down.
const MINE_CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:0100\r\n#001D1:0022\r\n";

/// A hold that starts before the note and lasts past the mine.
fn mine_events() -> Vec<ReplayEvent> {
    vec![ReplayEvent { t: 1_900_000, lane: 0, press: true, backward: false }, ReplayEvent { t: 3_400_000, lane: 0, press: false, backward: false }]
}

fn mine_replay() -> Replay {
    Replay { events: mine_events(), ..replay_of(Vec::new()) }
}

#[test]
fn scrubbing_to_the_end_settles_the_same_clock_driven_damage_as_playing_straight_through() {
    let model = || rbms_chart::to_model(&rbms_parser::parse(MINE_CHART), rbms_model::Mode::BEAT_7K);
    let end_us = 5_000_000;

    let mut forward = PlaySession::new(model(), SessionOptions { replay: Some(mine_replay()), ..options() });
    let mut song_us = 0;
    while song_us <= end_us {
        forward.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }

    let mut scrubbed = PlaySession::new(model(), SessionOptions { replay: Some(mine_replay()), analysis: true, ..options() });
    scrubbed.seek(end_us);

    let played = forward.summary();
    assert!(played.gauge_value < 100.0, "the run really did take the mine, or this proves nothing: {played:?}");
    assert_eq!(scrubbed.summary(), played, "a seek has to replay the inputs against a moving clock, not settle them all at the target");
}

#[test]
fn a_replay_run_records_nothing_of_its_own() {
    let (_, recorded) = play_by_hand();
    let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay_of(recorded)), ..options() });
    session.tick(SessionClock::at(end_us()), &mut NullSink);
    assert!(session.recorded_events().is_empty(), "reproducing a replay must not record a second one on top of it");
}

/// Run `session` from the start of the chart to its end.
fn run_through(mut session: PlaySession) -> PlaySummary {
    let mut song_us = 0;
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    session.summary()
}

#[test]
fn a_reproduction_does_not_depend_on_the_offset_of_whoever_watches_it() {
    let (played, recorded) = play_by_hand();
    for watched_at_us in [0, JUDGE_OFFSET_US + 40_000, -150_000] {
        let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay_of(recorded.clone())), ..options() });
        session.set_judge_offset_us(watched_at_us);
        assert_eq!(run_through(session), played, "watched at {watched_at_us} us the same inputs came out otherwise");
    }
}

#[test]
fn a_replay_that_states_an_offset_is_judged_that_far_from_its_inputs() {
    let (played, recorded) = play_by_hand();
    let stated = Replay { offset_ms: RECORDED_OFFSET_MS, ..replay_of(recorded) };
    let session = PlaySession::new(model(), SessionOptions { replay: Some(stated), ..options() });
    assert_ne!(run_through(session), played, "a replay recorded while the offset moved the judgement has to be judged at the offset it states");
}

#[test]
fn a_replay_reproduces_a_run_played_on_a_custom_judge_setup() {
    let (played, recorded) = play_by_hand_with(custom_setup());
    assert_eq!(reproduce_with(recorded, None, custom_setup()), played, "a custom JUDGE setup must reproduce as exactly as the default one");
}

#[test]
fn scrubbing_a_custom_judge_setup_rebuilds_it_rather_than_the_defaults() {
    let (played, recorded) = play_by_hand_with(custom_setup());
    for target_us in [0, 2_600_000, 4_000_000, 5_400_000] {
        assert_eq!(reproduce_with(recorded.clone(), Some(target_us), custom_setup()), played, "a seek to {target_us} µs must rebuild the same setup");
    }
}

#[test]
fn a_custom_judge_setup_judges_the_same_inputs_differently_from_the_defaults() {
    let (default_run, _) = play_by_hand();
    let (custom_run, _) = play_by_hand_with(custom_setup());
    assert_ne!(custom_run, default_run, "the custom setup has to change something, or the tests above prove nothing");
}

/// A chart for the inputs a frame can change the judgement of: key-lane notes for short taps, a
/// long note on the second lane for a release and a re-grab four microseconds apart, and a mine on
/// the third lane for a key that comes up just before the mine and goes down again just after it.
///
/// Bar 1 runs from 2.0 s to 4.0 s at 120 BPM: notes on lane 0 every half second from 2.0 s, the long
/// note on lane 1 from 2.25 s to 3.75 s, and the mine on lane 2 at 3.0 s.
const FRAME_CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00152:0001000000000001\r\n#001D3:0000000022000000\r\n#00211:01\r\n";

/// The inputs of the run on [`FRAME_CHART`], `(time, lane, press)` in time order. Several pairs fall
/// inside a single frame at every rate tested, and several straddle where a 60 Hz frame would fall.
const FRAME_INPUTS: &[(i64, usize, bool)] = &[
    (1_998_000, 0, true),
    (2_004_000, 0, false),
    (2_251_000, 1, true),
    (2_492_000, 0, true),
    (2_492_900, 0, false),
    (2_900_000, 2, true),
    (2_999_998, 2, false),
    (3_000_003, 2, true),
    (3_031_000, 0, true),
    (3_036_000, 0, false),
    (3_100_000, 1, false),
    (3_100_004, 1, true),
    (3_200_000, 2, false),
    (3_499_000, 0, true),
    (3_499_001, 0, false),
    (3_749_000, 1, false),
];

/// The frame rates a run is played and reproduced at: 1 ms, a 120 Hz frame, a 60 Hz frame and a
/// dropped-frame 30 Hz one, in microseconds.
const FRAME_STEPS_US: [i64; 4] = [1_000, 8_000, 16_667, 33_333];

/// Frame lengths that change from frame to frame, the way a loaded machine's do.
const UNEVEN_STEPS_US: [i64; 7] = [16_667, 3_100, 41_000, 9_999, 16_667, 250, 27_300];

fn frame_model() -> rbms_model::Model {
    rbms_chart::to_model(&rbms_parser::parse(FRAME_CHART), rbms_model::Mode::BEAT_7K)
}

fn frame_end_us() -> i64 {
    PlaySession::new(frame_model(), options()).last_time_us() + TAIL_US
}

/// The frames a run is driven on: `steps` taken over and over from the start of the song until
/// past its end.
fn frames(steps: &[i64]) -> Vec<i64> {
    let end_us = frame_end_us();
    let mut frames = Vec::new();
    let mut song_us = 0;
    for step_us in steps.iter().cycle() {
        frames.push(song_us);
        if song_us > end_us {
            break;
        }
        song_us += step_us;
    }
    frames
}

/// Everything a run is judged by and everything read back off it.
type Outcome = (PlaySummary, rbms_play::PlayRecord);

/// Play [`FRAME_INPUTS`] by hand on `frames`, the way the application does: every input that
/// arrived since the last frame is handed over at the instant it arrived, then the frame is ticked.
fn play_on_frames(frames: &[i64], judge: JudgeSetup) -> (Outcome, Vec<ReplayEvent>) {
    let mut session = PlaySession::new(frame_model(), options());
    session.set_judge_setup(judge);
    let mut next = 0;
    for &frame_us in frames {
        while let Some(&(at_us, lane, press)) = FRAME_INPUTS.get(next).filter(|input| input.0 <= frame_us) {
            if press {
                session.press(lane, at_us, &mut NullSink);
            } else {
                session.release(lane, at_us);
            }
            next += 1;
        }
        session.tick(SessionClock::at(frame_us), &mut NullSink);
    }
    ((session.summary(), session.record()), session.recorded_events().to_vec())
}

/// Reproduce `events` on `frames`.
fn reproduce_on_frames(events: Vec<ReplayEvent>, frames: &[i64], judge: JudgeSetup) -> Outcome {
    let mut session = PlaySession::new(frame_model(), SessionOptions { replay: Some(replay_of(events)), ..options() });
    session.set_judge_setup(judge);
    for &frame_us in frames {
        session.tick(SessionClock::at(frame_us), &mut NullSink);
    }
    (session.summary(), session.record())
}

/// The same inputs played at 1 ms, 120 Hz, 60 Hz, 30 Hz and on uneven frames, by hand and from the
/// replay of any of them, judge every note, the gauge, the mine and the long note exactly alike:
/// the judgement depends on when each input arrived, never on where the frames fell around it.
fn assert_frame_rate_does_not_matter(judge: JudgeSetup) {
    let grids: Vec<Vec<i64>> = FRAME_STEPS_US.iter().map(|step_us| frames(&[*step_us])).chain([frames(&UNEVEN_STEPS_US)]).collect();
    let (reference, recorded) = play_on_frames(&grids[0], judge);
    assert!(reference.0.counts.iter().filter(|count| **count > 0).count() >= 3, "the run must span several judgements: {:?}", reference.0);
    for grid in &grids {
        let (played, events) = play_on_frames(grid, judge);
        assert_eq!(events, recorded, "every input is recorded at the instant it arrived, whatever the frame rate");
        assert_eq!(played, reference, "played by hand on frames {:?}", &grid[..3]);
        for replay_grid in &grids {
            assert_eq!(reproduce_on_frames(recorded.clone(), replay_grid, judge), reference, "reproduced on frames {:?}", &replay_grid[..3]);
        }
    }
}

#[test]
fn a_run_judges_the_same_at_every_frame_rate_and_so_does_its_replay() {
    assert_frame_rate_does_not_matter(JudgeSetup::default());
}

/// The long note judged as a hell-charge note pays its gauge ticks on the time it was held, so a
/// replay at another frame rate lands on the same gauge to the last tick.
#[test]
fn hell_charge_ticks_judge_the_same_at_every_frame_rate_and_so_does_their_replay() {
    let judge = JudgeSetup { ln_mode: rbms_judge::ln::LnMode::HellChargeNote, ..JudgeSetup::default() };
    assert_frame_rate_does_not_matter(judge);
}

/// The mine on lane 2 went by while the key was up for five microseconds, so it costs nothing,
/// however long the frame around it was: just past the mine the gauge is that of a run that never
/// touched lane 2.
#[test]
fn a_key_up_for_an_instant_around_a_mine_is_not_charged_for_it_at_any_frame_rate() {
    let gauge_after = |grid: &[i64], mine_lane_too: bool| {
        let mut session = PlaySession::new(frame_model(), options());
        let mut next = 0;
        for &frame_us in grid.iter().take_while(|frame_us| **frame_us <= JUST_PAST_THE_MINE_US) {
            while let Some(&(at_us, lane, press)) = FRAME_INPUTS.get(next).filter(|input| input.0 <= frame_us) {
                next += 1;
                if lane == MINE_LANE && !mine_lane_too {
                    continue;
                }
                if press {
                    session.press(lane, at_us, &mut NullSink);
                } else {
                    session.release(lane, at_us);
                }
            }
            session.tick(SessionClock::at(frame_us), &mut NullSink);
        }
        session.summary().gauge_value
    };
    for step_us in FRAME_STEPS_US {
        let grid = frames(&[step_us]);
        assert_eq!(gauge_after(&grid, true), gauge_after(&grid, false), "{step_us} us frames");
    }
}

/// The lane of [`FRAME_CHART`] whose only object is a mine.
const MINE_LANE: usize = 2;

/// A moment after the mine on that lane went by, and before the next judgement of the chart.
const JUST_PAST_THE_MINE_US: i64 = 3_020_000;
