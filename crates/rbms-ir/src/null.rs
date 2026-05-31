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
