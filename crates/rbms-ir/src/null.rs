use crate::dto::*;
use crate::{IrError, ScoreServer};

/// Offline stub used when no `--server` is configured. Every call reports `NotConfigured`,
/// so the UI shows a red (disconnected) indicator and play is unaffected.
pub struct NullScoreServer;

impl ScoreServer for NullScoreServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::NotConfigured)
    }
    fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::NotConfigured)
    }
    fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::NotConfigured)
    }
    fn player_best(&self, _chart: &ChartId, _player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
        Err(IrError::NotConfigured)
    }
    fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
        Err(IrError::NotConfigured)
    }
    fn rivals(&self, _player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        Err(IrError::NotConfigured)
    }
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::NotConfigured)
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::NotConfigured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn chart() -> ChartId {
        ChartId { md5: "m".into(), sha256: "s".into() }
    }

    fn player() -> PlayerId {
        PlayerId { id: "p".into() }
    }

    fn judge() -> JudgeBreakdown {
        JudgeBreakdown::default()
    }

    fn options() -> PlayOptions {
        PlayOptions {
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
        }
    }

    fn submission() -> ScoreSubmission {
        ScoreSubmission {
            api_version: crate::API_VERSION,
            chart: chart(),
            player: player(),
            mode: "BEAT_7K".into(),
            clear: ClearLamp::Normal,
            ex_score: 0,
            max_ex_score: 0,
            judge: judge(),
            max_combo: 0,
            total_notes: 0,
            passnotes: 0,
            minbp: 0,
            gauge_value: 0.0,
            options: options(),
            played_at: 0,
            client: "c".into(),
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

    fn course_submission() -> CourseSubmission {
        CourseSubmission {
            api_version: crate::API_VERSION,
            course_hash: "h".into(),
            player: player(),
            clear: ClearLamp::Normal,
            ex_score: 0,
            judge: judge(),
            max_combo: 0,
            gauge_value: 0.0,
            charts: vec![chart()],
            played_at: 0,
            lntype: 0,
            max_ex_score: 0,
            minbp: 0,
            trophy: None,
            extra: HashMap::new(),
        }
    }

    fn replay() -> ReplayData {
        ReplayData { format: "rbms-us-v1".into(), ..Default::default() }
    }

    fn settings() -> SettingsBlob {
        SettingsBlob { name: "settings".into(), content: "{}".into(), ..Default::default() }
    }

    fn auth() -> AuthRequest {
        AuthRequest::login("i", "pw")
    }

    #[test]
    fn null_health_not_configured() {
        assert!(matches!(NullScoreServer.health(), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_submit_score_not_configured() {
        assert!(matches!(NullScoreServer.submit_score(&submission()), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_chart_ranking_not_configured() {
        assert!(matches!(NullScoreServer.chart_ranking(&chart(), 10), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_player_best_not_configured() {
        assert!(matches!(NullScoreServer.player_best(&chart(), &player()), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_player_profile_not_configured() {
        assert!(matches!(NullScoreServer.player_profile(&player()), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_rivals_not_configured() {
        assert!(matches!(NullScoreServer.rivals(&player()), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_submit_course_not_configured() {
        assert!(matches!(NullScoreServer.submit_course(&course_submission()), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_upload_replay_not_configured() {
        assert!(matches!(NullScoreServer.upload_replay(&chart(), &replay()), Err(IrError::NotConfigured)));
    }

    #[test]
    fn null_course_ranking_unsupported() {
        assert!(matches!(NullScoreServer.course_ranking("h", 10), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_download_replay_unsupported() {
        assert!(matches!(NullScoreServer.download_replay("rid"), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_get_settings_unsupported() {
        assert!(matches!(NullScoreServer.get_settings(&player(), "settings"), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_put_settings_unsupported() {
        assert!(matches!(NullScoreServer.put_settings(&player(), &settings()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_register_unsupported() {
        assert!(matches!(NullScoreServer.register(&auth()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_login_unsupported() {
        assert!(matches!(NullScoreServer.login(&auth()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_whoami_unsupported() {
        assert!(matches!(NullScoreServer.whoami(), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_put_rivals_unsupported() {
        assert!(matches!(NullScoreServer.put_rivals(&player(), &["r".to_string()]), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_chart_ranking_page_unsupported() {
        assert!(matches!(NullScoreServer.chart_ranking_page(&chart(), &ChartRankingQuery::default()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_course_ranking_page_unsupported() {
        assert!(matches!(NullScoreServer.course_ranking_page("h", &ChartRankingQuery::default()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_player_scores_unsupported() {
        assert!(matches!(NullScoreServer.player_scores(&player(), &PlayerScoresQuery::default()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_chart_replays_unsupported() {
        assert!(matches!(NullScoreServer.chart_replays(&chart(), &ChartReplayQuery::default()), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_version_unsupported() {
        assert!(matches!(NullScoreServer.version(), Err(IrError::Unsupported)));
    }

    #[test]
    fn null_required_and_superset_errors_differ() {
        let required = NullScoreServer.health();
        let superset = NullScoreServer.register(&auth());
        assert!(matches!(required, Err(IrError::NotConfigured)));
        assert!(matches!(superset, Err(IrError::Unsupported)));
    }

    #[test]
    fn null_is_object_safe_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NullScoreServer>();
        let boxed: Box<dyn ScoreServer> = Box::new(NullScoreServer);
        assert!(matches!(boxed.health(), Err(IrError::NotConfigured)));
        assert!(matches!(boxed.login(&auth()), Err(IrError::Unsupported)));
    }
}
