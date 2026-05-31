use std::fmt;

pub mod dto;
pub mod http;
pub mod null;

pub use dto::*;
pub use http::HttpScoreServer;
pub use null::NullScoreServer;

/// Current rbms IR-superset API version. Sent in every submission so the backend can
/// evolve the contract without breaking older clients.
pub const API_VERSION: u32 = 1;

#[derive(Debug)]
pub enum IrError {
    NotConfigured,
    Network(String),
    Server(u16, String),
    Decode(String),
    Unsupported,
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrError::NotConfigured => write!(f, "score server not configured"),
            IrError::Network(s) => write!(f, "network error: {s}"),
            IrError::Server(c, s) => write!(f, "server error {c}: {s}"),
            IrError::Decode(s) => write!(f, "decode error: {s}"),
            IrError::Unsupported => write!(f, "operation not supported by server"),
        }
    }
}

impl std::error::Error for IrError {}

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

    // --- superset extensions (default to Unsupported so existing/offline servers need no change) ---

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
        // A minimal pre-superset payload (no early/late, no seed/build hash, no extended options)
        // must still deserialize, with the new fields taking their serde defaults.
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
        };
        let json = serde_json::to_string(&rd).unwrap();
        let back: ReplayData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.events.len(), 2);
        assert_eq!(back.events[0], ReplayEvent { t_us: 1_000_000, lane: 0, press: true });
        assert_eq!(back.seed, Some(7));
    }

    #[test]
    fn null_server_reports_unconfigured() {
        let s = NullScoreServer;
        assert!(matches!(s.health(), Err(IrError::NotConfigured)));
    }
}
