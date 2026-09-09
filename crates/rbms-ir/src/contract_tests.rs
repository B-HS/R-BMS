use crate::dto::*;
use crate::error::{ERROR_DETAIL_MAX_CHARS, STATUS_CONFLICT};
use crate::mock_http::{TestServer, created, no_content, ok, response};
use crate::{API_VERSION, IrError, ScoreServer};

mod fixtures;
use fixtures::{chart, course_submission, player, submission};

const TOKEN: &str = "tok-abc";
const AUTHORIZATION_HEADER: &str = "authorization: bearer tok-abc";

fn request_line(request: &str) -> &str {
    request.lines().next().unwrap_or_default()
}

fn assert_authorized(request: &str) {
    assert!(request.to_lowercase().contains(AUTHORIZATION_HEADER), "the call must carry the bearer token, got {request}");
}

fn body_of(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

fn error_envelope(code: &str, message: &str) -> String {
    format!(r#"{{"success":false,"error":{{"code":"{code}","message":"{message}"}}}}"#)
}

fn settings_blob_json() -> &'static str {
    r#"{"name":"keyconfig","format":"ron","content":"(keys:[1,2,3])","updated_at":1700000000000}"#
}

fn profile_json(id: &str) -> String {
    format!(r#"{{"id":"{id}","name":"{id} player","total_plays":42,"rank_points":13.5,"extra":{{"rank":"AAA"}}}}"#)
}

#[test]
fn version_reports_the_contract_and_build() {
    let server = TestServer::spawn(vec![ok(r#"{"api_version":1,"server":"rbms-web","commit":"c6f0885"}"#)]);
    let info = server.client().version().expect("version succeeds");
    assert_eq!(request_line(&server.next_request()), "GET /version HTTP/1.1");
    assert_eq!(info.api_version, API_VERSION);
    assert_eq!(info.server, "rbms-web");
    assert_eq!(info.commit.as_deref(), Some("c6f0885"));
}

#[test]
fn version_accepts_a_null_commit() {
    let server = TestServer::spawn(vec![ok(r#"{"api_version":1,"server":"rbms-web","commit":null}"#)]);
    let info = server.client().version().expect("version succeeds");
    let _ = server.next_request();
    assert!(info.commit.is_none());
}

#[test]
fn health_reads_every_advertised_capability() {
    let body = r#"{"name":"rbms-web","version":"0.1.0","ir_compat":"superset-1","capabilities":{"ranking":true,"player_best":true,"rivals":true,"courses":true,"replays":true,"tables":true,"settings_sync":true,"accounts":true,"lr2ir_compat":false}}"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let info = server.client().health().expect("health succeeds");
    assert_eq!(request_line(&server.next_request()), "GET /health HTTP/1.1");
    assert!(info.capabilities.settings_sync);
    assert!(info.capabilities.accounts);
    assert!(!info.capabilities.lr2ir_compat);
}

#[test]
fn health_defaults_capabilities_a_server_omits() {
    let server = TestServer::spawn(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c","capabilities":{"ranking":true}}"#)]);
    let info = server.client().health().expect("health succeeds");
    let _ = server.next_request();
    assert!(info.capabilities.ranking);
    assert!(!info.capabilities.settings_sync, "an omitted capability reads as unsupported");
}

#[test]
fn register_posts_the_account_body_and_returns_the_token() {
    let server = TestServer::spawn(vec![created(r#"{"token":"tok-new","player":{"id":"gkn"},"name":"gkn"}"#)]);
    let request = AuthRequest::register("gkn", "hunter2hunter2", "gkn@example.com", Some("gkn".into()));
    let response = server.client().register(&request).expect("register succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "POST /auth/register HTTP/1.1");
    assert!(captured.contains(r#""api_version":1"#), "got {captured}");
    assert!(captured.contains(r#""id":"gkn""#));
    assert!(captured.contains(r#""email":"gkn@example.com""#), "the server requires an email to register");
    assert!(captured.contains(r#""name":"gkn""#));
    assert_eq!(response.token, "tok-new");
    assert_eq!(response.player.id, "gkn");
}

#[test]
fn register_conflict_surfaces_the_server_error_code() {
    let server = TestServer::spawn(vec![response("409 Conflict", &error_envelope("IR_ACCOUNT_EXISTS", "already taken"))]);
    let request = AuthRequest::register("gkn", "hunter2hunter2", "gkn@example.com", None);
    let err = server.client().register(&request).expect_err("409 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::Conflict(_)), "got {err:?}");
    assert_eq!(err.error_code().as_deref(), Some("IR_ACCOUNT_EXISTS"));
    assert_eq!(err.status(), Some(STATUS_CONFLICT));
}

#[test]
fn login_sends_only_the_credentials_the_server_reads() {
    let server = TestServer::spawn(vec![ok(r#"{"token":"tok-login","player":{"id":"gkn"},"name":"gkn"}"#)]);
    let response = server.client().login(&AuthRequest::login("gkn", "hunter2hunter2")).expect("login succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "POST /auth/login HTTP/1.1");
    assert!(captured.contains(r#""password":"hunter2hunter2""#));
    assert!(captured.contains(r#""email":null"#), "login carries the nullable email slot, got {captured}");
    assert_eq!(response.token, "tok-login");
}

#[test]
fn login_with_bad_credentials_maps_to_unauthorized() {
    let server = TestServer::spawn(vec![response("401 Unauthorized", &error_envelope("UNAUTHORIZED", "bad credentials"))]);
    let err = server.client().login(&AuthRequest::login("gkn", "wrong")).expect_err("401 is an error");
    let _ = server.next_request();
    assert!(err.is_auth_failure(), "got {err:?}");
    assert_eq!(err.error_code().as_deref(), Some("UNAUTHORIZED"));
}

#[test]
fn whoami_reads_the_profile_behind_the_token() {
    let server = TestServer::spawn(vec![ok(&profile_json("gkn"))]);
    let profile = server.client_with_token(TOKEN).whoami().expect("whoami succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "GET /auth/me HTTP/1.1");
    assert_authorized(&captured);
    assert_eq!(profile.id, "gkn");
    assert_eq!(profile.total_plays, 42);
    assert_eq!(profile.extra.get("rank").and_then(|v| v.as_str()), Some("AAA"));
}

#[test]
fn whoami_without_a_token_maps_to_unauthorized() {
    let server = TestServer::spawn(vec![response("401 Unauthorized", &error_envelope("UNAUTHORIZED", "no token"))]);
    let err = server.client().whoami().expect_err("401 is an error");
    let captured = server.next_request();
    assert!(!captured.to_lowercase().contains("authorization:"), "a guest client sends no credentials");
    assert!(matches!(err, IrError::Unauthorized(_)), "got {err:?}");
}

#[test]
fn a_token_can_be_swapped_in_after_login() {
    let server = TestServer::spawn(vec![ok(&profile_json("gkn"))]);
    let client = server.client().with_token(Some(TOKEN.into()));
    assert_eq!(client.token(), Some(TOKEN));
    client.whoami().expect("whoami succeeds");
    assert_authorized(&server.next_request());
}

#[test]
fn a_token_can_be_dropped_on_sign_out() {
    let server = TestServer::spawn(vec![ok(r#"{"name":"n","version":"v","ir_compat":"c"}"#)]);
    let client = server.client_with_token(TOKEN).with_token(None);
    assert_eq!(client.token(), None);
    client.health().expect("health succeeds");
    assert!(!server.next_request().to_lowercase().contains("authorization:"));
}

#[test]
fn submit_score_reads_back_every_superset_response_field() {
    let body = r#"{"accepted":true,"rank":2,"previous_best":1400,"message":"saved","ranked":true,"flags":[],"is_new_best":true,"score_id":"sc_123"}"#;
    let server = TestServer::spawn(vec![created(body)]);
    let response = server.client_with_token(TOKEN).submit_score(&submission()).expect("submit succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "POST /scores HTTP/1.1");
    assert_authorized(&captured);
    assert!(response.ranked);
    assert!(response.is_new_best);
    assert_eq!(response.score_id.as_deref(), Some("sc_123"));
    assert_eq!(response.rank, Some(2));
    assert!(response.unranked_summary().is_none());
}

#[test]
fn submit_score_decodes_a_server_without_the_superset_fields() {
    let server = TestServer::spawn(vec![created(r#"{"accepted":true,"rank":null,"previous_best":null,"message":null}"#)]);
    let response = server.client().submit_score(&submission()).expect("submit succeeds");
    let _ = server.next_request();
    assert!(!response.ranked, "an absent flag defaults instead of failing the decode");
    assert!(response.flags.is_empty());
    assert!(!response.is_new_best);
    assert!(response.score_id.is_none());
}

#[test]
fn submit_score_reports_the_unranked_flags_verbatim() {
    let body = r#"{"accepted":true,"rank":null,"previous_best":null,"message":"recorded (unranked: guest)","ranked":false,"flags":["GUEST","UNKNOWN_BUILD","FUTURE_FLAG"],"is_new_best":false,"score_id":"sc_9"}"#;
    let server = TestServer::spawn(vec![created(body)]);
    let response = server.client().submit_score(&submission()).expect("submit succeeds");
    let _ = server.next_request();
    assert_eq!(response.unranked_reasons(), vec![ScoreFlag::Guest]);
    assert!(response.has_flag(ScoreFlag::UnknownBuild), "the non-blocking flag is still reported");
    assert_eq!(response.unknown_flags(), vec!["FUTURE_FLAG"]);
    assert_eq!(response.unranked_summary().as_deref(), Some("guest submission"));
}

#[test]
fn submit_score_sends_passnotes_alongside_total_notes() {
    let server = TestServer::spawn(vec![created(r#"{"accepted":true,"rank":null,"previous_best":null,"message":null}"#)]);
    server.client().submit_score(&submission()).expect("submit succeeds");
    let body = body_of(&server.next_request()).to_string();
    assert!(body.contains(r#""total_notes":812"#), "got {body}");
    assert!(body.contains(r#""passnotes":812"#), "the server persists this column; omitting it stores 0, got {body}");
}

#[test]
fn an_unauthenticated_submission_must_identify_as_the_guest_player() {
    let server = TestServer::spawn(vec![created(
        r#"{"accepted":true,"rank":null,"previous_best":null,"message":"recorded (unranked: guest)","ranked":false,"flags":["GUEST"]}"#,
    )]);
    let mut sub = submission();
    sub.player = PlayerId { id: crate::GUEST_PLAYER_ID.into() };
    let response = server.client().submit_score(&sub).expect("a guest submission is accepted");
    let captured = server.next_request();
    assert!(!captured.to_lowercase().contains("authorization:"), "a guest submission carries no bearer token");
    assert!(body_of(&captured).contains(r#""player":{"id":"guest"}"#), "got {captured}");
    assert_eq!(response.unranked_reasons(), vec![ScoreFlag::Guest]);
}

#[test]
fn an_unauthenticated_submission_under_another_id_is_refused_as_an_auth_failure() {
    let server = TestServer::spawn(vec![response("401 Unauthorized", &error_envelope("UNAUTHORIZED", "인증이 필요합니다"))]);
    let err = server.client().submit_score(&submission()).expect_err("401 is an error");
    let _ = server.next_request();
    assert_ne!(submission().player.id, crate::GUEST_PLAYER_ID, "the fixture submits under a real account id");
    assert!(err.is_auth_failure(), "got {err:?}");
}

#[test]
fn submit_score_rejects_an_oversized_body_as_payload_too_large() {
    let server = TestServer::spawn(vec![response("413 Payload Too Large", &error_envelope("IR_PAYLOAD_TOO_LARGE", "too big"))]);
    let err = server.client().submit_score(&submission()).expect_err("413 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::PayloadTooLarge(_)), "got {err:?}");
    assert!(!err.is_retryable(), "a too-large body will not shrink on its own");
}

#[test]
fn submit_score_rate_limit_is_retryable() {
    let server = TestServer::spawn(vec![response("429 Too Many Requests", &error_envelope("RATE_LIMITED", "slow down"))]);
    let err = server.client().submit_score(&submission()).expect_err("429 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::RateLimited(_)), "got {err:?}");
    assert!(err.is_retryable());
}

#[test]
fn submitting_another_players_score_maps_to_forbidden() {
    let server = TestServer::spawn(vec![response("403 Forbidden", &error_envelope("FORBIDDEN", "not your account"))]);
    let err = server.client_with_token(TOKEN).submit_score(&submission()).expect_err("403 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::Forbidden(_)), "got {err:?}");
    assert!(err.is_auth_failure());
}

#[test]
fn submit_course_sends_the_full_course_payload() {
    let server = TestServer::spawn(vec![created(r#"{"accepted":true,"rank":1,"previous_best":null,"message":"saved","ranked":true,"score_id":"cs_1"}"#)]);
    let response = server.client_with_token(TOKEN).submit_course(&course_submission()).expect("course submit succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "POST /courses HTTP/1.1");
    assert_authorized(&captured);
    assert!(captured.contains(r#""course_hash":"course-1""#));
    assert!(captured.contains(r#""lntype":1"#));
    assert!(captured.contains(r#""max_ex_score":6000"#));
    assert!(captured.contains(r#""minbp":12"#));
    assert!(captured.contains(r#""trophy":"bronzemedal""#));
    assert_eq!(response.score_id.as_deref(), Some("cs_1"));
}

#[test]
fn course_ranking_reads_the_rows() {
    let body = r#"[{"player":{"id":"gkn"},"player_name":"gkn","clear":"Hard","ex_score":5000,"max_combo":900,"minbp":4,"rank":1,"played_at":1700000000000,"lntype":1,"option":0,"total_notes":3000,"judge":null,"extra":{}}]"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let rows = server.client().course_ranking("course-1", 25).expect("course ranking succeeds");
    assert_eq!(request_line(&server.next_request()), "GET /courses/course-1/ranking?limit=25 HTTP/1.1");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].ex_score, 5000);
    assert!(rows[0].judge.is_none());
}

#[test]
fn chart_ranking_page_sends_the_rival_and_lnmode_filters() {
    let body = r#"[{"player":{"id":"rival-a"},"player_name":"rival-a","clear":"ExHard","ex_score":1600,"max_combo":812,"minbp":1,"rank":1,"played_at":1700000000000,"lntype":1,"option":0,"total_notes":812,"judge":null,"extra":{}}]"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let query = ChartRankingQuery { limit: 20, page: 1, lnmode: Some(1), rival_of: Some("gkn".into()) };
    let rows = server.client().chart_ranking_page(&chart(), &query).expect("ranking page succeeds");
    assert_eq!(request_line(&server.next_request()), "GET /charts/9f8e7d6c5b4a39281706f5e4d3c2b1a0/ranking?limit=20&page=1&lnmode=1&rival_of=gkn HTTP/1.1");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].player.id, "rival-a");
}

#[test]
fn course_ranking_page_sends_the_same_query() {
    let server = TestServer::spawn(vec![ok("[]")]);
    let query = ChartRankingQuery { limit: 5, page: 2, ..Default::default() };
    let rows = server.client().course_ranking_page("course-1", &query).expect("course ranking page succeeds");
    assert_eq!(request_line(&server.next_request()), "GET /courses/course-1/ranking?limit=5&page=2 HTTP/1.1");
    assert!(rows.is_empty());
}

#[test]
fn player_scores_sends_the_paging_and_filter_query() {
    let body = r#"[{"player":{"id":"gkn"},"player_name":"gkn","clear":"Normal","ex_score":1200,"max_combo":300,"minbp":9,"rank":null,"played_at":1700000000000,"lntype":0,"option":0,"total_notes":812,"judge":null,"extra":{}}]"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let query = PlayerScoresQuery { since: Some(1_699_000_000_000), mode: Some("BEAT_7K".into()), limit: 10, page: 3 };
    let rows = server.client().player_scores(&player(), &query).expect("player scores succeed");
    assert_eq!(request_line(&server.next_request()), "GET /players/gkn/scores?limit=10&page=3&since=1699000000000&mode=BEAT_7K HTTP/1.1");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].rank.is_none(), "the history endpoint does not rank rows");
}

#[test]
fn player_scores_defaults_match_the_server_defaults() {
    let server = TestServer::spawn(vec![ok("[]")]);
    let rows = server.client().player_scores(&player(), &PlayerScoresQuery::default()).expect("player scores succeed");
    assert_eq!(request_line(&server.next_request()), "GET /players/gkn/scores?limit=50&page=1 HTTP/1.1");
    assert!(rows.is_empty());
}

#[test]
fn player_scores_for_an_unknown_player_maps_to_not_found() {
    let server = TestServer::spawn(vec![response("404 Not Found", &error_envelope("IR_PLAYER_NOT_FOUND", "no such player"))]);
    let err = server.client().player_scores(&player(), &PlayerScoresQuery::default()).expect_err("404 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::NotFound(_)), "got {err:?}");
    assert_eq!(err.error_code().as_deref(), Some("IR_PLAYER_NOT_FOUND"));
}

#[test]
fn put_rivals_replaces_the_whole_list_and_returns_profiles() {
    let server = TestServer::spawn(vec![ok(&format!("[{},{}]", profile_json("rival-a"), profile_json("rival-b")))]);
    let rivals = ["rival-a".to_string(), "rival-b".to_string()];
    let profiles = server.client_with_token(TOKEN).put_rivals(&player(), &rivals).expect("put rivals succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "PUT /players/gkn/rivals HTTP/1.1");
    assert_authorized(&captured);
    assert_eq!(body_of(&captured), r#"{"rivals":["rival-a","rival-b"]}"#);
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[1].id, "rival-b");
}

#[test]
fn put_rivals_for_another_player_maps_to_forbidden() {
    let server = TestServer::spawn(vec![response("403 Forbidden", &error_envelope("FORBIDDEN", "not your account"))]);
    let err = server.client_with_token(TOKEN).put_rivals(&player(), &[]).expect_err("403 is an error");
    let captured = server.next_request();
    assert_eq!(body_of(&captured), r#"{"rivals":[]}"#, "an empty list clears the rivals");
    assert!(matches!(err, IrError::Forbidden(_)), "got {err:?}");
}

#[test]
fn get_settings_reads_the_named_blob() {
    let server = TestServer::spawn(vec![ok(settings_blob_json())]);
    let blob = server.client_with_token(TOKEN).get_settings(&player(), "keyconfig").expect("get settings succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "GET /players/gkn/settings/keyconfig HTTP/1.1");
    assert_authorized(&captured);
    assert_eq!(blob.format, "ron");
    assert_eq!(blob.updated_at, 1_700_000_000_000);
    assert!(blob.base_updated_at.is_none(), "the server never sends the lock base back");
}

#[test]
fn get_settings_for_an_unset_blob_maps_to_not_found() {
    let server = TestServer::spawn(vec![response("404 Not Found", &error_envelope("IR_SETTING_NOT_FOUND", "no such blob"))]);
    let err = server.client_with_token(TOKEN).get_settings(&player(), "keyconfig").expect_err("404 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::NotFound(_)), "got {err:?}");
}

#[test]
fn put_settings_sends_the_full_put_body_and_accepts_204() {
    let server = TestServer::spawn(vec![no_content()]);
    let blob = SettingsBlob {
        name: "keyconfig".into(),
        content: "(keys:[1,2,3])".into(),
        updated_at: 1_700_000_000_000,
        format: "ron".into(),
        base_updated_at: Some(1_699_000_000_000),
    };
    server.client_with_token(TOKEN).put_settings(&player(), &blob).expect("put settings succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "PUT /players/gkn/settings/keyconfig HTTP/1.1");
    assert_authorized(&captured);
    assert!(captured.contains(r#""api_version":1"#), "got {captured}");
    assert!(captured.contains(r#""name":"keyconfig""#));
    assert!(captured.contains(r#""format":"ron""#));
    assert!(captured.contains(r#""base_updated_at":1699000000000"#));
}

#[test]
fn put_settings_without_a_lock_base_sends_null() {
    let server = TestServer::spawn(vec![no_content()]);
    let blob = SettingsBlob { name: "settings".into(), content: "()".into(), ..Default::default() };
    server.client_with_token(TOKEN).put_settings(&player(), &blob).expect("put settings succeeds");
    let captured = server.next_request();
    assert!(captured.contains(r#""base_updated_at":null"#), "an unconditional write still sends the slot, got {captured}");
}

#[test]
fn put_settings_conflict_carries_the_server_copy() {
    let conflict_body = format!(r#"{{"conflict":true,"server":{}}}"#, settings_blob_json());
    let server = TestServer::spawn(vec![response("409 Conflict", &conflict_body)]);
    let blob = SettingsBlob { name: "keyconfig".into(), content: "()".into(), base_updated_at: Some(1), ..Default::default() };
    let err = server.client_with_token(TOKEN).put_settings(&player(), &blob).expect_err("409 is an error");
    let _ = server.next_request();
    match err {
        IrError::SettingsConflict(conflict) => {
            assert!(conflict.conflict);
            let remote = conflict.server.expect("the server copy is returned with the conflict");
            assert_eq!(remote.name, "keyconfig");
            assert_eq!(remote.content, "(keys:[1,2,3])");
            assert_eq!(remote.updated_at, 1_700_000_000_000);
        }
        other => panic!("expected SettingsConflict, got {other:?}"),
    }
}

#[test]
fn put_settings_conflict_without_a_server_copy_still_decodes() {
    let server = TestServer::spawn(vec![response("409 Conflict", r#"{"conflict":true,"server":null}"#)]);
    let blob = SettingsBlob { name: "keyconfig".into(), base_updated_at: Some(1), ..Default::default() };
    let err = server.client_with_token(TOKEN).put_settings(&player(), &blob).expect_err("409 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::SettingsConflict(ref conflict) if conflict.server.is_none()), "got {err:?}");
}

#[test]
fn put_settings_conflict_that_is_not_the_lock_envelope_stays_a_plain_conflict() {
    let server = TestServer::spawn(vec![response("409 Conflict", "gateway says no")]);
    let blob = SettingsBlob { name: "keyconfig".into(), ..Default::default() };
    let err = server.client_with_token(TOKEN).put_settings(&player(), &blob).expect_err("409 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::Conflict(ref body) if body == "gateway says no"), "got {err:?}");
}

#[test]
fn put_settings_over_the_size_cap_maps_to_payload_too_large() {
    let server = TestServer::spawn(vec![response("413 Payload Too Large", &error_envelope("IR_PAYLOAD_TOO_LARGE", "too big"))]);
    let blob = SettingsBlob { name: "keyconfig".into(), content: "x".repeat(64), ..Default::default() };
    let err = server.client_with_token(TOKEN).put_settings(&player(), &blob).expect_err("413 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::PayloadTooLarge(_)), "got {err:?}");
}

#[test]
fn upload_replay_sends_the_linked_body_and_returns_the_new_id() {
    let server = TestServer::spawn(vec![created(r#"{"id":"rp_77","url":"/api/replays/rp_77","event_count":2}"#)]);
    let replay = ReplayData {
        format: "rbms-us-v1".into(),
        events: vec![ReplayEvent { t_us: 0, lane: 1, press: true }, ReplayEvent { t_us: 120_000, lane: 1, press: false }],
        seed: Some(7),
        score_id: Some("sc_123".into()),
        chart: Some(chart()),
        mode: "BEAT_7K".into(),
        random: Some(RandomOption::Random),
        lntype: 1,
        ..Default::default()
    };
    let id = server.client_with_token(TOKEN).upload_replay(&chart(), &replay).expect("upload succeeds");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "POST /charts/9f8e7d6c5b4a39281706f5e4d3c2b1a0/replays HTTP/1.1");
    assert_authorized(&captured);
    assert!(captured.contains(r#""api_version":1"#), "got {captured}");
    assert!(captured.contains(r#""score_id":"sc_123""#), "the replay is linked to the stored score");
    assert!(captured.contains(r#""random":"Random""#));
    assert!(captured.contains(r#""t_us":120000"#));
    assert_eq!(id, "rp_77");
}

#[test]
fn upload_replay_without_a_token_maps_to_unauthorized() {
    let server = TestServer::spawn(vec![response("401 Unauthorized", &error_envelope("UNAUTHORIZED", "no token"))]);
    let replay = ReplayData { format: "rbms-us-v1".into(), ..Default::default() };
    let err = server.client().upload_replay(&chart(), &replay).expect_err("401 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::Unauthorized(_)), "got {err:?}");
}

#[test]
fn chart_replays_lists_the_stored_metadata() {
    let body = r#"[{"id":"rp_77","url":"/api/replays/rp_77","player":{"id":"gkn"},"player_name":"gkn","chart_sha256":"abc","score_id":"sc_123","format":"rbms-us-v1","mode":"BEAT_7K","seed":7,"lntype":1,"event_count":2,"duration_us":120000,"size":128,"client_build_sha256":null,"created_at":1700000000000}]"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let query = ChartReplayQuery { player: Some("gkn".into()), limit: 10 };
    let rows = server.client().chart_replays(&chart(), &query).expect("chart replays succeed");
    assert_eq!(request_line(&server.next_request()), "GET /charts/9f8e7d6c5b4a39281706f5e4d3c2b1a0/replays?limit=10&player=gkn HTTP/1.1");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "rp_77");
    assert_eq!(rows[0].url, "/api/replays/rp_77");
    assert_eq!(rows[0].score_id.as_deref(), Some("sc_123"));
    assert_eq!(rows[0].event_count, 2);
    assert!(rows[0].client_build_sha256.is_none());
}

#[test]
fn chart_replays_defaults_to_every_player_at_the_server_page_size() {
    let server = TestServer::spawn(vec![ok("[]")]);
    let rows = server.client().chart_replays(&chart(), &ChartReplayQuery::default()).expect("chart replays succeed");
    assert_eq!(request_line(&server.next_request()), "GET /charts/9f8e7d6c5b4a39281706f5e4d3c2b1a0/replays?limit=50 HTTP/1.1");
    assert!(rows.is_empty(), "an unknown chart answers with an empty list, not an error");
}

#[test]
fn download_replay_reads_back_the_stored_run() {
    let body = r#"{"api_version":1,"id":"rp_77","format":"rbms-us-v1","chart":{"md5":"9f8e7d6c5b4a39281706f5e4d3c2b1a0","sha256":"abc"},"score_id":"sc_123","mode":"BEAT_7K","random":"Random","random_p2":null,"seed":7,"lntype":1,"offset_ms":-3,"judge_rate":100,"scratch_auto":false,"constant":true,"gauge":"Hard","client_build_sha256":null,"events":[{"t_us":0,"lane":1,"press":true}],"event_count":1,"duration_us":0,"size":64,"extra":{}}"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let replay = server.client().download_replay("rp_77").expect("download succeeds");
    assert_eq!(request_line(&server.next_request()), "GET /replays/rp_77 HTTP/1.1");
    assert_eq!(replay.id.as_deref(), Some("rp_77"));
    assert_eq!(replay.score_id.as_deref(), Some("sc_123"));
    assert_eq!(replay.random, Some(RandomOption::Random));
    assert_eq!(replay.offset_ms, -3);
    assert!(replay.constant);
    assert_eq!(replay.events.len(), 1);
    assert_eq!(replay.gauge, Some(GaugeType::Hard), "the gauge the run used has to survive so the ghost can be reproduced");
    let chart = replay.chart.expect("chart travels with the replay");
    assert_eq!(chart.sha256, "abc");
    assert_eq!(chart.md5, "9f8e7d6c5b4a39281706f5e4d3c2b1a0", "both digests come back so the chart resolves either way");
}

#[test]
fn download_replay_without_a_gauge_leaves_it_unset_rather_than_failing() {
    let body = r#"{"api_version":1,"id":"rp_78","format":"rbms-us-v1","events":[],"seed":null}"#;
    let server = TestServer::spawn(vec![ok(body)]);
    let replay = server.client().download_replay("rp_78").expect("download succeeds");
    let _ = server.next_request();
    assert_eq!(replay.gauge, None, "a server that omits the gauge still decodes");
}

#[test]
fn download_replay_for_an_unknown_id_maps_to_not_found() {
    let server = TestServer::spawn(vec![response("404 Not Found", &error_envelope("IR_REPLAY_NOT_FOUND", "no such replay"))]);
    let err = server.client().download_replay("rp_nope").expect_err("404 is an error");
    let _ = server.next_request();
    assert!(matches!(err, IrError::NotFound(_)), "got {err:?}");
}

#[test]
fn an_html_error_page_is_truncated_in_the_message() {
    let server = TestServer::spawn(vec![response("502 Bad Gateway", &"<html>".repeat(200))]);
    let err = server.client().health().expect_err("502 is an error");
    let _ = server.next_request();
    let rendered = err.to_string();
    assert!(rendered.starts_with("server error 502: "), "got {rendered}");
    assert!(rendered.chars().count() <= "server error 502: ".len() + ERROR_DETAIL_MAX_CHARS + 1, "got {rendered}");
}
