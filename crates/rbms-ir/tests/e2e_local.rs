use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rbms_ir::{
    API_VERSION, AuthRequest, ChartId, ChartRankingQuery, ChartReplayQuery, ClearLamp, GaugeType, HttpScoreServer, IrError, JudgeBreakdown, PlayOptions,
    PlayerId, PlayerScoresQuery, RandomOption, ReplayData, ReplayEvent, ScoreFlag, ScoreServer, ScoreSubmission, SettingsBlob,
};

const E2E_URL_ENV: &str = "RBMS_IR_E2E_URL";
const ACCOUNT_PREFIX: &str = "e2e-phase-i-";
const PASSWORD: &str = "hunter2hunter2";
const SETTINGS_NAME: &str = "keyconfig";
const SETTINGS_CONTENT: &str = "(keys:[1,2,3])";
const STALE_LOCK_BASE: i64 = 1;
const RANKING_LIMIT: u32 = 50;
const TOTAL_NOTES: u32 = 10;
const PGREAT: u32 = 8;
const GREAT: u32 = 2;
const EX_SCORE: u32 = PGREAT * 2 + GREAT;
const MAX_EX_SCORE: u32 = TOTAL_NOTES * 2;
const MD5_HEX_LEN: usize = 32;
const SHA256_HEX_LEN: usize = 64;
const REPLAY_SIZE: u64 = 128;

fn nanos_seed() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after epoch").as_nanos()
}

fn hex_of(seed: u128, salt: u128, len: usize) -> String {
    let mut out = String::new();
    let mut value = seed ^ (salt.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    while out.len() < len {
        out.push_str(&format!("{:016x}", (value as u64) ^ ((value >> 64) as u64)));
        value = value.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    }
    out.truncate(len);
    out
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after epoch").as_millis() as i64
}

fn judge() -> JudgeBreakdown {
    JudgeBreakdown { pgreat: PGREAT, great: GREAT, epg: PGREAT, egr: GREAT, ..Default::default() }
}

fn options() -> PlayOptions {
    PlayOptions {
        gauge: GaugeType::Normal,
        random: RandomOption::Off,
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
        green_number: 310.0,
    }
}

fn submission(chart: &ChartId, player: &PlayerId, played_at: i64) -> ScoreSubmission {
    ScoreSubmission {
        api_version: API_VERSION,
        chart: chart.clone(),
        player: player.clone(),
        mode: "BEAT_7K".into(),
        clear: ClearLamp::Hard,
        ex_score: EX_SCORE,
        max_ex_score: MAX_EX_SCORE,
        judge: judge(),
        max_combo: TOTAL_NOTES,
        total_notes: TOTAL_NOTES,
        passnotes: TOTAL_NOTES,
        minbp: 0,
        gauge_value: 86.0,
        options: options(),
        played_at,
        client: "rbms-e2e/0.1".into(),
        replay_id: None,
        seed: 42,
        judge_algorithm: "Combo".into(),
        rule: String::new(),
        skin: "NORMAL".into(),
        client_build_sha256: None,
        client_platform: Some("macos-aarch64".into()),
        extra: HashMap::new(),
    }
}

/// A replay with every optional field pinned, so the download can be compared to it field for
/// field: an unset `event_count`/`duration_us`/`size` would be derived server-side instead.
fn replay(chart: &ChartId, score_id: Option<String>) -> ReplayData {
    let events = vec![
        ReplayEvent { t_us: 0, lane: 1, press: true },
        ReplayEvent { t_us: 120_000, lane: 1, press: false },
        ReplayEvent { t_us: 250_000, lane: 7, press: true },
        ReplayEvent { t_us: -1_000, lane: 0, press: true },
    ];
    ReplayData {
        format: "rbms-us-v1".into(),
        event_count: Some(events.len() as u32),
        duration_us: Some(251_000),
        size: Some(REPLAY_SIZE),
        events,
        seed: Some(42),
        chart: Some(chart.clone()),
        score_id,
        mode: "BEAT_7K".into(),
        random: Some(RandomOption::SRandom),
        random_p2: Some(RandomOption::Mirror),
        lntype: 1,
        offset_ms: -3,
        judge_rate: 100,
        scratch_auto: true,
        constant: true,
        gauge: Some(GaugeType::Normal),
        ..Default::default()
    }
}

#[test]
fn phase_i_round_trip_against_a_live_server() {
    let Ok(base) = std::env::var(E2E_URL_ENV) else {
        eprintln!("{E2E_URL_ENV} unset; skipping the live round trip");
        return;
    };

    let seed = nanos_seed();
    let login_id = format!("{ACCOUNT_PREFIX}{}", hex_of(seed, 0, 12));
    let email = format!("{login_id}@example.invalid");
    let chart = ChartId { md5: hex_of(seed, 1, MD5_HEX_LEN), sha256: hex_of(seed, 2, SHA256_HEX_LEN) };
    let player = PlayerId { id: login_id.clone() };

    let anonymous = HttpScoreServer::try_new(&base, None).expect("client builds");
    let info = anonymous.health().expect("health succeeds");
    assert_eq!(info.ir_compat, "superset-1", "the deployment speaks the superset contract");
    assert!(info.capabilities.accounts && info.capabilities.settings_sync && info.capabilities.replays);
    assert_eq!(anonymous.version().expect("version succeeds").api_version, API_VERSION);

    let registered = anonymous.register(&AuthRequest::register(&login_id, PASSWORD, &email, Some(login_id.clone()))).expect("register succeeds");
    assert_eq!(registered.player.id, login_id);
    eprintln!("created throwaway account {login_id}");

    let logged_in = anonymous.login(&AuthRequest::login(&login_id, PASSWORD)).expect("login succeeds");
    assert_eq!(logged_in.player.id, login_id);

    let authed = HttpScoreServer::try_new(&base, Some(logged_in.token.clone())).expect("client builds");
    let me = authed.whoami().expect("whoami succeeds");
    assert_eq!(me.id, login_id);

    let played_at = now_ms();
    let submitted = authed.submit_score(&submission(&chart, &player, played_at)).expect("submit succeeds");
    assert!(submitted.accepted, "the server accepted the submission");
    eprintln!(
        "submit -> accepted={} rank={:?} ranked={} flags={:?} is_new_best={} score_id={:?}",
        submitted.accepted, submitted.rank, submitted.ranked, submitted.flags, submitted.is_new_best, submitted.score_id
    );

    let score_id = submitted.score_id.clone().expect("the response carries the stored score id, which links a replay to the run");
    assert!(!score_id.is_empty());
    assert!(submitted.ranked, "an unhashed build still ranks unless the deployment requires a trusted one");
    assert!(submitted.has_flag(ScoreFlag::UnknownBuild), "a submission without a build hash is flagged, got {:?}", submitted.flags);
    assert_eq!(submitted.unranked_reasons(), Vec::new(), "UNKNOWN_BUILD alone does not block the ranking");
    assert!(submitted.is_new_best, "the first ranked score on a fresh chart is the player's new best");
    assert_eq!(submitted.previous_best, None, "there was no earlier best to report");

    let mut worse = submission(&chart, &player, played_at + 1);
    worse.clear = ClearLamp::Failed;
    worse.ex_score = EX_SCORE - 4;
    worse.judge = JudgeBreakdown { pgreat: PGREAT - 2, great: GREAT, epg: PGREAT - 2, egr: GREAT, ..Default::default() };
    let resubmitted = authed.submit_score(&worse).expect("the second submit succeeds");
    assert!(!resubmitted.is_new_best, "a worse run is recorded but never becomes the best");
    assert_ne!(resubmitted.score_id, submitted.score_id, "each run is stored under its own id");
    assert_eq!(resubmitted.previous_best, Some(EX_SCORE), "the response reports the best it was measured against");

    let ranking = authed.chart_ranking(&chart, RANKING_LIMIT).expect("ranking succeeds");
    assert_eq!(ranking.len(), 1, "the fresh chart holds exactly the one score");
    assert_eq!(ranking[0].player.id, login_id);
    assert_eq!(ranking[0].ex_score, EX_SCORE);
    assert_eq!(ranking[0].clear, ClearLamp::Hard);

    let paged = authed.chart_ranking_page(&chart, &ChartRankingQuery { limit: 10, page: 1, lnmode: Some(1), rival_of: None }).expect("ranking page succeeds");
    assert_eq!(paged.len(), 1, "the lnmode filter matches the submitted lntype");

    let best = authed.player_best(&chart, &player).expect("best succeeds").expect("the player has a best");
    assert_eq!(best.ex_score, EX_SCORE);
    assert_eq!(best.rank, Some(1));

    let history = authed.player_scores(&player, &PlayerScoresQuery::default()).expect("player scores succeed");
    assert!(history.iter().any(|row| row.ex_score == EX_SCORE), "the submission shows in the history");

    let uploaded = replay(&chart, Some(score_id.clone()));
    let replay_id = authed.upload_replay(&chart, &uploaded).expect("replay upload succeeds");
    let downloaded = authed.download_replay(&replay_id).expect("replay download succeeds");
    assert_eq!(
        serde_json::to_string(&downloaded.events).expect("events serialise"),
        serde_json::to_string(&uploaded.events).expect("events serialise"),
        "the replay events survive the round trip byte for byte"
    );
    assert_eq!(downloaded.id.as_deref(), Some(replay_id.as_str()));
    assert_eq!(downloaded.gauge, uploaded.gauge, "the gauge the run used has to come back or the ghost cannot be reproduced");
    let downloaded_chart = downloaded.chart.clone().expect("the chart travels with the replay");
    assert_eq!(downloaded_chart.sha256, chart.sha256);
    assert_eq!(downloaded_chart.md5, chart.md5, "both digests come back so the chart resolves either way");

    let mut expected = serde_json::to_value(&uploaded).expect("the uploaded replay serialises");
    expected["id"] = serde_json::json!(replay_id);
    assert_eq!(
        serde_json::to_value(&downloaded).expect("the downloaded replay serialises"),
        expected,
        "every field of the uploaded replay survives the round trip; only the server-assigned id is new"
    );

    let listed = authed.chart_replays(&chart, &ChartReplayQuery::default()).expect("replay list succeeds");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, replay_id);
    assert_eq!(listed[0].player.id, login_id);

    let blob =
        SettingsBlob { name: SETTINGS_NAME.into(), content: SETTINGS_CONTENT.into(), format: "ron".into(), updated_at: played_at, base_updated_at: None };
    let put = authed.put_settings(&player, &blob).expect("unconditional put succeeds");
    assert!(put.from_server, "a successful PUT answers with the stamp it stored, so no read-back is needed");
    let stored = authed.get_settings(&player, SETTINGS_NAME).expect("get settings succeeds");
    assert_eq!(stored.content, SETTINGS_CONTENT);
    assert_eq!(stored.format, "ron");
    assert!(stored.base_updated_at.is_none(), "the response never carries the lock base");
    assert_eq!(put.updated_at, stored.updated_at, "the stamp the PUT reported is the one the stored row carries");
    assert_ne!(put.updated_at, blob.updated_at, "the server stamps its own clock and ignores the one the client sent");

    let stale = SettingsBlob { base_updated_at: Some(STALE_LOCK_BASE), content: "(keys:[9])".into(), ..blob.clone() };
    match authed.put_settings(&player, &stale).expect_err("a stale lock base must lose") {
        IrError::SettingsConflict(conflict) => {
            assert!(conflict.conflict);
            let server_copy = conflict.server.expect("the 409 carries the server copy");
            assert_eq!(server_copy.content, SETTINGS_CONTENT, "the winning copy is the one already stored");
            assert_eq!(server_copy.updated_at, stored.updated_at);
        }
        other => panic!("expected SettingsConflict, got {other:?}"),
    }

    match authed.put_settings(&player, &stale).expect_err("the conflict did not hand the client a usable base") {
        IrError::SettingsConflict(conflict) => {
            assert_eq!(conflict.server.expect("the 409 carries the server copy").content, SETTINGS_CONTENT, "the losing write never landed");
        }
        other => panic!("expected SettingsConflict, got {other:?}"),
    }

    let fresh = SettingsBlob { base_updated_at: Some(put.updated_at), content: "(keys:[4,5,6])".into(), ..blob.clone() };
    let second = authed.put_settings(&player, &fresh).expect("the base the PUT reported is what the next write must carry");
    assert!(second.from_server);
    let after = authed.get_settings(&player, SETTINGS_NAME).expect("get succeeds");
    assert_eq!(after.content, "(keys:[4,5,6])");
    assert_eq!(second.updated_at, after.updated_at, "the second PUT reports the stamp it stored too");

    let cleared = authed.put_rivals(&player, &[]).expect("clearing rivals succeeds");
    assert!(cleared.is_empty());
    let with_unknown = authed.put_rivals(&player, &["no-such-player-e2e".to_string()]).expect("put rivals succeeds");
    assert!(with_unknown.is_empty(), "an unresolvable rival id is dropped rather than stored");
    assert!(authed.rivals(&player).expect("rivals read succeeds").is_empty());

    let forbidden = authed.put_settings(&PlayerId { id: "someone-else".into() }, &blob).expect_err("writing another player's settings must fail");
    assert!(forbidden.is_auth_failure(), "got {forbidden:?}");
}
