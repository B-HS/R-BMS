//! Fixtures the contract tests post to the mock server: the chart and player they address, and a
//! fully populated score and course submission. Kept beside the tests so the request bodies the
//! assertions read are built in one place.

use crate::API_VERSION;
use crate::dto::*;

pub(super) fn chart() -> ChartId {
    ChartId { md5: "9f8e7d6c5b4a39281706f5e4d3c2b1a0".into(), sha256: "0a1b2c3d".repeat(8) }
}

pub(super) fn player() -> PlayerId {
    PlayerId { id: "gkn".into() }
}

pub(super) fn submission() -> ScoreSubmission {
    ScoreSubmission {
        api_version: API_VERSION,
        chart: chart(),
        player: player(),
        mode: "BEAT_7K".into(),
        clear: ClearLamp::Hard,
        ex_score: 1488,
        max_ex_score: 1624,
        judge: JudgeBreakdown { pgreat: 712, great: 64, epg: 400, lpg: 312, ..Default::default() },
        max_combo: 540,
        total_notes: 812,
        passnotes: 812,
        minbp: 7,
        gauge_value: 86.0,
        options: PlayOptions {
            gauge: GaugeType::Hard,
            random: RandomOption::Random,
            random_p2: None,
            scratch_auto: false,
            lntype: 1,
            input_device: "keyboard".into(),
            assist: vec![],
            option: 0,
            judge_rate: 100,
            offset_ms: 0,
            constant: false,
            hispeed: 3.0,
            lift: 0.0,
            lane_cover: 0.0,
            total_override: 0.0,
            autoplay: false,
            auto_offset: false,
            scratch_left: false,
            green_number: 310.0,
        },
        played_at: 1_700_000_000_000,
        client: "rbms/0.1".into(),
        replay_id: None,
        seed: 42,
        judge_algorithm: "Combo".into(),
        rule: String::new(),
        skin: "NORMAL".into(),
        client_build_sha256: None,
        client_platform: Some("macos-aarch64".into()),
        extra: Default::default(),
    }
}

pub(super) fn course_submission() -> CourseSubmission {
    CourseSubmission {
        api_version: API_VERSION,
        course_hash: "course-1".into(),
        player: player(),
        clear: ClearLamp::Normal,
        ex_score: 4800,
        judge: JudgeBreakdown::default(),
        max_combo: 900,
        gauge_value: 51.0,
        charts: vec![chart()],
        played_at: 1_700_000_000_000,
        lntype: 1,
        max_ex_score: 6000,
        minbp: 12,
        trophy: Some("bronzemedal".into()),
        extra: Default::default(),
    }
}
