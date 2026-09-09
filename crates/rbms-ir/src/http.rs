use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::dto::*;
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

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, IrError> {
        let mut req = self.client()?.get(self.url(path));
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
        let mut req = self.client()?.post(self.url(path)).json(body);
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
        let mut req = self.client()?.put(self.url(path)).json(body);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread;

    /// Per-request budget used by the tests; short enough that the timeout case stays fast.
    const TEST_TIMEOUT: Duration = Duration::from_millis(300);
    /// How long a test waits for the server thread to hand over a captured request.
    const CAPTURE_WAIT: Duration = Duration::from_secs(10);
    /// Delay the "slow server" case uses; must exceed `TEST_TIMEOUT`.
    const SLOW_SERVER_DELAY: Duration = Duration::from_millis(1500);
    /// Delay the ignored default-constructor case uses; must exceed `REQUEST_TIMEOUT`.
    const UNREACHABLE_DELAY: Duration = Duration::from_secs(30);
    /// Stand-in reason for a `reqwest` builder failure (no TLS backend, for instance).
    const BUILDER_FAILURE: &str = "no tls backend";

    /// Minimal single-shot HTTP/1.1 server on loopback: serves the canned responses in order,
    /// one per connection, and reports each raw request (head + body) back over a channel.
    ///
    /// The server thread is deliberately detached. Joining it from `Drop` would block forever on
    /// `accept()` whenever a test unwinds before connecting, and the test harness has no
    /// per-test timeout to break that out.
    struct TestServer {
        base: String,
        requests: mpsc::Receiver<String>,
    }

    impl TestServer {
        fn spawn(responses: Vec<String>) -> TestServer {
            TestServer::spawn_with_delay(responses, Duration::from_millis(0))
        }

        fn spawn_with_delay(responses: Vec<String>, delay: Duration) -> TestServer {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            let addr = listener.local_addr().expect("local addr");
            let (tx, requests) = mpsc::channel();
            thread::spawn(move || {
                for response in responses {
                    let Ok((mut stream, _)) = listener.accept() else { return };
                    if let Some(req) = read_request(&stream) {
                        let _ = tx.send(req);
                    }
                    thread::sleep(delay);
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            });
            TestServer { base: format!("http://{addr}"), requests }
        }

        fn client(&self) -> HttpScoreServer {
            HttpScoreServer::try_with_timeout(&self.base, None, TEST_TIMEOUT).expect("client builds")
        }

        fn next_request(&self) -> String {
            self.requests.recv_timeout(CAPTURE_WAIT).expect("server captured a request")
        }
    }

    fn read_request(stream: &TcpStream) -> Option<String> {
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).ok()? == 0 {
                return None;
            }
            let end_of_head = line == "\r\n" || line == "\n";
            head.push_str(&line);
            if end_of_head {
                break;
            }
        }
        let mut content_length = 0usize;
        for line in head.lines() {
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body).ok()?;
        }
        head.push_str(&String::from_utf8_lossy(&body));
        Some(head)
    }

    fn response(status: &str, body: &str) -> String {
        format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
    }

    fn ok(body: &str) -> String {
        response("200 OK", body)
    }

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
            extra: Default::default(),
        }
    }

    fn auth_request() -> AuthRequest {
        AuthRequest { id: "p1".into(), password: "pw".into(), email: None, name: Some("P1".into()) }
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
        let blob = SettingsBlob { name: "keyconfig".into(), content: "()".into(), updated_at: 0 };
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
        let replay = ReplayData { format: "rbms-us-v1".into(), events: vec![ReplayEvent { t_us: 1, lane: 0, press: true }], seed: Some(7) };
        let id = server.client().upload_replay(&chart(), &replay).expect("upload succeeds");
        assert!(server.next_request().starts_with("POST /charts/abc123/replays HTTP/1.1"));
        assert_eq!(id, "replay-99");
    }

    #[test]
    fn put_settings_accepts_a_204_no_content_reply() {
        let server = TestServer::spawn(vec!["HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()]);
        let blob = SettingsBlob { name: "keyconfig".into(), content: "()".into(), updated_at: 0 };
        server.client().put_settings(&PlayerId { id: "p1".into() }, &blob).expect("put succeeds");
        assert!(server.next_request().starts_with("PUT /players/p1/settings/keyconfig HTTP/1.1"));
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
    fn not_found_maps_to_server_error_with_status_and_body() {
        let server = TestServer::spawn(vec![response("404 Not Found", r#"{"error":"no such chart"}"#)]);
        let err = server.client().chart_ranking(&chart(), 10).expect_err("404 is an error");
        let _ = server.next_request();
        match err {
            IrError::Server(code, body) => {
                assert_eq!(code, 404);
                assert!(body.contains("no such chart"), "the server body is preserved");
            }
            other => panic!("expected Server(404, _), got {other:?}"),
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
    fn unauthorized_maps_to_server_error_401() {
        let server = TestServer::spawn(vec![response("401 Unauthorized", "token required")]);
        let err = server.client().player_profile(&PlayerId { id: "p1".into() }).expect_err("401 is an error");
        let _ = server.next_request();
        assert!(matches!(err, IrError::Server(401, _)), "got {err:?}");
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
