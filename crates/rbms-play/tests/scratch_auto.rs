//! SCRATCH AUTO as the player asked for it (2026-10-14): the turntable's notes are played by the
//! chart, sound on time and are drawn, and count for nothing. A run with it on has to judge exactly
//! what the same chart with no turntable notes judges -- notes, judgements, EX, combo, gauge,
//! fast/slow and progress -- whatever the turntable keys do, and its replay has to come out the
//! same again.

use rbms_model::{Mode, Model};
use rbms_play::{NullSink, PlaySession, PlaySource, PlaySummary, SessionClock, SessionOptions, SoundRequest, SoundSink, SoundTime};
use rbms_store::{Replay, ReplayEvent};

/// How finely the runs step the song clock. Every input lands on a multiple of it.
const STEP_US: i64 = 5_000;

/// How long past the last note the runs keep stepping, so every unhit note is swept.
const TAIL_US: i64 = 2_500_000;

/// How long each press is held.
const HOLD_US: i64 = 50_000;

/// The key notes both charts share: lane 1 at 2.0 s .. 5.5 s, under a stated TOTAL so the gauge
/// does not depend on how many notes the chart has.
const KEYS: &str = "#BPM 120\r\n#RANK 3\r\n#TOTAL 200\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00111:01010101\r\n#00211:01010101\r\n";

/// The turntable notes of the chart SCRATCH AUTO plays: four between the key notes of the first
/// measure, a long note over the second and a mine.
const TURNTABLE: &str = "#00116:0002000200020002\r\n#00256:02000002\r\n#001D6:00000001\r\n";

/// Presses on the key lane: on time, late, early, far late, one note left alone, on time.
const KEY_PRESSES_US: &[i64] = &[2_000_000, 2_530_000, 2_975_000, 3_620_000, 4_000_000, 4_500_000, 5_500_000];

/// Presses on the turntable, some on its notes and some nowhere near one.
const TURNTABLE_PRESSES_US: &[i64] = &[2_250_000, 2_600_000, 3_250_000, 4_000_000, 4_900_000];

fn chart(text: &str, mode: Mode) -> Model {
    rbms_chart::to_model(&rbms_parser::parse(text.as_bytes()), mode)
}

fn turntable_lanes(mode: Mode) -> Vec<bool> {
    (0..mode.key).map(|lane| mode.is_scratch(lane)).collect()
}

/// Every press and its release, in time order.
fn inputs(presses: &[(i64, usize)]) -> Vec<ReplayEvent> {
    let mut events: Vec<ReplayEvent> = presses
        .iter()
        .flat_map(|(at, lane)| {
            [ReplayEvent { t: *at, lane: *lane, press: true, backward: false }, ReplayEvent { t: at + HOLD_US, lane: *lane, press: false, backward: false }]
        })
        .collect();
    events.sort_by_key(|event| event.t);
    events
}

/// The sounds a run books, kept.
#[derive(Default)]
struct Booked(Vec<SoundRequest>);

impl SoundSink for Booked {
    fn play(&mut self, sound: SoundRequest) {
        self.0.push(sound);
    }

    fn stop_all(&mut self) {}
}

/// Play `model` by hand with `events`, stepping the clock to the end, and hand back the session.
fn play(model: Model, options: SessionOptions, events: &[ReplayEvent], sink: &mut dyn SoundSink) -> PlaySession {
    let mut session = PlaySession::new(model, options);
    let end = session.last_time_us() + TAIL_US;
    let mut next = 0;
    let mut now = 0;
    while now <= end {
        while let Some(event) = events.get(next).filter(|event| event.t <= now) {
            if event.press {
                session.press(event.lane, event.t, sink);
            } else {
                session.release(event.lane, event.t);
            }
            next += 1;
        }
        session.tick(SessionClock::at(now), sink);
        now += STEP_US;
    }
    session
}

fn key_presses() -> Vec<(i64, usize)> {
    KEY_PRESSES_US.iter().map(|at| (*at, 0)).collect()
}

fn with_turntable_presses(lane: usize) -> Vec<(i64, usize)> {
    key_presses().into_iter().chain(TURNTABLE_PRESSES_US.iter().map(|at| (*at, lane))).collect()
}

/// What two runs have to agree on for one to count exactly as the other.
fn tally(summary: PlaySummary) -> impl PartialEq + std::fmt::Debug {
    (
        summary.total_notes,
        summary.total_judged,
        summary.counts,
        summary.ex_score,
        summary.max_ex_score,
        summary.max_combo,
        (summary.fast, summary.slow, summary.early, summary.late),
        summary.empty_poor,
        summary.min_bp,
        summary.gauge_value.to_bits(),
        summary.clear_lamp,
    )
}

#[test]
fn a_run_with_scratch_auto_judges_what_the_chart_without_turntable_notes_judges() {
    let with_turntable = chart(&format!("{KEYS}{TURNTABLE}"), Mode::BEAT_7K);
    let without = chart(KEYS, Mode::BEAT_7K);
    let auto = SessionOptions { auto_lanes: turntable_lanes(Mode::BEAT_7K), ..SessionOptions::default() };

    let pressed = play(with_turntable.clone(), auto.clone(), &inputs(&with_turntable_presses(7)), &mut NullSink);
    let untouched = play(with_turntable, auto, &inputs(&key_presses()), &mut NullSink);
    let plain = play(without, SessionOptions::default(), &inputs(&key_presses()), &mut NullSink);

    assert!(pressed.has_auto_played_notes());
    assert_eq!(plain.summary().total_notes, 8);
    assert_eq!(tally(pressed.summary()), tally(plain.summary()), "the turntable notes counted for something");
    assert_eq!(tally(untouched.summary()), tally(plain.summary()), "spinning the turntable changed the run");
    assert_eq!(pressed.score_progress(), plain.score_progress());
    assert_eq!(pressed.judge().combo, plain.judge().combo);
}

#[test]
fn the_turntable_notes_sound_by_themselves_at_their_own_times() {
    let with_turntable = chart(&format!("{KEYS}{TURNTABLE}"), Mode::BEAT_7K);
    let turntable_heads: Vec<i64> = with_turntable
        .timelines
        .iter()
        .filter(|tl| tl.notes[7].as_ref().is_some_and(|note| matches!(note.kind, rbms_model::NoteKind::Normal | rbms_model::NoteKind::LongStart { .. })))
        .map(|tl| tl.time_us)
        .collect();
    let auto = SessionOptions { auto_lanes: turntable_lanes(Mode::BEAT_7K), ..SessionOptions::default() };
    let mut booked = Booked::default();
    play(with_turntable, auto, &inputs(&with_turntable_presses(7)), &mut booked);
    let scheduled_keys: Vec<i64> = booked
        .0
        .iter()
        .filter(|sound| sound.source == PlaySource::Key && sound.wav == 2)
        .filter_map(|sound| match sound.at {
            SoundTime::Scheduled(at) => Some(at),
            SoundTime::Immediate => None,
        })
        .collect();
    assert_eq!(turntable_heads.len(), 5, "four notes and the head of the long note");
    assert_eq!(scheduled_keys, turntable_heads, "each turntable note is heard once, at its own time, on the key bus");
}

#[test]
fn the_field_still_draws_the_turntable_notes() {
    let with_turntable = chart(&format!("{KEYS}{TURNTABLE}"), Mode::BEAT_7K);
    let session = PlaySession::new(with_turntable, SessionOptions { auto_lanes: turntable_lanes(Mode::BEAT_7K), ..SessionOptions::default() });
    let on_turntable = |timelines: &[rbms_model::TimeLine]| timelines.iter().filter(|tl| tl.notes[7].is_some()).count();
    assert_eq!(on_turntable(&session.model().timelines), 0, "the judged chart holds none");
    assert_eq!(on_turntable(session.field_timelines()), 6, "the field holds the four notes and both ends of the long note, and no mine");
    assert_eq!(session.auto_lanes(), turntable_lanes(Mode::BEAT_7K).as_slice());
}

#[test]
fn a_replay_of_a_scratch_auto_run_and_a_scrub_of_it_come_out_the_same() {
    let with_turntable = chart(&format!("{KEYS}{TURNTABLE}"), Mode::BEAT_7K);
    let auto = SessionOptions { auto_lanes: turntable_lanes(Mode::BEAT_7K), ..SessionOptions::default() };
    let live = play(with_turntable.clone(), auto.clone(), &inputs(&with_turntable_presses(7)), &mut NullSink);
    let replay = Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: true,
        gauge: String::new(),
        judge: Default::default(),
        events: live.recorded_events().to_vec(),
    };
    let watched = play(with_turntable, SessionOptions { replay: Some(replay), ..auto }, &[], &mut NullSink);
    assert_eq!(tally(watched.summary()), tally(live.summary()), "the replay reproduced another run");
    assert!(watched.has_auto_played_notes(), "and it was watched with the turntable played by the chart");

    let mut scrubbed = watched;
    scrubbed.seek(scrubbed.last_time_us() + TAIL_US);
    assert_eq!(tally(scrubbed.summary()), tally(live.summary()), "a scrub rebuilt the run with the turntable notes left out again");
    assert_eq!(scrubbed.field_timelines().iter().filter(|tl| tl.notes[7].is_some()).count(), 6, "and still draws them");
}

#[test]
fn both_turntables_of_a_double_play_chart_are_left_out() {
    let keys = "#BPM 120\r\n#TOTAL 200\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00121:01010101\r\n";
    let turntables = "#00116:00010001\r\n#00126:00010001\r\n";
    let both = chart(&format!("{keys}{turntables}"), Mode::BEAT_14K);
    let without = chart(keys, Mode::BEAT_14K);
    let auto = SessionOptions { auto_lanes: turntable_lanes(Mode::BEAT_14K), ..SessionOptions::default() };
    let presses: Vec<(i64, usize)> = [(2_000_000, 0), (2_500_000, 8), (3_000_000, 7), (3_000_000, 15), (3_500_000, 8)].to_vec();
    let auto_run = play(both, auto, &inputs(&presses), &mut NullSink);
    let plain = play(without, SessionOptions::default(), &inputs(&presses), &mut NullSink);
    assert_eq!(plain.summary().total_notes, 8);
    assert_eq!(tally(auto_run.summary()), tally(plain.summary()));
}

#[test]
fn nine_buttons_have_no_turntable_so_nothing_is_left_out() {
    let text = "#BPM 120\r\n#TOTAL 200\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00125:01010101\r\n";
    let lanes = turntable_lanes(Mode::POPN_9K);
    assert!(lanes.iter().all(|auto| !auto), "pop'n has no turntable lane");
    let session = PlaySession::new(chart(text, Mode::POPN_9K), SessionOptions { auto_lanes: lanes, ..SessionOptions::default() });
    assert!(!session.has_auto_played_notes());
    assert_eq!(session.judge().total_notes(), 8);
}
