use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dto::{ChartId, GaugeType, PlayerId, RandomOption};

/// A single µs-resolution input event in a replay: lane press/release at an absolute song time.
/// Lossless (µs), so the server-side ghost/analysis can reproduce timing exactly — unlike a byte
/// stream, this survives re-encoding and is self-describing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayEvent {
    pub t_us: i64,
    pub lane: u32,
    pub press: bool,
}

/// One replay, in both directions: the body of `POST /charts/{hash}/replays` and the payload of
/// `GET /replays/{id}`. Only `format` and `events` are load-bearing on upload; the rest describes
/// the run so the server can replay it faithfully, and every one of them carries a serde default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayData {
    #[serde(default = "crate::dto::current_api_version")]
    pub api_version: u32,
    pub format: String,
    pub events: Vec<ReplayEvent>,
    pub seed: Option<u64>,
    /// Server-side id; present when the payload came back from `GET /replays/{id}`.
    #[serde(default)]
    pub id: Option<String>,
    /// Chart the replay belongs to, resolving the path hash when the server only knows one of the
    /// two digests.
    #[serde(default)]
    pub chart: Option<ChartId>,
    /// Score this replay belongs to. Set from [`SubmitResponse::score_id`] to link the two.
    #[serde(default)]
    pub score_id: Option<String>,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub random: Option<RandomOption>,
    #[serde(default)]
    pub random_p2: Option<RandomOption>,
    #[serde(default)]
    pub lntype: i32,
    #[serde(default)]
    pub offset_ms: i32,
    #[serde(default)]
    pub judge_rate: i32,
    #[serde(default)]
    pub scratch_auto: bool,
    #[serde(default)]
    pub constant: bool,
    #[serde(default)]
    pub gauge: Option<GaugeType>,
    #[serde(default)]
    pub client_build_sha256: Option<String>,
    /// Event count as the client counted it; `None` lets the server derive it from `events`.
    #[serde(default)]
    pub event_count: Option<u32>,
    /// Span between the first and last event in microseconds; `None` lets the server derive it.
    #[serde(default)]
    pub duration_us: Option<i64>,
    /// Serialised size in bytes; `None` lets the server derive it.
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for ReplayData {
    fn default() -> ReplayData {
        ReplayData {
            api_version: crate::API_VERSION,
            format: String::new(),
            events: Vec::new(),
            seed: None,
            id: None,
            chart: None,
            score_id: None,
            mode: String::new(),
            random: None,
            random_p2: None,
            lntype: 0,
            offset_ms: 0,
            judge_rate: 0,
            scratch_auto: false,
            constant: false,
            gauge: None,
            client_build_sha256: None,
            event_count: None,
            duration_us: None,
            size: None,
            extra: HashMap::new(),
        }
    }
}

/// A row of `GET /charts/{hash}/replays`: the stored replay's metadata, without its events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayMeta {
    pub id: String,
    /// Download path as the server renders it, relative to the **deployment origin** and therefore
    /// already carrying the deployment's own API prefix (`/api/replays/rp_77`). It is *not*
    /// relative to the base `HttpScoreServer` was configured with, which normally ends in that same
    /// prefix, so joining the two double-prefixes the path. Use [`ReplayMeta::download_id`] with
    /// [`crate::ScoreServer::download_replay`] instead of string-joining this.
    pub url: String,
    pub player: PlayerId,
    pub player_name: String,
    pub chart_sha256: String,
    pub score_id: Option<String>,
    pub format: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub lntype: i32,
    #[serde(default)]
    pub event_count: u32,
    #[serde(default)]
    pub duration_us: i64,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub client_build_sha256: Option<String>,
    #[serde(default)]
    pub created_at: i64,
}

impl ReplayMeta {
    /// The id to hand [`crate::ScoreServer::download_replay`], which builds the request path from
    /// the configured base itself.
    pub fn download_id(&self) -> &str {
        &self.id
    }
}

/// What `POST /charts/{hash}/replays` answers with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayUploadResponse {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub event_count: u32,
}

#[cfg(test)]
mod tests {
    use crate::dto::*;
    use serde_json::json;

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
        let e = ReplayEvent { t_us: -1_500_000, lane: 0, press: true };
        let back: ReplayEvent = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(back.t_us, -1_500_000);
    }

    #[test]
    fn replay_data_empty_events_and_no_seed_round_trips() {
        let rd = ReplayData { format: "rbms-us-v1".into(), ..Default::default() };
        let back: ReplayData = serde_json::from_str(&serde_json::to_string(&rd).unwrap()).unwrap();
        assert!(back.events.is_empty());
        assert_eq!(back.seed, None);
    }

    #[test]
    fn replay_data_event_order_and_count_preserved() {
        let events: Vec<ReplayEvent> = (0..50).map(|i| ReplayEvent { t_us: i as i64 * 1000, lane: (i % 8) as u32, press: i % 2 == 0 }).collect();
        let rd = ReplayData { format: "rbms-us-v1".into(), events: events.clone(), seed: Some(u64::MAX), ..Default::default() };
        let back: ReplayData = serde_json::from_str(&serde_json::to_string(&rd).unwrap()).unwrap();
        assert_eq!(back.events.len(), 50);
        assert_eq!(back.events, events);
        assert_eq!(back.seed, Some(u64::MAX));
    }

    #[test]
    fn replay_data_us_resolution_distinguishes_microseconds() {
        let rd = ReplayData {
            format: "rbms-us-v1".into(),
            events: vec![ReplayEvent { t_us: 1_000_000, lane: 0, press: true }, ReplayEvent { t_us: 1_000_001, lane: 0, press: false }],
            ..Default::default()
        };
        let back: ReplayData = serde_json::from_str(&serde_json::to_string(&rd).unwrap()).unwrap();
        assert_eq!(back.events[1].t_us - back.events[0].t_us, 1, "1 µs delta preserved");
    }

    #[test]
    fn replay_data_default_stamps_the_contract_version() {
        let rd = ReplayData::default();
        assert_eq!(rd.api_version, crate::API_VERSION);
        assert!(rd.chart.is_none() && rd.score_id.is_none() && rd.id.is_none());
        assert!(rd.event_count.is_none(), "an unset count lets the server derive it");
    }

    #[test]
    fn replay_data_decodes_the_download_payload() {
        let j = json!({
            "api_version": 1, "id": "rp_1", "format": "rbms-us-v1",
            "chart": {"md5": "", "sha256": "abc"}, "score_id": "sc_1", "mode": "BEAT_7K",
            "random": "SRandom", "random_p2": null, "seed": 9, "lntype": 2, "offset_ms": -4,
            "judge_rate": 100, "scratch_auto": true, "constant": false, "client_build_sha256": null,
            "events": [{"t_us": 5, "lane": 3, "press": true}],
            "event_count": 1, "duration_us": 0, "size": 32, "extra": {}
        });
        let rd: ReplayData = serde_json::from_value(j).unwrap();
        assert_eq!(rd.id.as_deref(), Some("rp_1"));
        assert_eq!(rd.random, Some(RandomOption::SRandom));
        assert!(rd.random_p2.is_none());
        assert_eq!(rd.lntype, 2);
        assert!(rd.scratch_auto);
        assert_eq!(rd.size, Some(32));
        assert_eq!(rd.chart.expect("chart present").sha256, "abc");
    }

    #[test]
    fn replay_data_upload_body_keeps_the_score_link() {
        let rd = ReplayData { format: "rbms-us-v1".into(), score_id: Some("sc_9".into()), gauge: Some(GaugeType::Hard), ..Default::default() };
        let json = serde_json::to_string(&rd).unwrap();
        assert!(json.contains(r#""score_id":"sc_9""#), "got {json}");
        assert!(json.contains(r#""gauge":"Hard""#));
        assert!(json.contains(r#""api_version":1"#));
    }

    #[test]
    fn replay_meta_decodes_a_listing_row() {
        let j = json!({
            "id": "rp_1", "url": "/api/replays/rp_1", "player": {"id": "p"}, "player_name": "P",
            "chart_sha256": "abc", "score_id": null, "format": "rbms-us-v1", "mode": "BEAT_7K",
            "seed": 3, "lntype": 1, "event_count": 12, "duration_us": 90000, "size": 256,
            "client_build_sha256": "deadbeef", "created_at": 1700000000000_i64
        });
        let meta: ReplayMeta = serde_json::from_value(j).unwrap();
        assert_eq!(meta.url, "/api/replays/rp_1");
        assert!(meta.score_id.is_none());
        assert_eq!(meta.event_count, 12);
        assert_eq!(meta.client_build_sha256.as_deref(), Some("deadbeef"));
        assert_eq!(meta.created_at, 1_700_000_000_000);
    }

    #[test]
    fn replay_upload_response_decodes_the_created_body() {
        let r: ReplayUploadResponse = serde_json::from_value(json!({"id": "rp_1", "url": "/api/replays/rp_1", "event_count": 4})).unwrap();
        assert_eq!(r.id, "rp_1");
        assert_eq!(r.event_count, 4);
        let minimal: ReplayUploadResponse = serde_json::from_value(json!({"id": "rp_2"})).unwrap();
        assert_eq!(minimal.id, "rp_2");
        assert!(minimal.url.is_empty());
    }

    #[test]
    fn replay_meta_url_is_origin_relative_so_only_the_id_is_safe_to_reuse() {
        let meta: ReplayMeta = serde_json::from_value(json!({
            "id": "rp_77", "url": "/api/replays/rp_77", "player": {"id": "gkn"}, "player_name": "gkn",
            "chart_sha256": "abc", "score_id": null, "format": "rbms-us-v1"
        }))
        .unwrap();
        assert_eq!(meta.download_id(), "rp_77");
        let configured_base = "https://bms.hyuns.uk/api";
        assert_eq!(
            format!("{configured_base}{}", meta.url),
            "https://bms.hyuns.uk/api/api/replays/rp_77",
            "joining the configured base with `url` double-prefixes; callers must use `download_id`"
        );
    }
}
