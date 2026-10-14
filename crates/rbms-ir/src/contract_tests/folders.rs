//! The calls a client makes for the player's folders, and the chart words a score carries, against
//! the mock server: the request each call sends and how each answer reads.

use super::*;

const FOLDER_ID: &str = "uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b";

fn index_json() -> &'static str {
    concat!(
        r#"{"folders":["#,
        r#"{"id":"uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b","name":"practice","description":"daily","is_public":false,"#,
        r#""position":0,"item_count":2,"updated_at":1760400005000},"#,
        r#"{"id":"uf_77aa0c0b7e2d4a6f8b1c2d3e4f5a6c","name":"LN","description":"","is_public":false,"#,
        r#""position":1,"item_count":0,"updated_at":1760300000000}]}"#,
    )
}

fn folder_json() -> &'static str {
    concat!(
        r#"{"id":"uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b","name":"practice","description":"daily","is_public":false,"#,
        r#""position":0,"item_count":2,"updated_at":1760400005000,"items":["#,
        r#"{"md5":"0123456789abcdef0123456789abcdef","sha256":"","title":"Example","artist":"Someone","note":""},"#,
        r#"{"md5":"fedcba9876543210fedcba9876543210","sha256":"","title":"Another","artist":"Else","note":"right hand"}]}"#,
    )
}

#[test]
fn the_folder_index_is_asked_for_with_the_token_and_read_in_the_players_order() {
    let server = TestServer::spawn(vec![ok(index_json())]);
    let index = server.client_with_token(TOKEN).user_folders(&player()).expect("the index reads");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), "GET /players/gkn/folders HTTP/1.1");
    assert_authorized(&captured);
    let names: Vec<&str> = index.folders.iter().map(|folder| folder.name.as_str()).collect();
    assert_eq!(names, vec!["practice", "LN"]);
    assert_eq!(index.folders[0].updated_at, 1_760_400_005_000);
}

#[test]
fn one_folder_is_asked_for_by_its_id_and_comes_with_its_charts() {
    let server = TestServer::spawn(vec![ok(folder_json())]);
    let folder = server.client_with_token(TOKEN).user_folder(&player(), FOLDER_ID).expect("the folder reads");
    let captured = server.next_request();
    assert_eq!(request_line(&captured), format!("GET /players/gkn/folders/{FOLDER_ID} HTTP/1.1"));
    assert_authorized(&captured);
    assert_eq!(folder.summary.id, FOLDER_ID);
    assert_eq!(
        folder.items.iter().map(|item| item.md5.as_str()).collect::<Vec<_>>(),
        vec!["0123456789abcdef0123456789abcdef", "fedcba9876543210fedcba9876543210"]
    );
}

/// A server from before folders answers 404, and a token that is not the player's 401 or 403: each
/// is told apart, so a client can pass over the first quietly and hide the folders on the others.
#[test]
fn a_server_without_folders_and_a_refused_token_are_told_apart() {
    let server = TestServer::spawn(vec![
        response("404 Not Found", &error_envelope("NOT_FOUND", "no such route")),
        response("401 Unauthorized", &error_envelope("UNAUTHORIZED", "sign in")),
        response("403 Forbidden", &error_envelope("FORBIDDEN", "not yours")),
    ]);
    let client = server.client_with_token(TOKEN);
    assert!(matches!(client.user_folders(&player()), Err(IrError::NotFound(_))));
    let _ = server.next_request();
    assert!(matches!(client.user_folders(&player()), Err(IrError::Unauthorized(_))));
    let _ = server.next_request();
    assert!(matches!(client.user_folder(&player(), FOLDER_ID), Err(IrError::Forbidden(_))));
    let _ = server.next_request();
}

/// An id that would address something else in the path is never sent.
#[test]
fn a_folder_id_that_is_not_safe_in_a_path_is_refused_without_a_request() {
    let server = TestServer::spawn(Vec::new());
    let err = server.client_with_token(TOKEN).user_folder(&player(), "../settings/player").expect_err("the id is refused");
    assert!(matches!(err, IrError::NotFound(_)), "got {err:?}");
}

/// The folder answer is read only up to its limit, so a server that streams without end cannot
/// fill memory.
#[test]
fn a_folder_answer_larger_than_the_limit_is_refused() {
    let padding = " ".repeat(usize::try_from(crate::http::MAX_FOLDER_RESPONSE_BYTES).expect("the limit fits a usize"));
    let server = TestServer::spawn(vec![ok(&format!("{padding}{}", index_json()))]);
    let err = server.client_with_token(TOKEN).user_folders(&player()).expect_err("an oversized answer is refused");
    let _ = server.next_request();
    assert!(matches!(err, IrError::Decode(ref message) if message.contains("larger than")), "got {err:?}");
}

/// The chart a score is for carries its title, artist and level beside the hashes, inside the same
/// object, so the server can name a chart it has not seen before.
#[test]
fn a_score_names_its_chart_beside_the_hashes() {
    let server = TestServer::spawn(vec![created(r#"{"accepted":true,"rank":null,"previous_best":null,"message":null}"#)]);
    let mut sub = submission();
    sub.chart = SubmittedChart::described(chart(), " Born [29Another] ", "Ninja Action Team / 29", "16");
    server.client_with_token(TOKEN).submit_score(&sub).expect("submit succeeds");
    let body: serde_json::Value = serde_json::from_str(body_of(&server.next_request())).expect("the body is JSON");
    assert_eq!(
        body["chart"],
        serde_json::json!({ "md5": chart().md5, "sha256": chart().sha256, "title": "Born [29Another]", "artist": "Ninja Action Team / 29", "level": 16 })
    );
}

/// A chart without words sends the body a server from before them reads, and a level that is not a
/// whole number is no level.
#[test]
fn a_chart_without_words_sends_only_its_hashes() {
    let bare = serde_json::to_value(SubmittedChart::from(chart())).expect("a chart is JSON");
    assert_eq!(bare, serde_json::json!({ "md5": chart().md5, "sha256": chart().sha256 }));
    let odd = SubmittedChart::described(chart(), "  ", "", "12+");
    assert_eq!((odd.title, odd.artist, odd.level), (None, None, None));
    let back: SubmittedChart = serde_json::from_value(serde_json::json!({ "md5": "a", "sha256": "b" })).expect("an old chart reads");
    assert_eq!(back, SubmittedChart::from(ChartId { md5: "a".into(), sha256: "b".into() }));
}
