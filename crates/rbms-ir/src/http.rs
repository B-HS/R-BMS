use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::dto::*;
use crate::{IrError, ScoreServer};

/// Reference HTTP client for an rbms IR-superset backend. REST + JSON; optional bearer
/// token. The backend itself is not part of rbms — only this contract is.
pub struct HttpScoreServer {
    client: reqwest::blocking::Client,
    base: String,
    token: Option<String>,
}

impl HttpScoreServer {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(5)).build().unwrap_or_default();
        HttpScoreServer { client, base: base_url.into().trim_end_matches('/').to_string(), token }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, IrError> {
        let mut req = self.client.get(self.url(path));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().map_err(|e| IrError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(IrError::Server(status.as_u16(), resp.text().unwrap_or_default()));
        }
        resp.json::<T>().map_err(|e| IrError::Decode(e.to_string()))
    }

    fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, IrError> {
        let mut req = self.client.post(self.url(path)).json(body);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().map_err(|e| IrError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(IrError::Server(status.as_u16(), resp.text().unwrap_or_default()));
        }
        resp.json::<T>().map_err(|e| IrError::Decode(e.to_string()))
    }

    /// PUT a body and treat any 2xx as success (the endpoint may return 204 No Content).
    fn put_no_content<B: Serialize>(&self, path: &str, body: &B) -> Result<(), IrError> {
        let mut req = self.client.put(self.url(path)).json(body);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().map_err(|e| IrError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(IrError::Server(status.as_u16(), resp.text().unwrap_or_default()));
        }
        Ok(())
    }
}

impl ScoreServer for HttpScoreServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        self.get("/health")
    }

    fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        self.post("/scores", sub)
    }

    fn chart_ranking(&self, chart: &ChartId, limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        self.get(&format!("/charts/{}/ranking?limit={limit}", chart.md5))
    }

    fn player_best(&self, chart: &ChartId, player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
        self.get(&format!("/charts/{}/best?player={}", chart.md5, player.id))
    }

    fn player_profile(&self, player: &PlayerId) -> Result<PlayerProfile, IrError> {
        self.get(&format!("/players/{}", player.id))
    }

    fn rivals(&self, player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        self.get(&format!("/players/{}/rivals", player.id))
    }

    fn submit_course(&self, sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        self.post("/courses", sub)
    }

    fn upload_replay(&self, chart: &ChartId, replay: &ReplayData) -> Result<String, IrError> {
        #[derive(serde::Deserialize)]
        struct ReplayId {
            id: String,
        }
        let r: ReplayId = self.post(&format!("/charts/{}/replays", chart.md5), replay)?;
        Ok(r.id)
    }

    fn course_ranking(&self, course_hash: &str, limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        self.get(&format!("/courses/{course_hash}/ranking?limit={limit}"))
    }

    fn download_replay(&self, replay_id: &str) -> Result<ReplayData, IrError> {
        self.get(&format!("/replays/{replay_id}"))
    }

    fn get_settings(&self, player: &PlayerId, name: &str) -> Result<SettingsBlob, IrError> {
        self.get(&format!("/players/{}/settings/{name}", player.id))
    }

    fn put_settings(&self, player: &PlayerId, blob: &SettingsBlob) -> Result<(), IrError> {
        self.put_no_content(&format!("/players/{}/settings/{}", player.id, blob.name), blob)
    }

    fn register(&self, req: &AuthRequest) -> Result<AuthResponse, IrError> {
        self.post("/auth/register", req)
    }

    fn login(&self, req: &AuthRequest) -> Result<AuthResponse, IrError> {
        self.post("/auth/login", req)
    }
}
