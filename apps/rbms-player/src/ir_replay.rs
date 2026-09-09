//! Conversion between the locally saved [`Replay`] and the IR's [`ReplayData`], in both
//! directions: uploading the replay of a run just played, and playing back a replay downloaded
//! from a ranking row. The two functions are exact inverses over the fields both formats carry,
//! which the round-trip test pins.

use rbms_chart::shuffle::NoteOption;
use rbms_ir::{ChartId, GaugeType, RandomOption, ReplayData};
use rbms_judge::GaugeKind;

use crate::Stage;
use crate::ir_map::{gauge_from_name, gauge_token, ir_gauge, ir_random};
use crate::replay::{Replay, ReplayEvent};

/// Whether a finished replay download may still take over the app.
///
/// The download is polled in every stage, but applying it reloads the chart, so it may only land
/// while the user is still in song select. Landing during a load or a running play would swap the
/// chart out from under the run that is already going.
pub(crate) fn replay_download_is_applicable(stage: &Stage) -> bool {
    *stage == Stage::Select
}

/// Wire tag for the µs-resolution event stream this client writes. The server stores replays
/// opaquely and hands the tag back, so a future format can be told apart on download.
pub(crate) const REPLAY_FORMAT: &str = "rbms-us-v1";

/// Package a saved replay for upload. Everything the local file does not carry (judge window,
/// scroll mode, LN mode, build hash) is supplied by the run that produced it, so the server can
/// reproduce the run exactly.
pub(crate) fn to_ir_replay(replay: &Replay, chart: &ChartId, judge_rate: i32, constant: bool, lntype: i32, client_build_sha256: Option<String>) -> ReplayData {
    let events: Vec<rbms_ir::ReplayEvent> = replay.events.iter().map(|e| rbms_ir::ReplayEvent { t_us: e.t, lane: e.lane as u32, press: e.press }).collect();
    let duration_us = match (events.first(), events.last()) {
        (Some(first), Some(last)) => last.t_us - first.t_us,
        _ => 0,
    };
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
        judge_rate,
        scratch_auto: replay.scratch_auto,
        constant,
        gauge: Some(ir_gauge(gauge_from_name(&replay.gauge))),
        client_build_sha256,
        ..Default::default()
    }
}

/// Rebuild a locally playable replay from a downloaded one. `chart_path` and `md5` come from the
/// chart the ranking row was showing, since the payload only identifies the chart by hash.
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
        events: data.events.iter().map(|e| ReplayEvent { t: e.t_us, lane: e.lane as usize, press: e.press }).collect(),
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
            events: vec![
                ReplayEvent { t: 0, lane: 0, press: true },
                ReplayEvent { t: 1_500, lane: 0, press: false },
                ReplayEvent { t: 2_000_000, lane: 7, press: true },
            ],
        }
    }

    fn chart() -> ChartId {
        ChartId { md5: "DEADBEEF".into(), sha256: "0badc0de".into() }
    }

    #[test]
    fn upload_conversion_carries_the_run_and_its_context() {
        let local = sample();
        let data = to_ir_replay(&local, &chart(), 100, true, 2, Some("build-sha".into()));
        assert_eq!(data.format, REPLAY_FORMAT);
        assert_eq!(data.api_version, rbms_ir::API_VERSION);
        assert_eq!(data.event_count, Some(3));
        assert_eq!(data.duration_us, Some(2_000_000), "span between the first and last event");
        assert_eq!(data.seed, Some(local.seed));
        assert_eq!(data.chart.as_ref().map(|c| c.md5.as_str()), Some("DEADBEEF"));
        assert_eq!(data.lntype, 2);
        assert_eq!(data.judge_rate, 100);
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
        let data = to_ir_replay(&local, &chart(), 100, false, 0, None);
        assert_eq!(data.event_count, Some(0));
        assert_eq!(data.duration_us, Some(0));
        assert!(data.events.is_empty());
    }

    #[test]
    fn round_trip_preserves_every_shared_field() {
        let local = sample();
        let data = to_ir_replay(&local, &chart(), 125, false, 1, None);
        let back = from_ir_replay(&data, &local.chart_path, &local.md5);
        assert_eq!(back.chart_path, local.chart_path);
        assert_eq!(back.md5, local.md5);
        assert_eq!(back.mode, local.mode);
        assert_eq!(back.random, local.random);
        assert_eq!(back.seed, local.seed);
        assert_eq!(back.offset_ms, local.offset_ms);
        assert_eq!(back.scratch_auto, local.scratch_auto);
        assert_eq!(back.gauge, local.gauge);
        assert_eq!(back.events.len(), local.events.len());
        for (a, b) in local.events.iter().zip(&back.events) {
            assert_eq!((a.t, a.lane, a.press), (b.t, b.lane, b.press), "events survive the µs round trip");
        }
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
            let data = to_ir_replay(&local, &chart(), 100, false, 0, None);
            let back = from_ir_replay(&data, &local.chart_path, &local.md5);
            assert_eq!(back.random, option.label(), "{option:?} survives the wire form");
        }
    }

    #[test]
    fn a_downloaded_replay_is_applied_only_from_song_select() {
        assert!(replay_download_is_applicable(&Stage::Select));
        for stage in [Stage::Loading, Stage::Play, Stage::Result, Stage::Settings, Stage::KeyConfig, Stage::Tables, Stage::Folders] {
            assert!(!replay_download_is_applicable(&stage), "{stage:?} would have the download reload the chart under it");
        }
    }
}
