use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Identifies a chart. BMS IR keys on MD5; rbms additionally carries SHA-256 (superset).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChartId {
    pub md5: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerId {
    pub id: String,
}

/// Clear lamp — superset of the standard IR lamp set.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClearLamp {
    NoPlay,
    Failed,
    AssistEasy,
    LightAssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    FullCombo,
    Perfect,
    Max,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GaugeType {
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    Hazard,
    Class,
    ExClass,
    ExHardClass,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RandomOption {
    Off,
    Mirror,
    Random,
    RRandom,
    SRandom,
    Spiral,
    HRandom,
    AllScratch,
    Converge,
}

/// Per-judge tally. The `pgreat`..`miss` totals are the basic IR fields; `fast`/`slow`/`combobreak`
/// and the `e*`/`l*` early/late split (beatoraja `IRScoreData`'s 12 fields, `epg`..`lms`), plus
/// `avgjudge` (mean signed timing, µs) and rbms-only `empty_poor`, are the superset extras. The
/// split is additive: `epg + lpg == pgreat`, etc. Every superset field has a serde default so older
/// (split-less) submissions still decode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JudgeBreakdown {
    pub pgreat: u32,
    pub great: u32,
    pub good: u32,
    pub bad: u32,
    pub poor: u32,
    pub miss: u32,
    pub fast: u32,
    pub slow: u32,
    pub combobreak: u32,
    #[serde(default)]
    pub epg: u32,
    #[serde(default)]
    pub lpg: u32,
    #[serde(default)]
    pub egr: u32,
    #[serde(default)]
    pub lgr: u32,
    #[serde(default)]
    pub egd: u32,
    #[serde(default)]
    pub lgd: u32,
    #[serde(default)]
    pub ebd: u32,
    #[serde(default)]
    pub lbd: u32,
    #[serde(default)]
    pub epr: u32,
    #[serde(default)]
    pub lpr: u32,
    #[serde(default)]
    pub ems: u32,
    #[serde(default)]
    pub lms: u32,
    #[serde(default)]
    pub avgjudge: i64,
    #[serde(default)]
    pub empty_poor: u32,
}

/// How a chart was played. The first block is basic IR; the rest is the rbms superset that lets a
/// server evaluate fairness exactly (every modifier that affects difficulty is preserved). `option`
/// keeps beatoraja's raw option bitmask for round-tripping. All superset fields carry a serde
/// default so older submissions still decode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayOptions {
    pub gauge: GaugeType,
    pub random: RandomOption,
    pub random_p2: Option<RandomOption>,
    pub scratch_auto: bool,
    pub lntype: i32,
    pub input_device: String,
    pub assist: Vec<String>,
    #[serde(default)]
    pub option: i64,
    #[serde(default)]
    pub judge_rate: i32,
    #[serde(default)]
    pub offset_ms: i32,
    #[serde(default)]
    pub constant: bool,
    #[serde(default)]
    pub hispeed: f64,
    #[serde(default)]
    pub lift: f32,
    #[serde(default)]
    pub lane_cover: f32,
    #[serde(default)]
    pub total_override: f64,
    #[serde(default)]
    pub autoplay: bool,
    #[serde(default)]
    pub auto_offset: bool,
    #[serde(default)]
    pub scratch_left: bool,
    #[serde(default)]
    pub green_number: f64,
}

/// A score upload. `api_version` + `extra` keep this forward-compatible as the IR superset grows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSubmission {
    pub api_version: u32,
    pub chart: ChartId,
    pub player: PlayerId,
    pub mode: String,
    pub clear: ClearLamp,
    pub ex_score: u32,
    pub max_ex_score: u32,
    pub judge: JudgeBreakdown,
    pub max_combo: u32,
    pub total_notes: u32,
    pub minbp: u32,
    pub gauge_value: f32,
    pub options: PlayOptions,
    pub played_at: i64,
    pub client: String,
    pub replay_id: Option<String>,
    /// RNG seed the shuffle used (lets a server reproduce the exact lane layout for verification).
    #[serde(default)]
    pub seed: u64,
    /// beatoraja `JudgeAlgorithm` ("Combo"/"Duration"/"Lowest"/"Score"); empty = client default.
    #[serde(default)]
    pub judge_algorithm: String,
    /// beatoraja `BMSPlayerRule` the run used; empty = client default.
    #[serde(default)]
    pub rule: String,
    /// Skin identifier the run used (presentation only; recorded for completeness).
    #[serde(default)]
    pub skin: String,
    /// SHA-256 of the running client binary, for build-integrity / ranked eligibility. `None` when
    /// the client could not hash itself (the server decides how to treat an unknown build).
    #[serde(default)]
    pub client_build_sha256: Option<String>,
    /// Client platform as `OS-ARCH` (e.g. `macos-aarch64`), pairing with the build hash.
    #[serde(default)]
    pub client_platform: Option<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreRecord {
    pub player: PlayerId,
    pub player_name: String,
    pub clear: ClearLamp,
    pub ex_score: u32,
    pub max_combo: u32,
    pub minbp: u32,
    pub rank: Option<u32>,
    pub played_at: i64,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub accepted: bool,
    pub rank: Option<u32>,
    pub previous_best: Option<u32>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub total_plays: u64,
    pub rank_points: f64,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub ranking: bool,
    pub player_best: bool,
    pub rivals: bool,
    pub courses: bool,
    pub replays: bool,
    pub tables: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub ir_compat: String,
    #[serde(default)]
    pub capabilities: ServerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseSubmission {
    pub api_version: u32,
    pub course_hash: String,
    pub player: PlayerId,
    pub clear: ClearLamp,
    pub ex_score: u32,
    pub judge: JudgeBreakdown,
    pub max_combo: u32,
    pub gauge_value: f32,
    pub charts: Vec<ChartId>,
    pub played_at: i64,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single µs-resolution input event in a replay: lane press/release at an absolute song time.
/// Lossless (µs), so the server-side ghost/analysis can reproduce timing exactly — unlike a byte
/// stream, this survives re-encoding and is self-describing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayEvent {
    pub t_us: i64,
    pub lane: u32,
    pub press: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayData {
    pub format: String,
    pub events: Vec<ReplayEvent>,
    pub seed: Option<u64>,
}

/// A named settings blob (settings/keyconfig/tables …) for account-side sync. `content` is the raw
/// serialised body (RON/JSON) the client wrote; the server stores it opaquely per `(player, name)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBlob {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub updated_at: i64,
}

/// Register/login request. Superset of beatoraja `IRAccount{id,password,name}` with an optional
/// `email`. `name` is used on register; ignored on login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub id: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Successful auth: a bearer token plus the resolved player identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub player: PlayerId,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---------- enum serde round-trips (externally tagged => variant name string) ----------

    fn roundtrip_enum<T>(value: T, expected_json: &str)
    where
        T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug + Copy,
    {
        let s = serde_json::to_string(&value).unwrap();
        assert_eq!(s, expected_json, "serialized form");
        let back: T = serde_json::from_str(&s).unwrap();
        assert_eq!(back, value, "round-trip equality");
    }

    #[test]
    fn clear_lamp_all_variants_round_trip() {
        let all = [
            (ClearLamp::NoPlay, "\"NoPlay\""),
            (ClearLamp::Failed, "\"Failed\""),
            (ClearLamp::AssistEasy, "\"AssistEasy\""),
            (ClearLamp::LightAssistEasy, "\"LightAssistEasy\""),
            (ClearLamp::Easy, "\"Easy\""),
            (ClearLamp::Normal, "\"Normal\""),
            (ClearLamp::Hard, "\"Hard\""),
            (ClearLamp::ExHard, "\"ExHard\""),
            (ClearLamp::FullCombo, "\"FullCombo\""),
            (ClearLamp::Perfect, "\"Perfect\""),
            (ClearLamp::Max, "\"Max\""),
        ];
        for (v, j) in all {
            roundtrip_enum(v, j);
        }
    }

    #[test]
    fn gauge_type_all_variants_round_trip() {
        let all = [
            (GaugeType::AssistEasy, "\"AssistEasy\""),
            (GaugeType::Easy, "\"Easy\""),
            (GaugeType::Normal, "\"Normal\""),
            (GaugeType::Hard, "\"Hard\""),
            (GaugeType::ExHard, "\"ExHard\""),
            (GaugeType::Hazard, "\"Hazard\""),
            (GaugeType::Class, "\"Class\""),
            (GaugeType::ExClass, "\"ExClass\""),
            (GaugeType::ExHardClass, "\"ExHardClass\""),
        ];
        for (v, j) in all {
            roundtrip_enum(v, j);
        }
    }

    #[test]
    fn random_option_all_variants_round_trip() {
        let all = [
            (RandomOption::Off, "\"Off\""),
            (RandomOption::Mirror, "\"Mirror\""),
            (RandomOption::Random, "\"Random\""),
            (RandomOption::RRandom, "\"RRandom\""),
            (RandomOption::SRandom, "\"SRandom\""),
            (RandomOption::Spiral, "\"Spiral\""),
            (RandomOption::HRandom, "\"HRandom\""),
            (RandomOption::AllScratch, "\"AllScratch\""),
            (RandomOption::Converge, "\"Converge\""),
        ];
        for (v, j) in all {
            roundtrip_enum(v, j);
        }
    }

    #[test]
    fn unknown_enum_variant_fails_to_decode() {
        assert!(serde_json::from_str::<ClearLamp>("\"Bogus\"").is_err());
        assert!(serde_json::from_str::<GaugeType>("\"NotAGauge\"").is_err());
        assert!(serde_json::from_str::<RandomOption>("\"Shuffle\"").is_err());
    }

    #[test]
    fn enum_decode_is_case_sensitive() {
        // externally-tagged serde enums match the exact variant identifier.
        assert!(serde_json::from_str::<ClearLamp>("\"normal\"").is_err());
        assert_eq!(serde_json::from_str::<ClearLamp>("\"Normal\"").unwrap(), ClearLamp::Normal);
    }

    // ---------- JudgeBreakdown ----------

    #[test]
    fn judge_breakdown_default_is_all_zero() {
        let j = JudgeBreakdown::default();
        assert_eq!(j.pgreat, 0);
        assert_eq!(j.great, 0);
        assert_eq!(j.good, 0);
        assert_eq!(j.bad, 0);
        assert_eq!(j.poor, 0);
        assert_eq!(j.miss, 0);
        assert_eq!(j.fast, 0);
        assert_eq!(j.slow, 0);
        assert_eq!(j.combobreak, 0);
        assert_eq!(j.epg, 0);
        assert_eq!(j.lpg, 0);
        assert_eq!(j.egr, 0);
        assert_eq!(j.lgr, 0);
        assert_eq!(j.egd, 0);
        assert_eq!(j.lgd, 0);
        assert_eq!(j.ebd, 0);
        assert_eq!(j.lbd, 0);
        assert_eq!(j.epr, 0);
        assert_eq!(j.lpr, 0);
        assert_eq!(j.ems, 0);
        assert_eq!(j.lms, 0);
        assert_eq!(j.avgjudge, 0);
        assert_eq!(j.empty_poor, 0);
    }

    #[test]
    fn judge_breakdown_split_invariants_hold_when_constructed() {
        // The documented additive invariant: e* + l* == total for each judge tier.
        let j = JudgeBreakdown {
            pgreat: 100,
            great: 40,
            good: 12,
            bad: 4,
            poor: 6,
            miss: 8,
            epg: 60,
            lpg: 40,
            egr: 25,
            lgr: 15,
            egd: 7,
            lgd: 5,
            ebd: 1,
            lbd: 3,
            epr: 2,
            lpr: 4,
            ems: 5,
            lms: 3,
            ..Default::default()
        };
        let s = serde_json::to_string(&j).unwrap();
        let back: JudgeBreakdown = serde_json::from_str(&s).unwrap();
        assert_eq!(back.epg + back.lpg, back.pgreat);
        assert_eq!(back.egr + back.lgr, back.great);
        assert_eq!(back.egd + back.lgd, back.good);
        assert_eq!(back.ebd + back.lbd, back.bad);
        assert_eq!(back.epr + back.lpr, back.poor);
        assert_eq!(back.ems + back.lms, back.miss);
    }

    #[test]
    fn judge_breakdown_negative_avgjudge_round_trips() {
        let j = JudgeBreakdown { avgjudge: -123_456, ..Default::default() };
        let back: JudgeBreakdown = serde_json::from_str(&serde_json::to_string(&j).unwrap()).unwrap();
        assert_eq!(back.avgjudge, -123_456);
    }

    #[test]
    fn judge_breakdown_minimal_basic_fields_default_superset_to_zero() {
        // Only the 9 basic (non-defaulted) fields are present; the 14 superset fields default to 0.
        let j = json!({
            "pgreat": 5, "great": 4, "good": 3, "bad": 2, "poor": 1, "miss": 0,
            "fast": 9, "slow": 8, "combobreak": 7
        });
        let back: JudgeBreakdown = serde_json::from_value(j).unwrap();
        assert_eq!(back.pgreat, 5);
        assert_eq!(back.fast, 9);
        assert_eq!(back.slow, 8);
        assert_eq!(back.combobreak, 7);
        assert_eq!(back.epg, 0);
        assert_eq!(back.lms, 0);
        assert_eq!(back.avgjudge, 0);
        assert_eq!(back.empty_poor, 0);
    }

    #[test]
    fn judge_breakdown_missing_basic_field_fails() {
        // `pgreat` has no serde(default): omitting a basic field must error.
        let j = json!({
            "great": 4, "good": 3, "bad": 2, "poor": 1, "miss": 0,
            "fast": 9, "slow": 8, "combobreak": 7
        });
        assert!(serde_json::from_value::<JudgeBreakdown>(j).is_err());
    }

    // ---------- PlayOptions ----------

    #[test]
    fn play_options_minimal_decodes_with_defaults() {
        // Only the 7 non-defaulted fields present.
        let j = json!({
            "gauge": "Hard", "random": "Mirror", "random_p2": null,
            "scratch_auto": true, "lntype": 2, "input_device": "midi", "assist": ["A", "B"]
        });
        let o: PlayOptions = serde_json::from_value(j).unwrap();
        assert_eq!(o.gauge, GaugeType::Hard);
        assert_eq!(o.random, RandomOption::Mirror);
        assert_eq!(o.random_p2, None);
        assert!(o.scratch_auto);
        assert_eq!(o.lntype, 2);
        assert_eq!(o.input_device, "midi");
        assert_eq!(o.assist, vec!["A".to_string(), "B".to_string()]);
        // defaulted superset fields:
        assert_eq!(o.option, 0);
        assert_eq!(o.judge_rate, 0);
        assert_eq!(o.offset_ms, 0);
        assert!(!o.constant);
        assert_eq!(o.hispeed, 0.0);
        assert_eq!(o.lift, 0.0);
        assert_eq!(o.lane_cover, 0.0);
        assert_eq!(o.total_override, 0.0);
        assert!(!o.autoplay);
        assert!(!o.auto_offset);
        assert!(!o.scratch_left);
        assert_eq!(o.green_number, 0.0);
    }

    #[test]
    fn play_options_dual_play_p2_random_round_trips() {
        let j = json!({
            "gauge": "Easy", "random": "Random", "random_p2": "SRandom",
            "scratch_auto": false, "lntype": 1, "input_device": "bm", "assist": [],
            "option": 1234567890123_i64, "offset_ms": -42, "hispeed": 4.25
        });
        let o: PlayOptions = serde_json::from_value(j).unwrap();
        assert_eq!(o.random_p2, Some(RandomOption::SRandom));
        assert_eq!(o.option, 1234567890123_i64);
        assert_eq!(o.offset_ms, -42);
        let back: PlayOptions = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
        assert_eq!(back.random_p2, Some(RandomOption::SRandom));
        assert!((back.hispeed - 4.25).abs() < 1e-12);
    }

    #[test]
    fn play_options_missing_required_field_fails() {
        // `gauge` is required (no default).
        let j = json!({
            "random": "Off", "random_p2": null, "scratch_auto": false,
            "lntype": 0, "input_device": "kb", "assist": []
        });
        assert!(serde_json::from_value::<PlayOptions>(j).is_err());
    }

    // ---------- ChartId / PlayerId ----------

    #[test]
    fn chart_id_round_trips_and_eq() {
        let c = ChartId { md5: "deadbeef".into(), sha256: "cafe".into() };
        let back: ChartId = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn chart_id_empty_strings_round_trip() {
        let c = ChartId { md5: String::new(), sha256: String::new() };
        let back: ChartId = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn player_id_round_trips() {
        let p = PlayerId { id: "player-123".into() };
        let back: PlayerId = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back, p);
    }

    // ---------- ScoreRecord ----------

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
        // `player_name` has no default.
        let j = json!({
            "player": {"id": "p"}, "clear": "Easy",
            "ex_score": 10, "max_combo": 5, "minbp": 2, "rank": null, "played_at": 7
        });
        assert!(serde_json::from_value::<ScoreRecord>(j).is_err());
    }

    // ---------- SubmitResponse ----------

    #[test]
    fn submit_response_round_trips() {
        let s = SubmitResponse {
            accepted: true,
            rank: Some(3),
            previous_best: Some(1900),
            message: Some("new best".into()),
        };
        let back: SubmitResponse = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert!(back.accepted);
        assert_eq!(back.rank, Some(3));
        assert_eq!(back.previous_best, Some(1900));
        assert_eq!(back.message.as_deref(), Some("new best"));
    }

    #[test]
    fn submit_response_all_none_round_trips() {
        let s = SubmitResponse { accepted: false, rank: None, previous_best: None, message: None };
        let back: SubmitResponse = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert!(!back.accepted);
        assert_eq!(back.rank, None);
        assert_eq!(back.previous_best, None);
        assert_eq!(back.message, None);
    }

    // ---------- PlayerProfile ----------

    #[test]
    fn player_profile_round_trips() {
        let p = PlayerProfile {
            id: "p".into(),
            name: "Carol".into(),
            total_plays: 9999,
            rank_points: 12.5,
            extra: HashMap::new(),
        };
        let back: PlayerProfile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.total_plays, 9999);
        assert!((back.rank_points - 12.5).abs() < 1e-12);
        assert!(back.extra.is_empty());
    }

    #[test]
    fn player_profile_decodes_without_extra() {
        let j = json!({"id": "p", "name": "D", "total_plays": 0, "rank_points": 0.0});
        let p: PlayerProfile = serde_json::from_value(j).unwrap();
        assert!(p.extra.is_empty());
    }

    // ---------- ServerCapabilities / ServerInfo ----------

    #[test]
    fn server_capabilities_default_is_all_false() {
        let c = ServerCapabilities::default();
        assert!(!c.ranking);
        assert!(!c.player_best);
        assert!(!c.rivals);
        assert!(!c.courses);
        assert!(!c.replays);
        assert!(!c.tables);
    }

    #[test]
    fn server_info_round_trips_with_capabilities() {
        let info = ServerInfo {
            name: "rbms-test".into(),
            version: "1.2.3".into(),
            ir_compat: "lr2".into(),
            capabilities: ServerCapabilities {
                ranking: true,
                player_best: true,
                rivals: false,
                courses: true,
                replays: false,
                tables: true,
            },
        };
        let back: ServerInfo = serde_json::from_str(&serde_json::to_string(&info).unwrap()).unwrap();
        assert_eq!(back.name, "rbms-test");
        assert!(back.capabilities.ranking);
        assert!(!back.capabilities.rivals);
        assert!(back.capabilities.tables);
    }

    #[test]
    fn server_info_missing_capabilities_defaults_to_all_false() {
        let j = json!({"name": "n", "version": "v", "ir_compat": "c"});
        let info: ServerInfo = serde_json::from_value(j).unwrap();
        assert!(!info.capabilities.ranking);
        assert!(!info.capabilities.tables);
    }

    // ---------- CourseSubmission ----------

    #[test]
    fn course_submission_round_trips_preserving_chart_order_and_count() {
        let charts = vec![
            ChartId { md5: "a".into(), sha256: "1".into() },
            ChartId { md5: "b".into(), sha256: "2".into() },
            ChartId { md5: "c".into(), sha256: "3".into() },
        ];
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

    // ---------- ReplayData / ReplayEvent ----------

    #[test]
    fn replay_event_round_trips_press_and_release() {
        let press = ReplayEvent { t_us: 0, lane: 7, press: true };
        let release = ReplayEvent { t_us: 999_999, lane: 7, press: false };
        let bp: ReplayEvent = serde_json::from_str(&serde_json::to_string(&press).unwrap()).unwrap();
        let br: ReplayEvent = serde_json::from_str(&serde_json::to_string(&release).unwrap()).unwrap();
        assert_eq!(bp, press);
        assert_eq!(br, release);
    }

    #[test]
    fn replay_event_negative_time_round_trips() {
        // t_us is i64: pre-song-start (negative) timestamps must survive.
        let e = ReplayEvent { t_us: -1_500_000, lane: 0, press: true };
        let back: ReplayEvent = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(back.t_us, -1_500_000);
    }

    #[test]
    fn replay_data_empty_events_and_no_seed_round_trips() {
        let rd = ReplayData { format: "rbms-us-v1".into(), events: vec![], seed: None };
        let back: ReplayData = serde_json::from_str(&serde_json::to_string(&rd).unwrap()).unwrap();
        assert!(back.events.is_empty());
        assert_eq!(back.seed, None);
    }

    #[test]
    fn replay_data_event_order_and_count_preserved() {
        let events: Vec<ReplayEvent> = (0..50)
            .map(|i| ReplayEvent { t_us: i as i64 * 1000, lane: (i % 8) as u32, press: i % 2 == 0 })
            .collect();
        let rd = ReplayData { format: "rbms-us-v1".into(), events: events.clone(), seed: Some(u64::MAX) };
        let back: ReplayData = serde_json::from_str(&serde_json::to_string(&rd).unwrap()).unwrap();
        assert_eq!(back.events.len(), 50);
        assert_eq!(back.events, events);
        assert_eq!(back.seed, Some(u64::MAX));
    }

    #[test]
    fn replay_data_us_resolution_distinguishes_microseconds() {
        // Two events 1 microsecond apart must remain distinct after a round-trip.
        let rd = ReplayData {
            format: "rbms-us-v1".into(),
            events: vec![
                ReplayEvent { t_us: 1_000_000, lane: 0, press: true },
                ReplayEvent { t_us: 1_000_001, lane: 0, press: false },
            ],
            seed: None,
        };
        let back: ReplayData = serde_json::from_str(&serde_json::to_string(&rd).unwrap()).unwrap();
        assert_eq!(back.events[1].t_us - back.events[0].t_us, 1, "1 µs delta preserved");
    }

    // ---------- SettingsBlob ----------

    #[test]
    fn settings_blob_round_trips() {
        let b = SettingsBlob { name: "keyconfig".into(), content: "(raw ron)".into(), updated_at: 1700 };
        let back: SettingsBlob = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back.name, "keyconfig");
        assert_eq!(back.content, "(raw ron)");
        assert_eq!(back.updated_at, 1700);
    }

    #[test]
    fn settings_blob_missing_updated_at_defaults_zero() {
        let j = json!({"name": "tables", "content": "[]"});
        let b: SettingsBlob = serde_json::from_value(j).unwrap();
        assert_eq!(b.updated_at, 0);
    }

    #[test]
    fn settings_blob_opaque_content_preserved_verbatim() {
        let raw = "{\"a\":1,\"nested\":{\"b\":[1,2,3]},\"unicode\":\"日本語\"}";
        let b = SettingsBlob { name: "settings".into(), content: raw.into(), updated_at: 0 };
        let back: SettingsBlob = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back.content, raw);
    }

    // ---------- AuthRequest / AuthResponse ----------

    #[test]
    fn auth_request_full_round_trips() {
        let r = AuthRequest {
            id: "user".into(),
            password: "secret".into(),
            email: Some("u@e.com".into()),
            name: Some("Nick".into()),
        };
        let back: AuthRequest = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.id, "user");
        assert_eq!(back.email.as_deref(), Some("u@e.com"));
        assert_eq!(back.name.as_deref(), Some("Nick"));
    }

    #[test]
    fn auth_request_minimal_defaults_email_and_name_none() {
        // Login payload: only id + password.
        let j = json!({"id": "user", "password": "pw"});
        let r: AuthRequest = serde_json::from_value(j).unwrap();
        assert_eq!(r.email, None);
        assert_eq!(r.name, None);
    }

    #[test]
    fn auth_request_missing_password_fails() {
        let j = json!({"id": "user"});
        assert!(serde_json::from_value::<AuthRequest>(j).is_err());
    }

    #[test]
    fn auth_response_round_trips() {
        let r = AuthResponse {
            token: "bearer-abc".into(),
            player: PlayerId { id: "p42".into() },
            name: "Eve".into(),
        };
        let back: AuthResponse = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.token, "bearer-abc");
        assert_eq!(back.player.id, "p42");
        assert_eq!(back.name, "Eve");
    }

    // ---------- determinism: serializing the same value twice is byte-identical ----------

    #[test]
    fn serialization_is_deterministic_for_structs_without_maps() {
        let r = AuthResponse {
            token: "t".into(),
            player: PlayerId { id: "p".into() },
            name: "n".into(),
        };
        let a = serde_json::to_string(&r).unwrap();
        let b = serde_json::to_string(&r).unwrap();
        assert_eq!(a, b);
    }

    // ---------- extra map forward-compat: unknown top-level fields ----------

    #[test]
    fn extra_map_default_is_empty_when_field_absent() {
        // ScoreRecord without extra -> empty map (verified) AND truly unknown sibling keys
        // are NOT collected into extra (serde flatten is not used here).
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
            extra: extra.clone(),
        };
        let back: ScoreRecord = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.extra.get("nested").unwrap(), &json!({"x": [1, 2, 3], "y": null}));
        assert_eq!(back.extra.get("flag").unwrap(), &json!(true));
    }
}
