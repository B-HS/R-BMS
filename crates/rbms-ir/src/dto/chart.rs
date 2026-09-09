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
/// and the `e*`/`l*` early/late split (the reference implementation's `IRScoreData` 12 fields, `epg`..`lms`), plus
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
/// keeps the reference implementation's raw option bitmask for round-tripping. All superset fields carry a serde
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

#[cfg(test)]
mod tests {
    use crate::dto::*;
    use serde::Serialize;
    use serde_json::json;

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
        assert!(serde_json::from_str::<ClearLamp>("\"normal\"").is_err());
        assert_eq!(serde_json::from_str::<ClearLamp>("\"Normal\"").unwrap(), ClearLamp::Normal);
    }

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
        let j = json!({
            "great": 4, "good": 3, "bad": 2, "poor": 1, "miss": 0,
            "fast": 9, "slow": 8, "combobreak": 7
        });
        assert!(serde_json::from_value::<JudgeBreakdown>(j).is_err());
    }

    #[test]
    fn play_options_minimal_decodes_with_defaults() {
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
        let j = json!({
            "random": "Off", "random_p2": null, "scratch_auto": false,
            "lntype": 0, "input_device": "kb", "assist": []
        });
        assert!(serde_json::from_value::<PlayOptions>(j).is_err());
    }

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
}
