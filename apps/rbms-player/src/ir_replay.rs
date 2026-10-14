//! Conversion between the locally saved [`Replay`] and the IR's [`ReplayData`], in both
//! directions: uploading the replay of a run just played, and playing back a replay downloaded
//! from a ranking row. The two functions are exact inverses over the fields both formats carry,
//! which the round-trip test pins.

use rbms_chart::shuffle::NoteOption;
use rbms_ir::{ChartId, GaugeType, RandomOption, ReplayData};
use rbms_judge::GaugeKind;

use crate::ir_map::{gauge_from_name, gauge_token, ir_gauge, ir_random};
use crate::replay::{REPLAY_JUDGE_WIDTH_TIER_COUNT, Replay, ReplayEvent, ReplayJudge, ReplayOffsetStep};
use crate::stage::StageId;

/// Whether a finished replay download may still take over the app.
///
/// The download is polled in every stage, but applying it reloads the chart, so it may only land
/// while the user is still in song select. Landing during a load or a running play would swap the
/// chart out from under the run that is already going.
pub(crate) fn replay_download_is_applicable(stage: StageId) -> bool {
    stage == StageId::Select
}

/// Wire tag for the µs-resolution event stream this client writes. The server stores replays
/// opaquely and hands the tag back, so a future format can be told apart on download.
pub(crate) const REPLAY_FORMAT: &str = "rbms-us-v1";

/// Microseconds in a millisecond: the wire times inputs in the first and the judge offset in the
/// second.
const US_PER_MS: i64 = 1_000;

/// Which of the three JUDGE WIDTH tiers the upload reports, the contract's `judge_rate` being one
/// number rather than six.
const UPLOADED_JUDGE_WIDTH_TIER: usize = 0;

/// The time the recorded input at `index` is uploaded at, in microseconds.
///
/// The wire's `offset_ms` is how far from its time every input is judged, one number for the
/// whole run. A run played now is judged where its inputs are -- the judge offset moves the notes
/// and not the judgement -- so its replay states no offset and is uploaded at the times it was
/// played with an offset of nothing.
///
/// A replay recorded while the offset moved the judgement can state several, the ones a run moved
/// through while its offset adjusted itself, and the wire has no place for them (the server keeps
/// none of a replay's `extra`). So an input judged at another offset than the one the run ended at
/// is uploaded moved by the difference: judged at the one offset the wire carries, it lands on the
/// instant it was judged at when it was played.
///
/// The inputs keep their order, which is the order they are judged in, so one moved past its
/// neighbour is still judged before it. What is not carried is when the run's own clock swept a
/// note the player never reached: that clock is not moved by the offset, so an input uploaded
/// later than it was played can find a note already swept that it reached in the run. It takes an
/// input as late as the last window allows, by less than the offset moved.
fn uploaded_time_us(replay: &Replay, index: usize, event: &ReplayEvent) -> i64 {
    let judged_at_ms = ReplayOffsetStep::in_force_at(&replay.judge.offset_steps, index).unwrap_or(replay.offset_ms);
    event.t + i64::from(judged_at_ms - replay.offset_ms) * US_PER_MS
}

/// Package a saved replay for upload. Everything the local file does not carry (scroll mode, LN
/// mode, build hash) is supplied by the run that produced it, so the server can reproduce the run
/// exactly; the judge widths come from the replay itself, which recorded the ones it was played
/// under, and so do the offsets its inputs were judged at ([`uploaded_time_us`]).
pub(crate) fn to_ir_replay(replay: &Replay, chart: &ChartId, constant: bool, lntype: i32, client_build_sha256: Option<String>) -> ReplayData {
    let uploaded = |(index, event): (usize, &ReplayEvent)| rbms_ir::ReplayEvent {
        t_us: uploaded_time_us(replay, index, event),
        lane: event.lane as u32,
        press: event.press,
    };
    let events: Vec<rbms_ir::ReplayEvent> = replay.events.iter().enumerate().map(uploaded).collect();
    let times = || events.iter().map(|event| event.t_us);
    let duration_us = times().max().zip(times().min()).map_or(0, |(last, first)| last - first);
    ReplayData {
        format: REPLAY_FORMAT.to_string(),
        event_count: Some(events.len() as u32),
        duration_us: Some(duration_us),
        events,
        seed: Some(replay.seed),
        chart: Some(chart.clone()),
        mode: replay.mode.clone(),
        random: Some(ir_random(NoteOption::from_str(&replay.random))),
        lntype,
        offset_ms: replay.offset_ms,
        judge_rate: replay.judge.judge_rate_key[UPLOADED_JUDGE_WIDTH_TIER],
        scratch_auto: replay.scratch_auto,
        constant,
        gauge: Some(ir_gauge(gauge_from_name(&replay.gauge))),
        client_build_sha256,
        ..Default::default()
    }
}

/// Whether the upload states every judge width, the LN MARGIN and the TOTAL a run was played under,
/// so that the replay comes back from the wire judged as it was played ([`from_ir_replay`]).
///
/// The wire carries one `judge_rate`, no LN MARGIN and no TOTAL: a run whose six widths are one
/// number, whose margin is the stock one and which was played on the chart's own TOTAL is carried
/// whole, and any other is not. Such a run is still kept and played back from its local file, which
/// records all of them.
pub(crate) fn wire_carries_judge(judge: &ReplayJudge) -> bool {
    let uploaded = judge.judge_rate_key[UPLOADED_JUDGE_WIDTH_TIER];
    judge.judge_rate_key.iter().chain(&judge.judge_rate_scratch).all(|rate| *rate == uploaded)
        && judge.longnote_margin_rate == ReplayJudge::default().longnote_margin_rate
        && !crate::overrides_total(judge.total_override)
}

/// Rebuild a locally playable replay from a downloaded one. `chart_path` and `md5` come from the
/// chart the ranking row was showing, since the payload only identifies the chart by hash.
///
/// The wire event carries no scratch direction, so a downloaded run reproduces as forward spins
/// only; a back-spin scratch it contained will play back as a re-grab of the same direction.
pub(crate) fn from_ir_replay(data: &ReplayData, chart_path: &str, md5: &str) -> Replay {
    Replay {
        chart_path: chart_path.to_string(),
        md5: md5.to_string(),
        mode: data.mode.clone(),
        random: note_option_from_ir(data.random.unwrap_or(RandomOption::Off)).label().to_string(),
        seed: data.seed.unwrap_or_default(),
        offset_ms: data.offset_ms,
        scratch_auto: data.scratch_auto,
        gauge: gauge_token(gauge_kind_from_ir(data.gauge.unwrap_or(GaugeType::Normal))).to_string(),
        judge: ReplayJudge {
            judge_rate_key: [data.judge_rate; REPLAY_JUDGE_WIDTH_TIER_COUNT],
            judge_rate_scratch: [data.judge_rate; REPLAY_JUDGE_WIDTH_TIER_COUNT],
            ..ReplayJudge::default()
        },
        events: data.events.iter().map(|e| ReplayEvent { t: e.t_us, lane: e.lane as usize, press: e.press, backward: false }).collect(),
    }
}

/// Inverse of [`ir_random`]. `Converge` has no engine equivalent, so it falls back to no shuffle
/// rather than silently reordering the lanes of a downloaded run.
pub(crate) fn note_option_from_ir(option: RandomOption) -> NoteOption {
    match option {
        RandomOption::Off | RandomOption::Converge => NoteOption::Off,
        RandomOption::Mirror => NoteOption::Mirror,
        RandomOption::Random => NoteOption::Random,
        RandomOption::RRandom => NoteOption::RRandom,
        RandomOption::SRandom => NoteOption::SRandom,
        RandomOption::Spiral => NoteOption::Rotate,
        RandomOption::HRandom => NoteOption::HRandom,
        RandomOption::AllScratch => NoteOption::AllScratch,
    }
}

/// Inverse of [`ir_gauge`]. The three course (dan) gauges the engine does not implement fold onto
/// the closest playable groove: a class gauge behaves like NORMAL, its EX variants like EX-HARD.
pub(crate) fn gauge_kind_from_ir(gauge: GaugeType) -> GaugeKind {
    match gauge {
        GaugeType::AssistEasy => GaugeKind::AssistEasy,
        GaugeType::Easy => GaugeKind::Easy,
        GaugeType::Normal | GaugeType::Class => GaugeKind::Normal,
        GaugeType::Hard => GaugeKind::Hard,
        GaugeType::ExHard | GaugeType::ExClass | GaugeType::ExHardClass => GaugeKind::ExHard,
        GaugeType::Hazard => GaugeKind::Hazard,
    }
}

#[cfg(test)]
mod tests {
    use rbms_play::{NullSink, PlaySession, PlaySummary, SessionClock, SessionOptions};

    use super::*;

    const ALL_RANDOM: [RandomOption; 9] = [
        RandomOption::Off,
        RandomOption::Mirror,
        RandomOption::Random,
        RandomOption::RRandom,
        RandomOption::SRandom,
        RandomOption::Spiral,
        RandomOption::HRandom,
        RandomOption::AllScratch,
        RandomOption::Converge,
    ];

    const ALL_GAUGE: [GaugeType; 9] = [
        GaugeType::AssistEasy,
        GaugeType::Easy,
        GaugeType::Normal,
        GaugeType::Hard,
        GaugeType::ExHard,
        GaugeType::Hazard,
        GaugeType::Class,
        GaugeType::ExClass,
        GaugeType::ExHardClass,
    ];

    fn sample() -> Replay {
        Replay {
            chart_path: "/songs/a.bms".into(),
            md5: "DEADBEEF".into(),
            mode: "BEAT_7K".into(),
            random: "S-RANDOM".into(),
            seed: 0xDEAD_BEEF_CAFE_F00D,
            offset_ms: -12,
            scratch_auto: true,
            gauge: "hard".into(),
            judge: ReplayJudge { judge_rate_key: [95; REPLAY_JUDGE_WIDTH_TIER_COUNT], ..ReplayJudge::default() },
            events: vec![
                ReplayEvent { t: 0, lane: 0, press: true, backward: false },
                ReplayEvent { t: 1_500, lane: 0, press: false, backward: false },
                ReplayEvent { t: 2_000_000, lane: 7, press: true, backward: false },
            ],
        }
    }

    fn chart() -> ChartId {
        ChartId { md5: "DEADBEEF".into(), sha256: "0badc0de".into() }
    }

    #[test]
    fn upload_conversion_carries_the_run_and_its_context() {
        let local = sample();
        let data = to_ir_replay(&local, &chart(), true, 2, Some("build-sha".into()));
        assert_eq!(data.format, REPLAY_FORMAT);
        assert_eq!(data.api_version, rbms_ir::API_VERSION);
        assert_eq!(data.event_count, Some(3));
        assert_eq!(data.duration_us, Some(2_000_000), "span between the first and last event");
        assert_eq!(data.seed, Some(local.seed));
        assert_eq!(data.chart.as_ref().map(|c| c.md5.as_str()), Some("DEADBEEF"));
        assert_eq!(data.lntype, 2);
        assert_eq!(data.judge_rate, 95, "the upload reports the widths the replay was recorded under, not the ones set now");
        assert!(data.constant);
        assert!(data.scratch_auto);
        assert_eq!(data.random, Some(RandomOption::SRandom));
        assert_eq!(data.gauge, Some(GaugeType::Hard));
        assert_eq!(data.client_build_sha256.as_deref(), Some("build-sha"));
        assert_eq!(data.score_id, None, "the submit worker links the score id");
    }

    #[test]
    fn an_empty_run_reports_a_zero_duration_instead_of_panicking() {
        let mut local = sample();
        local.events.clear();
        let data = to_ir_replay(&local, &chart(), false, 0, None);
        assert_eq!(data.event_count, Some(0));
        assert_eq!(data.duration_us, Some(0));
        assert!(data.events.is_empty());
    }

    #[test]
    fn round_trip_preserves_every_shared_field() {
        let local = sample();
        let data = to_ir_replay(&local, &chart(), false, 1, None);
        let back = from_ir_replay(&data, &local.chart_path, &local.md5);
        assert_eq!(back.chart_path, local.chart_path);
        assert_eq!(back.md5, local.md5);
        assert_eq!(back.mode, local.mode);
        assert_eq!(back.random, local.random);
        assert_eq!(back.seed, local.seed);
        assert_eq!(back.offset_ms, local.offset_ms);
        assert_eq!(back.scratch_auto, local.scratch_auto);
        assert_eq!(back.gauge, local.gauge);
        assert_eq!(back.judge.judge_rate_key, local.judge.judge_rate_key, "the one judge width the contract carries comes back on every tier");
        assert_eq!(back.judge.algorithm, ReplayJudge::default().algorithm, "the contract carries no algorithm, so playback falls back to the engine's own");
        assert_eq!(back.events.len(), local.events.len());
        for (a, b) in local.events.iter().zip(&back.events) {
            assert_eq!((a.t, a.lane, a.press), (b.t, b.lane, b.press), "events survive the µs round trip");
        }
    }

    /// A replay is uploaded only when the wire brings back the widths and the margin it was judged
    /// by: one width on all six tiers and the stock margin, as stock or widened alike.
    #[test]
    fn only_a_judge_the_wire_can_state_is_uploaded() {
        const WIDENED_PERCENT: i32 = 150;
        const OWN_TOTAL: f64 = 5_000.0;
        let uniform = |rate: i32| ReplayJudge {
            judge_rate_key: [rate; REPLAY_JUDGE_WIDTH_TIER_COUNT],
            judge_rate_scratch: [rate; REPLAY_JUDGE_WIDTH_TIER_COUNT],
            ..ReplayJudge::default()
        };
        let carried_back = |judge: ReplayJudge| {
            let local = Replay { judge, ..sample() };
            let back = from_ir_replay(&to_ir_replay(&local, &chart(), false, 0, None), &local.chart_path, &local.md5).judge;
            (back.judge_rate_key, back.judge_rate_scratch, back.longnote_margin_rate, back.total_override)
                == (local.judge.judge_rate_key, local.judge.judge_rate_scratch, local.judge.longnote_margin_rate, local.judge.total_override)
        };
        let widened_margin = ReplayJudge { longnote_margin_rate: WIDENED_PERCENT, ..ReplayJudge::default() };
        let own_total = ReplayJudge { total_override: OWN_TOTAL, ..ReplayJudge::default() };
        for judge in [ReplayJudge::default(), uniform(WIDENED_PERCENT), sample().judge, widened_margin, own_total.clone()] {
            assert_eq!(wire_carries_judge(&judge), carried_back(judge.clone()), "{judge:?}");
        }
        assert!(wire_carries_judge(&uniform(WIDENED_PERCENT)), "a judge widened on every tier alike is carried");
        assert!(!wire_carries_judge(&sample().judge), "keys narrowed and the turntable left is not");
        assert!(!wire_carries_judge(&own_total), "a TOTAL of the player's own is not, the wire having no place for it");
    }

    #[test]
    fn download_conversion_defaults_a_payload_that_omits_the_options() {
        let data = ReplayData { format: REPLAY_FORMAT.into(), ..Default::default() };
        let back = from_ir_replay(&data, "/songs/b.bms", "AABB");
        assert_eq!(back.random, NoteOption::Off.label());
        assert_eq!(back.gauge, gauge_token(GaugeKind::Normal));
        assert_eq!(back.seed, 0);
        assert!(back.events.is_empty());
        assert_eq!(back.chart_path, "/songs/b.bms");
        assert_eq!(back.md5, "AABB");
    }

    #[test]
    fn every_ir_random_maps_to_an_engine_option_and_the_shared_ones_round_trip() {
        for option in ALL_RANDOM {
            let mapped = note_option_from_ir(option);
            if option != RandomOption::Converge {
                assert_eq!(ir_random(mapped), option, "{option:?} round-trips through the engine option");
            }
        }
        assert_eq!(note_option_from_ir(RandomOption::Converge), NoteOption::Off, "an unplayable shuffle falls back to OFF");
        assert_eq!(note_option_from_ir(RandomOption::Spiral), NoteOption::Rotate, "IR SPIRAL is the engine's ROTATE");
    }

    #[test]
    fn every_ir_gauge_maps_to_a_playable_groove_and_the_shared_ones_round_trip() {
        for gauge in ALL_GAUGE {
            let mapped = gauge_kind_from_ir(gauge);
            if !matches!(gauge, GaugeType::Class | GaugeType::ExClass | GaugeType::ExHardClass) {
                assert_eq!(ir_gauge(mapped), gauge, "{gauge:?} round-trips through the engine gauge");
            }
        }
        assert_eq!(gauge_kind_from_ir(GaugeType::Class), GaugeKind::Normal);
        assert_eq!(gauge_kind_from_ir(GaugeType::ExClass), GaugeKind::ExHard);
        assert_eq!(gauge_kind_from_ir(GaugeType::ExHardClass), GaugeKind::ExHard);
    }

    #[test]
    fn every_engine_shuffle_survives_a_full_replay_round_trip() {
        for option in NoteOption::ALL {
            let mut local = sample();
            local.random = option.label().to_string();
            let data = to_ir_replay(&local, &chart(), false, 0, None);
            let back = from_ir_replay(&data, &local.chart_path, &local.md5);
            assert_eq!(back.random, option.label(), "{option:?} survives the wire form");
        }
    }

    /// How finely the song clock of a hand-played run is stepped, how long it runs past the last
    /// note and how long each press is held, in microseconds.
    const CLOCK_STEP_US: i64 = 5_000;
    const TAIL_US: i64 = 2_500_000;
    const HOLD_US: i64 = 20_000;

    /// How far ahead of its note the steady player of the adjusted run presses, in microseconds.
    const EARLY_BY_US: i64 = 40_000;

    /// The lane the adjusted run is played on.
    const PLAYED_LANE: usize = 0;

    /// A replay of [`straight_chart`] written by the build in which the judge offset moved the
    /// judgement, of a run whose offset adjusted itself: it states the twenty-seven offsets its
    /// inputs were judged at. The play crate keeps it, with what that build counted for the run.
    const STEPPED_REPLAY: &str = "../../crates/rbms-play/tests/fixtures/replay-wave12-adjusting.ron";
    const STEPPED_COUNTS: [u32; 6] = [44, 20, 0, 0, 0, 0];

    /// Sixty-four notes on one lane, a quarter of a second apart, from two seconds in.
    fn straight_chart() -> rbms_model::Model {
        let measures: String = (1..=8).map(|measure| format!("#{measure:03}11:0101010101010101\r\n")).collect();
        let chart = format!("#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n{measures}");
        rbms_chart::to_model(&rbms_parser::parse(chart.as_bytes()), rbms_model::Mode::BEAT_7K)
    }

    fn run_end_us() -> i64 {
        PlaySession::new(straight_chart(), SessionOptions::default()).last_time_us() + TAIL_US
    }

    /// A run of [`straight_chart`] played [`EARLY_BY_US`] ahead of every note with the judge offset
    /// adjusting itself: the replay the result screen saves of it, which states no offset, what the
    /// run came to, and the offset it ended at.
    fn adjusted_run() -> (Replay, PlaySummary, i32) {
        let chart = straight_chart();
        let notes = chart.timelines.iter().filter(|timeline| timeline.notes.first().is_some_and(Option::is_some));
        let presses: Vec<i64> = notes.map(|timeline| timeline.time_us - EARLY_BY_US).collect();
        let mut session = PlaySession::new(chart, SessionOptions { auto_calibration: true, ..SessionOptions::default() });
        let mut song_us = 0;
        while song_us <= run_end_us() {
            if presses.contains(&song_us) {
                session.press(PLAYED_LANE, song_us, &mut NullSink);
            }
            if presses.contains(&(song_us - HOLD_US)) {
                session.release(PLAYED_LANE, song_us);
            }
            session.tick(SessionClock::at(song_us), &mut NullSink);
            song_us += CLOCK_STEP_US;
        }
        let replay = Replay { offset_ms: 0, events: session.recorded_events().to_vec(), ..sample() };
        (replay, session.summary(), session.judge_offset_ms())
    }

    /// What `replay` comes to when it is played back the way the application plays one back, by a
    /// player whose own judge offset is `watched_at_ms`.
    fn played_back(replay: Replay, watched_at_ms: i32) -> PlaySummary {
        let judge_offset_us = i64::from(watched_at_ms) * US_PER_MS;
        let mut session = PlaySession::new(straight_chart(), SessionOptions { replay: Some(replay), judge_offset_us, ..SessionOptions::default() });
        let mut song_us = 0;
        while song_us <= run_end_us() {
            session.tick(SessionClock::at(song_us), &mut NullSink);
            song_us += CLOCK_STEP_US;
        }
        session.summary()
    }

    #[test]
    fn a_run_whose_offset_moved_is_uploaded_as_it_was_played_and_plays_back_the_same() {
        let (local, summary, ended_at_ms) = adjusted_run();
        assert_ne!(ended_at_ms, 0, "the run moved its offset as it went");
        assert_eq!(played_back(local.clone(), 0), summary, "the saved replay reproduces the run");

        let data = to_ir_replay(&local, &chart(), false, 0, None);
        assert_eq!(data.offset_ms, 0, "the inputs were judged where they are, so the wire has no offset to judge them at");
        let times: Vec<i64> = data.events.iter().map(|event| event.t_us).collect();
        assert_eq!(times, local.events.iter().map(|event| event.t).collect::<Vec<_>>(), "and they are uploaded at the times they were played");
        let back = from_ir_replay(&data, &local.chart_path, &local.md5);
        for watched_at_ms in [0, ended_at_ms, 90] {
            assert_eq!(played_back(back.clone(), watched_at_ms), summary, "watched at {watched_at_ms} ms the download is another run");
        }
    }

    #[test]
    fn a_replay_that_states_the_offsets_it_moved_through_plays_back_from_the_wire_as_it_was_judged() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(STEPPED_REPLAY);
        let local = Replay::load(&path).unwrap_or_else(|error| panic!("fixture {} is committed: {error}", path.display()));
        assert!(local.judge.offset_steps.len() > 2, "the run moved its offset as it went: {:?}", local.judge.offset_steps);
        assert_eq!(played_back(local.clone(), 0).counts, STEPPED_COUNTS, "the saved replay reproduces the run");

        let flattened = Replay { judge: ReplayJudge::default(), ..local.clone() };
        assert_ne!(played_back(flattened, 0).counts, STEPPED_COUNTS, "the same inputs judged at the last offset alone are a different run");

        let data = to_ir_replay(&local, &chart(), false, 0, None);
        assert_eq!(data.offset_ms, local.offset_ms, "the wire carries the one offset the run ended at");
        assert!(data.duration_us.is_some_and(|span| span >= 0), "the span of the uploaded inputs is never negative");
        let back = from_ir_replay(&data, &local.chart_path, &local.md5);
        assert!(back.judge.offset_steps.is_empty(), "the wire has no steps, so the download is judged at one offset");
        assert_eq!(played_back(back, 0).counts, STEPPED_COUNTS, "each input comes back where its own offset judged it");
    }

    #[test]
    fn a_run_judged_at_one_offset_is_uploaded_at_the_times_it_was_played() {
        let local = sample();
        let data = to_ir_replay(&local, &chart(), false, 0, None);
        let times: Vec<i64> = data.events.iter().map(|event| event.t_us).collect();
        assert_eq!(times, local.events.iter().map(|event| event.t).collect::<Vec<_>>());
    }

    #[test]
    fn a_downloaded_replay_is_applied_only_from_song_select() {
        assert!(replay_download_is_applicable(StageId::Select));
        for stage in StageId::ALL.into_iter().filter(|s| *s != StageId::Select) {
            assert!(!replay_download_is_applicable(stage), "{stage:?} would have the download reload the chart under it");
        }
    }
}
