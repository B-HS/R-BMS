use crate::dto::*;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn score_record_round_trips_with_rank_some_and_extra() {
    let mut extra = HashMap::new();
    extra.insert("dan".to_string(), json!("kaiden"));
    let r = ScoreRecord {
        player: PlayerId { id: "p".into() },
        player_name: "Alice".into(),
        clear: ClearLamp::FullCombo,
        ex_score: 2000,
        max_combo: 1000,
        minbp: 0,
        rank: Some(1),
        played_at: 42,
        lntype: 0,
        option: 0,
        total_notes: 0,
        judge: None,
        extra,
    };
    let back: ScoreRecord = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back.rank, Some(1));
    assert_eq!(back.clear, ClearLamp::FullCombo);
    assert_eq!(back.extra.get("dan").unwrap(), &json!("kaiden"));
}

#[test]
fn score_record_decodes_with_missing_extra_as_empty_map() {
    let j = json!({
        "player": {"id": "p"}, "player_name": "Bob", "clear": "Easy",
        "ex_score": 10, "max_combo": 5, "minbp": 2, "rank": null, "played_at": 7
    });
    let r: ScoreRecord = serde_json::from_value(j).unwrap();
    assert!(r.extra.is_empty());
    assert_eq!(r.rank, None);
}

#[test]
fn score_record_missing_required_field_fails() {
    let j = json!({
        "player": {"id": "p"}, "clear": "Easy",
        "ex_score": 10, "max_combo": 5, "minbp": 2, "rank": null, "played_at": 7
    });
    assert!(serde_json::from_value::<ScoreRecord>(j).is_err());
}

#[test]
fn submit_response_round_trips() {
    let s = SubmitResponse { accepted: true, rank: Some(3), previous_best: Some(1900), message: Some("new best".into()), ..Default::default() };
    let back: SubmitResponse = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(back.accepted);
    assert_eq!(back.rank, Some(3));
    assert_eq!(back.previous_best, Some(1900));
    assert_eq!(back.message.as_deref(), Some("new best"));
}

#[test]
fn submit_response_all_none_round_trips() {
    let s = SubmitResponse { accepted: false, ..Default::default() };
    let back: SubmitResponse = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(!back.accepted);
    assert_eq!(back.rank, None);
    assert_eq!(back.previous_best, None);
    assert_eq!(back.message, None);
}

#[test]
fn course_submission_round_trips_preserving_chart_order_and_count() {
    let charts =
        vec![ChartId { md5: "a".into(), sha256: "1".into() }, ChartId { md5: "b".into(), sha256: "2".into() }, ChartId { md5: "c".into(), sha256: "3".into() }];
    let cs = CourseSubmission {
        api_version: crate::API_VERSION,
        course_hash: "course-xyz".into(),
        player: PlayerId { id: "p".into() },
        clear: ClearLamp::Hard,
        ex_score: 5000,
        judge: JudgeBreakdown { pgreat: 1, ..Default::default() },
        max_combo: 2500,
        gauge_value: 71.5,
        charts: charts.clone(),
        played_at: 123,
        lntype: 2,
        max_ex_score: 6000,
        minbp: 30,
        trophy: Some("goldmedal".into()),
        extra: HashMap::new(),
    };
    let back: CourseSubmission = serde_json::from_str(&serde_json::to_string(&cs).unwrap()).unwrap();
    assert_eq!(back.charts.len(), 3, "count preserved");
    assert_eq!(back.charts, charts, "order preserved");
    assert_eq!(back.course_hash, "course-xyz");
    assert_eq!(back.clear, ClearLamp::Hard);
}

#[test]
fn course_submission_empty_charts_round_trips() {
    let cs = CourseSubmission {
        api_version: crate::API_VERSION,
        course_hash: "h".into(),
        player: PlayerId { id: "p".into() },
        clear: ClearLamp::NoPlay,
        ex_score: 0,
        judge: JudgeBreakdown::default(),
        max_combo: 0,
        gauge_value: 0.0,
        charts: vec![],
        played_at: 0,
        lntype: 0,
        max_ex_score: 0,
        minbp: 0,
        trophy: None,
        extra: HashMap::new(),
    };
    let back: CourseSubmission = serde_json::from_str(&serde_json::to_string(&cs).unwrap()).unwrap();
    assert!(back.charts.is_empty());
}

#[test]
fn course_submission_decodes_without_extra() {
    let j = json!({
        "api_version": 1, "course_hash": "h", "player": {"id": "p"},
        "clear": "Normal", "ex_score": 1, "judge": JudgeBreakdown::default(),
        "max_combo": 1, "gauge_value": 50.0, "charts": [], "played_at": 0
    });
    let cs: CourseSubmission = serde_json::from_value(j).unwrap();
    assert!(cs.extra.is_empty());
}

#[test]
fn extra_map_default_is_empty_when_field_absent() {
    let j = json!({
        "player": {"id": "p"}, "player_name": "X", "clear": "Normal",
        "ex_score": 1, "max_combo": 1, "minbp": 0, "rank": null, "played_at": 0,
        "some_future_field": 12345
    });
    let r: ScoreRecord = serde_json::from_value(j).unwrap();
    assert!(r.extra.is_empty(), "unknown sibling fields are ignored, not captured into extra");
}

#[test]
fn extra_map_round_trips_arbitrary_json() {
    let mut extra = HashMap::new();
    extra.insert("nested".to_string(), json!({"x": [1, 2, 3], "y": null}));
    extra.insert("flag".to_string(), json!(true));
    let r = ScoreRecord {
        player: PlayerId { id: "p".into() },
        player_name: "X".into(),
        clear: ClearLamp::Normal,
        ex_score: 1,
        max_combo: 1,
        minbp: 0,
        rank: None,
        played_at: 0,
        lntype: 0,
        option: 0,
        total_notes: 0,
        judge: None,
        extra: extra.clone(),
    };
    let back: ScoreRecord = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back.extra.get("nested").unwrap(), &json!({"x": [1, 2, 3], "y": null}));
    assert_eq!(back.extra.get("flag").unwrap(), &json!(true));
}

#[test]
fn score_record_basic_payload_defaults_superset_fields_to_zero_and_none() {
    let j = json!({
        "player": {"id": "p"}, "player_name": "Bob", "clear": "Easy",
        "ex_score": 10, "max_combo": 5, "minbp": 2, "rank": null, "played_at": 7
    });
    let r: ScoreRecord = serde_json::from_value(j).unwrap();
    assert_eq!(r.lntype, 0);
    assert_eq!(r.option, 0);
    assert_eq!(r.total_notes, 0);
    assert!(r.judge.is_none());
}

#[test]
fn score_record_superset_fields_round_trip_hcn_and_judge_split() {
    let mut r = ScoreRecord {
        player: PlayerId { id: "p".into() },
        player_name: "Alice".into(),
        clear: ClearLamp::Hard,
        ex_score: 1488,
        max_combo: 540,
        minbp: 7,
        rank: Some(3),
        played_at: 42,
        lntype: 2,
        option: 1_048_576,
        total_notes: 812,
        judge: Some(JudgeBreakdown { pgreat: 712, epg: 400, lpg: 312, great: 64, avgjudge: -1500, ..Default::default() }),
        extra: HashMap::new(),
    };
    r.extra.insert("dan".into(), json!("kaiden"));
    let back: ScoreRecord = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back.lntype, 2, "HCN survives the round trip");
    assert_eq!(back.option, 1_048_576);
    assert_eq!(back.total_notes, 812);
    let judge = back.judge.expect("judge preserved");
    assert_eq!(judge.epg + judge.lpg, 712);
    assert_eq!(judge.avgjudge, -1500);
}

#[test]
fn score_record_judge_decodes_from_partial_object() {
    let j = json!({
        "player": {"id": "p"}, "player_name": "X", "clear": "Normal",
        "ex_score": 1, "max_combo": 1, "minbp": 0, "rank": null, "played_at": 0,
        "lntype": 1,
        "judge": {"pgreat": 3, "great": 2, "good": 0, "bad": 0, "poor": 0, "miss": 0, "fast": 0, "slow": 0, "combobreak": 0}
    });
    let r: ScoreRecord = serde_json::from_value(j).unwrap();
    assert_eq!(r.lntype, 1);
    let judge = r.judge.expect("judge present");
    assert_eq!(judge.pgreat, 3);
    assert_eq!(judge.great, 2);
    assert_eq!(judge.epg, 0, "missing early/late split defaults to 0");
}

#[test]
fn score_flag_wire_names_mirror_the_server() {
    let expected = ["AUTOPLAY", "SCRATCH_AUTO", "ASSIST", "JUDGE_WIDTH", "TOTAL_OVERRIDE", "UNKNOWN_BUILD", "GUEST"];
    let actual: Vec<&str> = ScoreFlag::ALL.iter().map(|flag| flag.as_wire()).collect();
    assert_eq!(actual, expected, "the wire strings are the server's SCORE_FLAG values");
}

#[test]
fn score_flag_round_trips_through_its_wire_name() {
    for flag in ScoreFlag::ALL {
        assert_eq!(ScoreFlag::from_wire(flag.as_wire()), Some(flag));
        assert_eq!(flag.to_string(), flag.as_wire());
        assert!(!flag.describe().is_empty());
    }
}

#[test]
fn score_flag_rejects_an_unknown_wire_name() {
    assert_eq!(ScoreFlag::from_wire("FUTURE_FLAG"), None);
    assert_eq!(ScoreFlag::from_wire("autoplay"), None, "the match is case sensitive");
}

#[test]
fn only_unknown_build_leaves_a_score_rankable() {
    assert!(!ScoreFlag::UnknownBuild.blocks_ranking(), "the server blocks it only when it requires a trusted build");
    for flag in ScoreFlag::ALL.into_iter().filter(|flag| *flag != ScoreFlag::UnknownBuild) {
        assert!(flag.blocks_ranking(), "{flag} must block ranking");
    }
}

fn submit_response_fixture() -> serde_json::Value {
    json!({
        "accepted": true,
        "rank": 3,
        "previous_best": 1400,
        "message": "recorded (unranked: assist,guest)",
        "ranked": false,
        "flags": ["ASSIST", "GUEST", "FUTURE_FLAG"],
        "is_new_best": false,
        "score_id": "sc_abc",
        "extra": {"verified": false}
    })
}

#[test]
fn submit_response_decodes_every_superset_field() {
    let r: SubmitResponse = serde_json::from_value(submit_response_fixture()).unwrap();
    assert!(r.accepted);
    assert!(!r.ranked);
    assert!(!r.is_new_best);
    assert_eq!(r.score_id.as_deref(), Some("sc_abc"));
    assert_eq!(r.flags.len(), 3);
    assert_eq!(r.extra.get("verified").unwrap(), &json!(false));
}

#[test]
fn submit_response_classifies_known_and_unknown_flags() {
    let r: SubmitResponse = serde_json::from_value(submit_response_fixture()).unwrap();
    assert_eq!(r.known_flags(), vec![ScoreFlag::Assist, ScoreFlag::Guest]);
    assert_eq!(r.unknown_flags(), vec!["FUTURE_FLAG"]);
    assert_eq!(r.unranked_reasons(), vec![ScoreFlag::Assist, ScoreFlag::Guest]);
    assert!(r.has_flag(ScoreFlag::Assist));
    assert!(!r.has_flag(ScoreFlag::Autoplay));
    assert_eq!(r.unranked_summary().as_deref(), Some("assist options, guest submission"));
}

#[test]
fn submit_response_with_only_a_non_blocking_flag_is_not_unranked() {
    let r = SubmitResponse { accepted: true, flags: vec!["UNKNOWN_BUILD".into()], ..Default::default() };
    assert!(r.unranked_reasons().is_empty());
    assert!(r.unranked_summary().is_none());
    assert!(r.has_flag(ScoreFlag::UnknownBuild));
}

#[test]
fn submit_response_from_a_pre_superset_server_defaults_the_new_fields() {
    let j = json!({"accepted": true, "rank": 1, "previous_best": null, "message": "saved"});
    let r: SubmitResponse = serde_json::from_value(j).unwrap();
    assert!(!r.ranked);
    assert!(r.flags.is_empty());
    assert!(!r.is_new_best);
    assert!(r.score_id.is_none());
    assert!(r.extra.is_empty());
}

#[test]
fn submit_response_round_trips_the_superset_fields() {
    let r: SubmitResponse = serde_json::from_value(submit_response_fixture()).unwrap();
    let back: SubmitResponse = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(back.flags, r.flags);
    assert_eq!(back.score_id, r.score_id);
    assert_eq!(back.ranked, r.ranked);
}

#[test]
fn course_submission_carries_the_full_server_shape() {
    let cs = CourseSubmission {
        api_version: crate::API_VERSION,
        course_hash: "course-1".into(),
        player: PlayerId { id: "p".into() },
        clear: ClearLamp::ExHard,
        ex_score: 4800,
        judge: JudgeBreakdown::default(),
        max_combo: 900,
        gauge_value: 51.0,
        charts: vec![],
        played_at: 1,
        lntype: 2,
        max_ex_score: 6000,
        minbp: 12,
        trophy: Some("bronzemedal".into()),
        extra: HashMap::new(),
    };
    let json = serde_json::to_string(&cs).unwrap();
    assert!(json.contains(r#""lntype":2"#), "got {json}");
    assert!(json.contains(r#""max_ex_score":6000"#));
    assert!(json.contains(r#""minbp":12"#));
    assert!(json.contains(r#""trophy":"bronzemedal""#));
    let back: CourseSubmission = serde_json::from_str(&json).unwrap();
    assert_eq!(back.trophy.as_deref(), Some("bronzemedal"));
}

#[test]
fn course_submission_without_the_superset_fields_defaults_them() {
    let j = json!({
        "api_version": 1, "course_hash": "h", "player": {"id": "p"}, "clear": "Normal",
        "ex_score": 1, "judge": JudgeBreakdown::default(), "max_combo": 1, "gauge_value": 1.0,
        "charts": [], "played_at": 0
    });
    let cs: CourseSubmission = serde_json::from_value(j).unwrap();
    assert_eq!(cs.lntype, 0);
    assert_eq!(cs.max_ex_score, 0);
    assert_eq!(cs.minbp, 0);
    assert!(cs.trophy.is_none());
}

fn submission_for_bounds() -> ScoreSubmission {
    ScoreSubmission {
        api_version: crate::API_VERSION,
        chart: ChartId { md5: "m".into(), sha256: "s".into() },
        player: PlayerId { id: crate::GUEST_PLAYER_ID.into() },
        mode: "BEAT_7K".into(),
        clear: ClearLamp::Failed,
        ex_score: 10,
        max_ex_score: 1624,
        judge: JudgeBreakdown::default(),
        max_combo: 4,
        total_notes: 812,
        passnotes: 120,
        minbp: 3,
        gauge_value: 12.5,
        options: PlayOptions {
            gauge: GaugeType::Normal,
            random: RandomOption::Off,
            random_p2: None,
            scratch_auto: false,
            lntype: 0,
            input_device: "keyboard".into(),
            assist: vec![],
            option: 0,
            judge_rate: 100,
            offset_ms: 0,
            constant: false,
            hispeed: 1.0,
            lift: 0.0,
            lane_cover: 0.0,
            total_override: 0.0,
            autoplay: false,
            auto_offset: false,
            scratch_left: false,
            green_number: 0.0,
        },
        played_at: 1,
        client: "rbms/0.1.0".into(),
        replay_id: None,
        seed: 0,
        judge_algorithm: String::new(),
        rule: String::new(),
        skin: String::new(),
        client_build_sha256: None,
        client_platform: None,
        extra: HashMap::new(),
    }
}

#[test]
fn submission_passnotes_is_sent_on_the_wire_and_round_trips() {
    let sub = submission_for_bounds();
    let json = serde_json::to_string(&sub).unwrap();
    assert!(json.contains(r#""passnotes":120"#), "got {json}");
    let back: ScoreSubmission = serde_json::from_str(&json).unwrap();
    assert_eq!(back.passnotes, 120);
    assert_eq!(back.total_notes, 812, "passnotes is independent of total_notes");
}

#[test]
fn submission_without_passnotes_defaults_it_to_zero() {
    let mut value = serde_json::to_value(submission_for_bounds()).unwrap();
    value.as_object_mut().unwrap().remove("passnotes");
    let back: ScoreSubmission = serde_json::from_value(value).unwrap();
    assert_eq!(back.passnotes, 0);
}

#[test]
fn clamping_lifts_a_pre_epoch_played_at_to_the_server_minimum() {
    let mut sub = submission_for_bounds();
    sub.played_at = -1;
    sub.clamp_to_server_bounds();
    assert_eq!(sub.played_at, MIN_PLAYED_AT_MS, "a negative stamp is a 400 on the server");
}

#[test]
fn clamping_replaces_a_non_finite_gauge_value_that_would_serialise_to_null() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut sub = submission_for_bounds();
        sub.gauge_value = bad;
        sub.clamp_to_server_bounds();
        assert_eq!(sub.gauge_value, FALLBACK_GAUGE_VALUE);
        let json = serde_json::to_string(&sub).unwrap();
        assert!(!json.contains(r#""gauge_value":null"#), "got {json}");
    }
}

#[test]
fn clamping_leaves_an_in_range_submission_untouched() {
    let mut sub = submission_for_bounds();
    sub.clamp_to_server_bounds();
    assert_eq!(sub.played_at, 1);
    assert!((sub.gauge_value - 12.5).abs() < f32::EPSILON);
}

#[test]
fn clamping_a_course_submission_applies_the_same_two_bounds() {
    let mut cs = CourseSubmission {
        api_version: crate::API_VERSION,
        course_hash: "h".into(),
        player: PlayerId { id: "p".into() },
        clear: ClearLamp::Failed,
        ex_score: 1,
        judge: JudgeBreakdown::default(),
        max_combo: 1,
        gauge_value: f32::NAN,
        charts: vec![],
        played_at: -5,
        lntype: 0,
        max_ex_score: 0,
        minbp: 0,
        trophy: None,
        extra: HashMap::new(),
    };
    cs.clamp_to_server_bounds();
    assert_eq!(cs.played_at, MIN_PLAYED_AT_MS);
    assert_eq!(cs.gauge_value, FALLBACK_GAUGE_VALUE);
}
