use std::time::Duration;

use reqwest::blocking::{RequestBuilder, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::dto::*;
use crate::error::STATUS_CONFLICT;
use crate::{IrError, ScoreServer};

/// Wall-clock budget for a single IR request. Every call (health poll, submission, ranking
/// fetch) shares it, so a stalled backend can never block a worker thread indefinitely.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Reference HTTP client for an rbms IR-superset backend. REST + JSON; optional bearer
/// token. The backend itself is not part of rbms — only this contract is.
///
/// The client is built with [`REQUEST_TIMEOUT`]. If the builder fails (no TLS backend, for
/// instance) the server enters a *degraded* state instead of silently falling back to a
/// client without a timeout: [`HttpScoreServer::degraded`] reports the reason and every
/// request fails fast with [`IrError::Network`].
///
/// Identifiers are interpolated into the path and query verbatim, without percent-encoding, so
/// every player id, settings-blob name, replay id and course hash handed to this client must
/// already be URL-safe. A value containing `/`, `?`, `&` or `#` addresses a different resource
/// than the caller intended.
pub struct HttpScoreServer {
    client: Result<reqwest::blocking::Client, String>,
    base: String,
    token: Option<String>,
}

impl HttpScoreServer {
    /// Build a client, reporting a builder failure as [`IrError::Network`].
    pub fn try_new(base_url: impl Into<String>, token: Option<String>) -> Result<Self, IrError> {
        Self::try_with_timeout(base_url, token, REQUEST_TIMEOUT)
    }

    /// Like [`HttpScoreServer::try_new`] with an explicit per-request timeout.
    pub fn try_with_timeout(base_url: impl Into<String>, token: Option<String>, timeout: Duration) -> Result<Self, IrError> {
        match Self::build_client(timeout) {
            Ok(client) => Ok(HttpScoreServer { client: Ok(client), base: Self::normalise_base(base_url), token }),
            Err(e) => Err(IrError::Network(e)),
        }
    }

    /// Infallible constructor kept for callers that cannot handle a build failure. A failure is
    /// recorded instead of panicking or downgrading to an unbounded client; see
    /// [`HttpScoreServer::degraded`].
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        HttpScoreServer { client: Self::build_client(REQUEST_TIMEOUT), base: Self::normalise_base(base_url), token }
    }

    /// Same connection, different credentials — how the app swaps in the token it just got from
    /// `login`/`register`, or drops it on sign-out, without rebuilding the HTTP client.
    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    /// The bearer token every authenticated call sends, when one is configured.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// The normalised base URL every path is appended to.
    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// The one place a client is built, so no constructor can end up without a request timeout.
    fn build_client(timeout: Duration) -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder().timeout(timeout).build().map_err(|e| e.to_string())
    }

    /// `Some(reason)` when the HTTP client could not be built and every request will fail.
    pub fn degraded(&self) -> Option<&str> {
        self.client.as_ref().err().map(String::as_str)
    }

    fn normalise_base(base_url: impl Into<String>) -> String {
        base_url.into().trim_end_matches('/').to_string()
    }

    fn client(&self) -> Result<&reqwest::blocking::Client, IrError> {
        self.client.as_ref().map_err(|e| IrError::Network(format!("http client unavailable: {e}")))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn authorized(&self, req: RequestBuilder) -> RequestBuilder {
        match &self.token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    fn send(&self, req: RequestBuilder) -> Result<Response, IrError> {
        self.authorized(req).send().map_err(|e| IrError::Network(e.to_string()))
    }

    fn succeeding(resp: Response) -> Result<Response, IrError> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        Err(IrError::from_status(status.as_u16(), resp.text().unwrap_or_default()))
    }

    fn decode<T: DeserializeOwned>(resp: Response) -> Result<T, IrError> {
        resp.json::<T>().map_err(|e| IrError::Decode(e.to_string()))
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, IrError> {
        let resp = self.send(self.client()?.get(self.url(path)))?;
        Self::decode(Self::succeeding(resp)?)
    }

    fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, IrError> {
        let resp = self.send(self.client()?.post(self.url(path)).json(body))?;
        Self::decode(Self::succeeding(resp)?)
    }

    fn put<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, IrError> {
        let resp = self.send(self.client()?.put(self.url(path)).json(body))?;
        Self::decode(Self::succeeding(resp)?)
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
        let uploaded: ReplayUploadResponse = self.post(&format!("/charts/{}/replays", chart.md5), replay)?;
        Ok(uploaded.id)
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

    /// Stores `blob` under `(player, blob.name)` and reports the stamp the row now carries.
    ///
    /// The success body is `{ "updated_at": … }`; a deployment that predates it answers
    /// `204 No Content`, and the sent stamp is echoed back with
    /// [`SettingsPutResult::from_server`] false so the caller knows to read the row back. When
    /// `blob.base_updated_at` is set and the server's copy has moved on, the 409 body is decoded
    /// into [`IrError::SettingsConflict`] so the caller can merge without re-fetching.
    fn put_settings(&self, player: &PlayerId, blob: &SettingsBlob) -> Result<SettingsPutResult, IrError> {
        let path = format!("/players/{}/settings/{}", player.id, blob.name);
        let body = SettingsPutRequest::from_blob(blob);
        let resp = self.send(self.client()?.put(self.url(&path)).json(&body))?;
        let status = resp.status().as_u16();
        if status == STATUS_CONFLICT {
            let text = resp.text().unwrap_or_default();
            return Err(match serde_json::from_str::<SettingsConflict>(&text) {
                Ok(conflict) => IrError::SettingsConflict(Box::new(conflict)),
                Err(_) => IrError::Conflict(text),
            });
        }
        let text = Self::succeeding(resp)?.text().unwrap_or_default();
        let stored = serde_json::from_str::<SettingsPutResponse>(&text).ok().and_then(|response| response.updated_at);
        Ok(match stored {
            Some(updated_at) => SettingsPutResult { updated_at, from_server: true },
            None => SettingsPutResult { updated_at: blob.updated_at, from_server: false },
        })
    }

    fn register(&self, req: &AuthRequest) -> Result<AuthResponse, IrError> {
        self.post("/auth/register", req)
    }

    fn login(&self, req: &AuthRequest) -> Result<AuthResponse, IrError> {
        self.post("/auth/login", req)
    }

    fn whoami(&self) -> Result<PlayerProfile, IrError> {
        self.get("/auth/me")
    }

    fn put_rivals(&self, player: &PlayerId, rivals: &[String]) -> Result<Vec<PlayerProfile>, IrError> {
        let body = RivalPutRequest { rivals: rivals.to_vec() };
        self.put(&format!("/players/{}/rivals", player.id), &body)
    }

    fn chart_ranking_page(&self, chart: &ChartId, query: &ChartRankingQuery) -> Result<Vec<ScoreRecord>, IrError> {
        self.get(&format!("/charts/{}/ranking?{}", chart.md5, query.to_query_string()))
    }

    fn course_ranking_page(&self, course_hash: &str, query: &ChartRankingQuery) -> Result<Vec<ScoreRecord>, IrError> {
        self.get(&format!("/courses/{course_hash}/ranking?{}", query.to_query_string()))
    }

    fn player_scores(&self, player: &PlayerId, query: &PlayerScoresQuery) -> Result<Vec<ScoreRecord>, IrError> {
        self.get(&format!("/players/{}/scores?{}", player.id, query.to_query_string()))
    }

    fn chart_replays(&self, chart: &ChartId, query: &ChartReplayQuery) -> Result<Vec<ReplayMeta>, IrError> {
        self.get(&format!("/charts/{}/replays?{}", chart.md5, query.to_query_string()))
    }

    fn version(&self) -> Result<VersionInfo, IrError> {
        self.get("/version")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_http::{TEST_TIMEOUT, TestServer, no_content, ok, response};
    use std::net::TcpListener;

    /// Delay the "slow server" case uses; must exceed `TEST_TIMEOUT`.
    const SLOW_SERVER_DELAY: Duration = Duration::from_millis(1500);
    /// Delay the ignored default-constructor case uses; must exceed `REQUEST_TIMEOUT`.
    const UNREACHABLE_DELAY: Duration = Duration::from_secs(30);
    /// Stand-in reason for a `reqwest` builder failure (no TLS backend, for instance).
    const BUILDER_FAILURE: &str = "no tls backend";

    fn chart() -> ChartId {
        ChartId { md5: "abc123".into(), sha256: "def456".into() }
    }

    fn course_submission() -> CourseSubmission {
        CourseSubmission {
            api_version: crate::API_VERSION,
            course_hash: "ch1".into(),
            player: PlayerId { id: "p1".into() },
            clear: ClearLamp::Normal,
            ex_score: 4000,
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

    fn auth_request() -> AuthRequest {
        AuthRequest { name: Some("P1".into()), ..AuthRequest::login("p1", "pw") }
    }

    fn submission() -> ScoreSubmission {
        ScoreSubmission {
            api_version: crate::API_VERSION,
            chart: chart(),
            player: PlayerId { id: "p1".into() },
            mode: "BEAT_7K".into(),
            clear: ClearLamp::Hard,
            ex_score: 1488,
            max_ex_score: 1624,
            judge: JudgeBreakdown::default(),
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
                green_number: 0.0,
            },
            played_at: 1_700_000_000_000,
            client: "rbms/0.1".into(),
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
    fn try_new_builds_a_client_that_is_not_degraded() {
        let server = HttpScoreServer::try_new("http://127.0.0.1:1", None).expect("builder succeeds");
        assert!(server.degraded().is_none());
    }

    #[test]
    fn new_reports_no_degradation_on_a_healthy_platform() {
        let server = HttpScoreServer::new("http://127.0.0.1:1", None);
        assert!(server.degraded().is_none(), "the infallible constructor must not silently drop the timeout");
    }

    #[test]
    fn request_timeout_pins_the_five_second_default() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(5));
    }

    /// Long-running counterpart to `a_server_slower_than_the_timeout_maps_to_network_error`: it
    /// exercises the default constructor, whose budget is [`REQUEST_TIMEOUT`]. Ignored by default
    /// because a passing run costs five seconds; run with `--ignored` when touching the
    /// constructors.
    #[test]
    #[ignore = "waits out the five-second default timeout"]
    fn a_client_from_the_default_constructor_gives_up_on_a_stalled_server() {
        let server = TestServer::spawn_with_delay(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)], UNREACHABLE_DELAY);
        let client = HttpScoreServer::new(&server.base, None);
        let started = std::time::Instant::now();
        let err = client.health().expect_err("the default constructor must apply a timeout");
        assert!(matches!(err, IrError::Network(_)), "got {err:?}");
        assert!(started.elapsed() < UNREACHABLE_DELAY, "the call returned before the server replied");
    }

    fn degraded_server() -> HttpScoreServer {
        HttpScoreServer { client: Err(BUILDER_FAILURE.into()), base: "http://127.0.0.1:1".into(), token: None }
    }

    #[test]
    fn a_failed_builder_is_reported_by_degraded() {
        assert_eq!(degraded_server().degraded(), Some(BUILDER_FAILURE));
    }

    #[test]
    fn a_degraded_client_fails_every_request_without_touching_the_network() {
        let server = degraded_server();
        let blob = SettingsBlob { name: "keyconfig".into(), content: "()".into(), ..Default::default() };
        let errors = [
            server.health().expect_err("health cannot run without a client"),
            server.submit_score(&submission()).expect_err("submit cannot run without a client"),
            server.put_settings(&PlayerId { id: "p1".into() }, &blob).expect_err("put cannot run without a client"),
        ];
        for err in errors {
            match err {
                IrError::Network(msg) => {
                    assert!(msg.contains("http client unavailable"), "got {msg}");
                    assert!(msg.contains(BUILDER_FAILURE), "the builder reason is preserved, got {msg}");
                }
                other => panic!("expected Network(_), got {other:?}"),
            }
        }
    }

    #[test]
    fn health_parses_server_info_and_capabilities() {
        let server = TestServer::spawn(vec![ok(
            r#"{"name":"rbms-ir","version":"0.1.0","ir_compat":"superset-1","capabilities":{"ranking":true,"player_best":true,"rivals":false,"courses":false,"replays":true,"tables":false}}"#,
        )]);
        let info = server.client().health().expect("health succeeds");
        assert!(server.next_request().starts_with("GET /health HTTP/1.1"));
        assert_eq!(info.name, "rbms-ir");
        assert_eq!(info.ir_compat, "superset-1");
        assert!(info.capabilities.ranking);
        assert!(!info.capabilities.tables);
    }

    #[test]
    fn health_tolerates_a_response_without_capabilities() {
        let server = TestServer::spawn(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)]);
        let info = server.client().health().expect("health succeeds");
        let _ = server.next_request();
        assert!(!info.capabilities.ranking, "absent capabilities default to all-false");
    }

    #[test]
    fn base_url_trailing_slash_does_not_double_the_path_separator() {
        let server = TestServer::spawn(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)]);
        let client = HttpScoreServer::try_with_timeout(format!("{}/", server.base), None, TEST_TIMEOUT).expect("client builds");
        client.health().expect("health succeeds");
        assert!(server.next_request().starts_with("GET /health HTTP/1.1"));
    }

    #[test]
    fn submit_score_posts_the_submission_json_and_parses_the_response() {
        let server = TestServer::spawn(vec![ok(r#"{"accepted":true,"rank":3,"previous_best":1400,"message":"ok"}"#)]);
        let resp = server.client().submit_score(&submission()).expect("submit succeeds");
        let req = server.next_request();
        assert!(req.starts_with("POST /scores HTTP/1.1"));
        assert!(req.to_lowercase().contains("content-type: application/json"), "the submission is sent as JSON, got {req}");
        assert!(req.contains(r#""api_version":1"#), "body carries the contract version");
        assert!(req.contains(r#""ex_score":1488"#), "body carries the submission");
        assert!(req.contains(r#""sha256":"def456""#), "body carries the chart identity, not only the md5");
        assert!(req.contains(r#""lntype":1"#), "body carries the play options");
        assert!(resp.accepted);
        assert_eq!(resp.rank, Some(3));
        assert_eq!(resp.previous_best, Some(1400));
        assert_eq!(resp.message.as_deref(), Some("ok"));
    }

    #[test]
    fn chart_ranking_sends_the_limit_and_parses_rows() {
        let server = TestServer::spawn(vec![ok(
            r#"[{"player":{"id":"a"},"player_name":"A","clear":"Hard","ex_score":1500,"max_combo":500,"minbp":3,"rank":1,"played_at":10,"lntype":2,"total_notes":812},
                {"player":{"id":"b"},"player_name":"B","clear":"Normal","ex_score":1200,"max_combo":300,"minbp":9,"rank":2,"played_at":20}]"#,
        )]);
        let rows = server.client().chart_ranking(&chart(), 50).expect("ranking succeeds");
        assert!(server.next_request().starts_with("GET /charts/abc123/ranking?limit=50 HTTP/1.1"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ex_score, 1500);
        assert_eq!(rows[0].lntype, 2);
        assert_eq!(rows[0].total_notes, 812);
        assert_eq!(rows[1].lntype, 0, "a basic ranking row defaults the superset fields");
        assert!(rows[1].judge.is_none());
    }

    #[test]
    fn player_best_decodes_json_null_as_no_record() {
        let server = TestServer::spawn(vec![ok("null")]);
        let best = server.client().player_best(&chart(), &PlayerId { id: "p1".into() }).expect("request succeeds");
        assert!(server.next_request().starts_with("GET /charts/abc123/best?player=p1 HTTP/1.1"));
        assert!(best.is_none());
    }

    #[test]
    fn upload_replay_returns_the_server_side_id() {
        let server = TestServer::spawn(vec![ok(r#"{"id":"replay-99"}"#)]);
        let events = vec![ReplayEvent { t_us: 1, lane: 0, press: true }];
        let replay = ReplayData { format: "rbms-us-v1".into(), events, seed: Some(7), ..Default::default() };
        let id = server.client().upload_replay(&chart(), &replay).expect("upload succeeds");
        assert!(server.next_request().starts_with("POST /charts/abc123/replays HTTP/1.1"));
        assert_eq!(id, "replay-99");
    }

    #[test]
    fn put_settings_reads_back_the_stamp_the_server_stored() {
        let server = TestServer::spawn(vec![ok(r#"{"updated_at":1700000000042}"#)]);
        let blob = SettingsBlob { name: "keyconfig".into(), content: "()".into(), updated_at: 1, ..Default::default() };
        let stored = server.client().put_settings(&PlayerId { id: "p1".into() }, &blob).expect("put succeeds");
        assert!(server.next_request().starts_with("PUT /players/p1/settings/keyconfig HTTP/1.1"));
        assert_eq!(stored.updated_at, 1_700_000_000_042, "the lock base is the server's stamp, not the one the client sent");
        assert!(stored.from_server);
    }

    #[test]
    fn put_settings_accepts_a_204_no_content_reply_from_an_older_server() {
        let server = TestServer::spawn(vec![no_content()]);
        let blob = SettingsBlob { name: "keyconfig".into(), content: "()".into(), updated_at: 1_700_000_000_000, ..Default::default() };
        let stored = server.client().put_settings(&PlayerId { id: "p1".into() }, &blob).expect("put succeeds");
        assert!(server.next_request().starts_with("PUT /players/p1/settings/keyconfig HTTP/1.1"));
        assert_eq!(stored.updated_at, 1_700_000_000_000, "an empty body falls back to the stamp the client sent");
        assert!(!stored.from_server, "the caller has to read the row back to learn the real stamp");
    }

    #[test]
    fn player_profile_requests_the_player_resource() {
        let server = TestServer::spawn(vec![ok(r#"{"id":"p1","name":"P1","total_plays":3,"rank_points":12.5}"#)]);
        let profile = server.client().player_profile(&PlayerId { id: "p1".into() }).expect("profile succeeds");
        assert!(server.next_request().starts_with("GET /players/p1 HTTP/1.1"));
        assert_eq!(profile.name, "P1");
        assert_eq!(profile.total_plays, 3);
    }

    #[test]
    fn rivals_requests_the_rivals_collection_under_the_player() {
        let server = TestServer::spawn(vec![ok(r#"[{"id":"r1","name":"R1","total_plays":0,"rank_points":0.0}]"#)]);
        let rivals = server.client().rivals(&PlayerId { id: "p1".into() }).expect("rivals succeeds");
        assert!(server.next_request().starts_with("GET /players/p1/rivals HTTP/1.1"));
        assert_eq!(rivals.len(), 1);
        assert_eq!(rivals[0].id, "r1");
    }

    #[test]
    fn submit_course_posts_to_the_courses_collection() {
        let server = TestServer::spawn(vec![ok(r#"{"accepted":true,"rank":null,"previous_best":null,"message":null}"#)]);
        let resp = server.client().submit_course(&course_submission()).expect("course submit succeeds");
        let req = server.next_request();
        assert!(req.starts_with("POST /courses HTTP/1.1"));
        assert!(req.contains(r#""course_hash":"ch1""#), "body carries the course identity");
        assert!(resp.accepted);
    }

    #[test]
    fn course_ranking_sends_the_limit_on_the_course_resource() {
        let server = TestServer::spawn(vec![ok(
            r#"[{"player":{"id":"a"},"player_name":"A","clear":"Normal","ex_score":100,"max_combo":10,"minbp":1,"rank":1,"played_at":5}]"#,
        )]);
        let rows = server.client().course_ranking("ch1", 25).expect("course ranking succeeds");
        assert!(server.next_request().starts_with("GET /courses/ch1/ranking?limit=25 HTTP/1.1"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ex_score, 100);
    }

    #[test]
    fn download_replay_requests_the_replay_resource_by_id() {
        let server = TestServer::spawn(vec![ok(r#"{"format":"rbms-us-v1","events":[{"t_us":9,"lane":2,"press":false}],"seed":7}"#)]);
        let replay = server.client().download_replay("replay-99").expect("download succeeds");
        assert!(server.next_request().starts_with("GET /replays/replay-99 HTTP/1.1"));
        assert_eq!(replay.format, "rbms-us-v1");
        assert_eq!(replay.events.len(), 1);
        assert_eq!(replay.seed, Some(7));
    }

    #[test]
    fn get_settings_requests_the_named_blob_under_the_player() {
        let server = TestServer::spawn(vec![ok(r#"{"name":"keyconfig","content":"(k:1)","updated_at":42}"#)]);
        let blob = server.client().get_settings(&PlayerId { id: "p1".into() }, "keyconfig").expect("get succeeds");
        assert!(server.next_request().starts_with("GET /players/p1/settings/keyconfig HTTP/1.1"));
        assert_eq!(blob.content, "(k:1)");
        assert_eq!(blob.updated_at, 42);
    }

    #[test]
    fn register_posts_to_the_auth_register_endpoint() {
        let server = TestServer::spawn(vec![ok(r#"{"token":"tok","player":{"id":"p1"},"name":"P1"}"#)]);
        let resp = server.client().register(&auth_request()).expect("register succeeds");
        assert!(server.next_request().starts_with("POST /auth/register HTTP/1.1"));
        assert_eq!(resp.token, "tok");
        assert_eq!(resp.player.id, "p1");
    }

    #[test]
    fn login_posts_to_the_auth_login_endpoint() {
        let server = TestServer::spawn(vec![ok(r#"{"token":"tok","player":{"id":"p1"},"name":"P1"}"#)]);
        let resp = server.client().login(&auth_request()).expect("login succeeds");
        let req = server.next_request();
        assert!(req.starts_with("POST /auth/login HTTP/1.1"));
        assert!(req.contains(r#""password":"pw""#), "credentials reach the body");
        assert_eq!(resp.name, "P1");
    }

    #[test]
    fn requests_carry_the_bearer_token_when_configured() {
        let server = TestServer::spawn(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)]);
        let client = HttpScoreServer::try_with_timeout(&server.base, Some("tok-123".into()), TEST_TIMEOUT).expect("client builds");
        client.health().expect("health succeeds");
        assert!(server.next_request().to_lowercase().contains("authorization: bearer tok-123"));
    }

    #[test]
    fn requests_omit_authorization_without_a_token() {
        let server = TestServer::spawn(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)]);
        server.client().health().expect("health succeeds");
        assert!(!server.next_request().to_lowercase().contains("authorization:"));
    }

    #[test]
    fn not_found_maps_to_the_not_found_variant_with_the_body() {
        let server = TestServer::spawn(vec![response("404 Not Found", r#"{"error":"no such chart"}"#)]);
        let err = server.client().chart_ranking(&chart(), 10).expect_err("404 is an error");
        let _ = server.next_request();
        match err {
            IrError::NotFound(body) => assert!(body.contains("no such chart"), "the server body is preserved"),
            other => panic!("expected NotFound(_), got {other:?}"),
        }
    }

    #[test]
    fn internal_server_error_maps_to_server_error_500() {
        let server = TestServer::spawn(vec![response("500 Internal Server Error", "boom")]);
        let err = server.client().submit_score(&submission()).expect_err("500 is an error");
        let _ = server.next_request();
        assert!(matches!(err, IrError::Server(500, ref body) if body == "boom"), "got {err:?}");
    }

    #[test]
    fn unauthorized_maps_to_the_unauthorized_variant() {
        let server = TestServer::spawn(vec![response("401 Unauthorized", "token required")]);
        let err = server.client().player_profile(&PlayerId { id: "p1".into() }).expect_err("401 is an error");
        let _ = server.next_request();
        assert!(matches!(err, IrError::Unauthorized(ref body) if body == "token required"), "got {err:?}");
        assert!(err.is_auth_failure());
    }

    #[test]
    fn malformed_success_body_maps_to_decode_error() {
        let server = TestServer::spawn(vec![ok("{ not json")]);
        let err = server.client().health().expect_err("broken JSON is an error");
        let _ = server.next_request();
        assert!(matches!(err, IrError::Decode(_)), "got {err:?}");
    }

    #[test]
    fn success_body_of_the_wrong_shape_maps_to_decode_error() {
        let server = TestServer::spawn(vec![ok(r#"{"unexpected":true}"#)]);
        let err = server.client().health().expect_err("missing required fields is an error");
        let _ = server.next_request();
        assert!(matches!(err, IrError::Decode(_)), "got {err:?}");
    }

    #[test]
    fn connection_refused_maps_to_network_error() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        drop(listener);
        let client = HttpScoreServer::try_with_timeout(format!("http://{addr}"), None, TEST_TIMEOUT).expect("client builds");
        assert!(matches!(client.health(), Err(IrError::Network(_))));
    }

    #[test]
    fn a_server_slower_than_the_timeout_maps_to_network_error() {
        let server = TestServer::spawn_with_delay(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)], SLOW_SERVER_DELAY);
        let started = std::time::Instant::now();
        let err = server.client().health().expect_err("a stalled server must not hang the call");
        let _ = server.next_request();
        assert!(matches!(err, IrError::Network(_)), "got {err:?}");
        assert!(started.elapsed() < SLOW_SERVER_DELAY, "the call returned before the server replied");
    }
}
