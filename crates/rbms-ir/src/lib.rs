pub mod dto;
pub mod error;
pub mod http;
pub mod null;
pub mod worker;

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod mock_http;

pub use dto::*;
pub use error::{ErrorBody, ErrorEnvelope, IrError};
pub use http::HttpScoreServer;
pub use null::NullScoreServer;
pub use worker::{SubmitJob, SubmitOutcome, is_replay_upload_warranted, spawn_query, spawn_submit};

/// Current rbms IR-superset API version. Sent in every submission so the backend can
/// evolve the contract without breaking older clients.
pub const API_VERSION: u32 = 1;

/// The one player id an unauthenticated submission may use. `POST /scores` without a bearer token
/// is rejected 401 for every other id, so a client that is not signed in must submit under this
/// exact string; the server then flags the score `GUEST` and leaves it out of the ranking.
pub const GUEST_PLAYER_ID: &str = "guest";

/// The rbms IR-superset score-server contract.
///
/// This is intentionally a *superset* of what BMS IRs (LR2IR, Mocha, Cinnamon, …) expose:
/// it carries both MD5 and SHA-256 chart ids, fast/slow tallies, replays, courses,
/// capability discovery and a free-form `extra` map for forward compatibility. A future
/// rbms backend implements the matching HTTP endpoints documented in
/// `docs/reference/ir-api.md`; the reference client is [`HttpScoreServer`], and
/// [`NullScoreServer`] is the offline stub.
pub trait ScoreServer: Send + Sync {
    /// Probe the server. Used both for capability discovery and the connection indicator.
    fn health(&self) -> Result<ServerInfo, IrError>;
    fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError>;
    fn chart_ranking(&self, chart: &ChartId, limit: u32) -> Result<Vec<ScoreRecord>, IrError>;
    fn player_best(&self, chart: &ChartId, player: &PlayerId) -> Result<Option<ScoreRecord>, IrError>;
    fn player_profile(&self, player: &PlayerId) -> Result<PlayerProfile, IrError>;
    fn rivals(&self, player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError>;
    fn submit_course(&self, sub: &CourseSubmission) -> Result<SubmitResponse, IrError>;
    fn upload_replay(&self, chart: &ChartId, replay: &ReplayData) -> Result<String, IrError>;

    /// Ranking for a course (concatenated charts), keyed by `course_hash`.
    fn course_ranking(&self, _course_hash: &str, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    /// Download a previously uploaded replay by id (for leaderboard ghosts / analysis).
    fn download_replay(&self, _replay_id: &str) -> Result<ReplayData, IrError> {
        Err(IrError::Unsupported)
    }
    /// Fetch an account-synced settings blob by name.
    fn get_settings(&self, _player: &PlayerId, _name: &str) -> Result<SettingsBlob, IrError> {
        Err(IrError::Unsupported)
    }
    /// Store an account-synced settings blob.
    fn put_settings(&self, _player: &PlayerId, _blob: &SettingsBlob) -> Result<(), IrError> {
        Err(IrError::Unsupported)
    }
    /// Create an account; returns a bearer token + identity.
    fn register(&self, _req: &AuthRequest) -> Result<AuthResponse, IrError> {
        Err(IrError::Unsupported)
    }
    /// Authenticate; returns a bearer token + identity.
    fn login(&self, _req: &AuthRequest) -> Result<AuthResponse, IrError> {
        Err(IrError::Unsupported)
    }
    /// Resolve the account behind the configured bearer token. Fails with
    /// [`IrError::Unauthorized`] when the token is missing or stale, which is how the app tells a
    /// live session from a saved-but-revoked one.
    fn whoami(&self) -> Result<PlayerProfile, IrError> {
        Err(IrError::Unsupported)
    }
    /// Replace the whole rival list; returns the resolved profiles the server kept.
    fn put_rivals(&self, _player: &PlayerId, _rivals: &[String]) -> Result<Vec<PlayerProfile>, IrError> {
        Err(IrError::Unsupported)
    }
    /// Ranking page with the server's paging and filters, for a rival row or a second page.
    fn chart_ranking_page(&self, _chart: &ChartId, _query: &ChartRankingQuery) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    /// Course ranking page with the same paging and filters.
    fn course_ranking_page(&self, _course_hash: &str, _query: &ChartRankingQuery) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    /// One player's submission history, newest first.
    fn player_scores(&self, _player: &PlayerId, _query: &PlayerScoresQuery) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    /// Replays stored for a chart, without their events; download one with
    /// [`ScoreServer::download_replay`].
    fn chart_replays(&self, _chart: &ChartId, _query: &ChartReplayQuery) -> Result<Vec<ReplayMeta>, IrError> {
        Err(IrError::Unsupported)
    }
    /// Contract version and build of the deployment.
    fn version(&self) -> Result<VersionInfo, IrError> {
        Err(IrError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submission_round_trips_json() {
        let sub = ScoreSubmission {
            api_version: API_VERSION,
            chart: ChartId { md5: "abc".into(), sha256: "def".into() },
            player: PlayerId { id: "p1".into() },
            mode: "BEAT_7K".into(),
            clear: ClearLamp::Hard,
            ex_score: 1488,
            max_ex_score: 1624,
            judge: JudgeBreakdown { pgreat: 712, great: 64, epg: 400, lpg: 312, avgjudge: -1500, empty_poor: 3, ..Default::default() },
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
                offset_ms: -5,
                constant: true,
                hispeed: 3.5,
                lift: 0.1,
                lane_cover: 0.2,
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
            rule: "".into(),
            skin: "NORMAL".into(),
            client_build_sha256: Some("deadbeef".into()),
            client_platform: Some("macos-aarch64".into()),
            extra: Default::default(),
        };
        let json = serde_json::to_string(&sub).unwrap();
        let back: ScoreSubmission = serde_json::from_str(&json).unwrap();
        assert_eq!(back.chart.sha256, "def");
        assert_eq!(back.clear, ClearLamp::Hard);
        assert_eq!(back.judge.pgreat, 712);
        assert_eq!(back.judge.epg + back.judge.lpg, back.judge.pgreat, "early+late split sums to total");
        assert_eq!(back.seed, 42);
        assert_eq!(back.client_build_sha256.as_deref(), Some("deadbeef"));
        assert!((back.options.hispeed - 3.5).abs() < 1e-9);
    }

    #[test]
    fn old_submission_without_superset_fields_still_decodes() {
        let json = r#"{
            "api_version":1,"chart":{"md5":"a","sha256":"b"},"player":{"id":"p"},"mode":"BEAT_7K",
            "clear":"Normal","ex_score":10,"max_ex_score":20,
            "judge":{"pgreat":5,"great":0,"good":0,"bad":0,"poor":0,"miss":0,"fast":0,"slow":0,"combobreak":0},
            "max_combo":5,"total_notes":5,"minbp":0,"gauge_value":80.0,
            "options":{"gauge":"Normal","random":"Off","random_p2":null,"scratch_auto":false,"lntype":1,"input_device":"keyboard","assist":[]},
            "played_at":1,"client":"rbms/0.1","replay_id":null
        }"#;
        let back: ScoreSubmission = serde_json::from_str(json).unwrap();
        assert_eq!(back.judge.epg, 0, "missing split defaults to 0");
        assert_eq!(back.seed, 0, "missing seed defaults to 0");
        assert!(back.client_build_sha256.is_none());
        assert_eq!(back.options.hispeed, 0.0, "missing option field defaults");
    }

    #[test]
    fn replay_data_us_events_round_trip() {
        let rd = ReplayData {
            format: "rbms-us-v1".into(),
            events: vec![ReplayEvent { t_us: 1_000_000, lane: 0, press: true }, ReplayEvent { t_us: 1_120_000, lane: 0, press: false }],
            seed: Some(7),
            ..Default::default()
        };
        let json = serde_json::to_string(&rd).unwrap();
        let back: ReplayData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.events.len(), 2);
        assert_eq!(back.events[0], ReplayEvent { t_us: 1_000_000, lane: 0, press: true });
        assert_eq!(back.seed, Some(7));
        assert_eq!(back.api_version, API_VERSION, "the default constructor stamps the contract version");
    }

    #[test]
    fn null_server_reports_unconfigured() {
        let s = NullScoreServer;
        assert!(matches!(s.health(), Err(IrError::NotConfigured)));
    }

    #[test]
    fn api_version_is_one() {
        assert_eq!(API_VERSION, 1);
    }

    #[test]
    fn api_version_round_trips_in_submission() {
        let json = serde_json::to_string(&minimal_submission()).unwrap();
        let back: ScoreSubmission = serde_json::from_str(&json).unwrap();
        assert_eq!(back.api_version, API_VERSION);
    }

    fn minimal_submission() -> ScoreSubmission {
        ScoreSubmission {
            api_version: API_VERSION,
            chart: ChartId { md5: "m".into(), sha256: "s".into() },
            player: PlayerId { id: "p".into() },
            mode: "BEAT_7K".into(),
            clear: ClearLamp::Normal,
            ex_score: 0,
            max_ex_score: 0,
            judge: JudgeBreakdown::default(),
            max_combo: 0,
            total_notes: 0,
            passnotes: 0,
            minbp: 0,
            gauge_value: 0.0,
            options: PlayOptions {
                gauge: GaugeType::Normal,
                random: RandomOption::Off,
                random_p2: None,
                scratch_auto: false,
                lntype: 0,
                input_device: "kb".into(),
                assist: vec![],
                option: 0,
                judge_rate: 0,
                offset_ms: 0,
                constant: false,
                hispeed: 0.0,
                lift: 0.0,
                lane_cover: 0.0,
                total_override: 0.0,
                autoplay: false,
                auto_offset: false,
                scratch_left: false,
                green_number: 0.0,
            },
            played_at: 0,
            client: "c".into(),
            replay_id: None,
            seed: 0,
            judge_algorithm: String::new(),
            rule: String::new(),
            skin: String::new(),
            client_build_sha256: None,
            client_platform: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn submission_full_round_trip_preserves_all_superset_fields() {
        let mut sub = minimal_submission();
        sub.seed = u64::MAX;
        sub.judge_algorithm = "Duration".into();
        sub.rule = "LR2".into();
        sub.skin = "FANCY".into();
        sub.client_build_sha256 = Some("abc123".into());
        sub.client_platform = Some("linux-x86_64".into());
        sub.options.random_p2 = Some(RandomOption::Mirror);
        let back: ScoreSubmission = serde_json::from_str(&serde_json::to_string(&sub).unwrap()).unwrap();
        assert_eq!(back.seed, u64::MAX);
        assert_eq!(back.judge_algorithm, "Duration");
        assert_eq!(back.rule, "LR2");
        assert_eq!(back.skin, "FANCY");
        assert_eq!(back.client_build_sha256.as_deref(), Some("abc123"));
        assert_eq!(back.client_platform.as_deref(), Some("linux-x86_64"));
        assert_eq!(back.options.random_p2, Some(RandomOption::Mirror));
    }

    #[test]
    fn submission_negative_played_at_round_trips_but_is_rejected_by_the_server_schema() {
        let mut sub = minimal_submission();
        sub.played_at = -1;
        let back: ScoreSubmission = serde_json::from_str(&serde_json::to_string(&sub).unwrap()).unwrap();
        assert_eq!(back.played_at, -1);
    }

    #[test]
    fn submission_nan_gauge_value_serialises_to_null_which_then_fails_to_decode() {
        let mut sub = minimal_submission();
        sub.gauge_value = f32::NAN;
        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"gauge_value\":null"), "NaN serializes to null");
        assert!(serde_json::from_str::<ScoreSubmission>(&json).is_err(), "null fails to decode into f32");
    }

    #[test]
    fn submission_extra_map_preserved() {
        let mut sub = minimal_submission();
        sub.extra.insert("k".into(), serde_json::json!({"v": 1}));
        let back: ScoreSubmission = serde_json::from_str(&serde_json::to_string(&sub).unwrap()).unwrap();
        assert_eq!(back.extra.get("k").unwrap(), &serde_json::json!({"v": 1}));
    }

    #[test]
    fn submission_replay_id_some_round_trips() {
        let mut sub = minimal_submission();
        sub.replay_id = Some("replay-99".into());
        let back: ScoreSubmission = serde_json::from_str(&serde_json::to_string(&sub).unwrap()).unwrap();
        assert_eq!(back.replay_id.as_deref(), Some("replay-99"));
    }

    #[test]
    fn ex_score_at_or_below_max_is_a_consistent_invariant() {
        let mut sub = minimal_submission();
        sub.max_ex_score = 1624;
        sub.ex_score = 1624;
        assert!(sub.ex_score <= sub.max_ex_score);
        let back: ScoreSubmission = serde_json::from_str(&serde_json::to_string(&sub).unwrap()).unwrap();
        assert_eq!(back.ex_score, back.max_ex_score);
    }
}
