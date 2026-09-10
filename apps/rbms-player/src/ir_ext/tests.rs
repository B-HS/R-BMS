use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rbms_ir::{
    ChartId, ClearLamp, CourseSubmission, GaugeType, JudgeBreakdown, PlayOptions, PlayerId, PlayerProfile, RandomOption, ReplayData, ScoreRecord, ServerInfo,
};

use super::*;

/// How long a rendezvous server waits for its siblings before giving up. Long enough that a loaded
/// machine still passes, short enough that a serial fan-out fails the test instead of hanging.
const RENDEZVOUS_WAIT: Duration = Duration::from_secs(5);

/// Gap between checks while a rendezvous server waits for the others to arrive.
const RENDEZVOUS_POLL: Duration = Duration::from_millis(1);

/// How long a test waits for a spawned fan-out to report back.
const RECV_WAIT: Duration = Duration::from_secs(10);

/// Score id the accepting stub reports, so a result can be traced back to the server that produced
/// it.
const STUB_SCORE_ID: &str = "sc-stub";

/// Message the failing stub answers with.
const STUB_FAILURE: &str = "stub is down";

/// What a [`StubServer`] does when a score reaches it.
enum Behaviour {
    /// Take the score and report `score_id`.
    Accept(String),
    /// Fail the call.
    Fail,
    /// Unwind inside the call.
    Panic,
    /// Take the score only once every profile has arrived, proving the fan-out is parallel.
    Rendezvous { arrived: Arc<AtomicUsize>, expected: usize },
}

struct StubServer {
    behaviour: Behaviour,
    seen: Mutex<Vec<ScoreSubmission>>,
}

impl StubServer {
    fn new(behaviour: Behaviour) -> Arc<StubServer> {
        Arc::new(StubServer { behaviour, seen: Mutex::new(Vec::new()) })
    }

    fn accepting() -> Arc<StubServer> {
        StubServer::new(Behaviour::Accept(STUB_SCORE_ID.to_string()))
    }

    fn submissions(&self) -> Vec<ScoreSubmission> {
        self.seen.lock().expect("submission log").clone()
    }
}

impl ScoreServer for StubServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        self.seen.lock().expect("submission log").push(sub.clone());
        match &self.behaviour {
            Behaviour::Accept(score_id) => Ok(SubmitResponse { accepted: true, score_id: Some(score_id.clone()), ..Default::default() }),
            Behaviour::Fail => Err(IrError::Network(STUB_FAILURE.to_string())),
            Behaviour::Panic => panic!("{STUB_FAILURE}"),
            Behaviour::Rendezvous { arrived, expected } => {
                arrived.fetch_add(1, Ordering::SeqCst);
                let deadline = Instant::now() + RENDEZVOUS_WAIT;
                while arrived.load(Ordering::SeqCst) < *expected && Instant::now() < deadline {
                    std::thread::sleep(RENDEZVOUS_POLL);
                }
                if arrived.load(Ordering::SeqCst) < *expected {
                    return Err(IrError::Network("profiles ran one after another".to_string()));
                }
                Ok(SubmitResponse { accepted: true, ..Default::default() })
            }
        }
    }
    fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    fn player_best(&self, _chart: &ChartId, _player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
        Err(IrError::Unsupported)
    }
    fn rivals(&self, _player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::Unsupported)
    }
}

fn profile(name: &str, url: &str, enabled: bool) -> IrProfile {
    IrProfile { name: name.to_string(), base_url: url.to_string(), token: None, enabled }
}

fn multi(profiles: Vec<IrProfile>) -> MultiIr {
    MultiIr { profiles, primary: 0, has_legacy: false }
}

fn submission() -> ScoreSubmission {
    ScoreSubmission {
        api_version: rbms_ir::API_VERSION,
        chart: ChartId { md5: "m".into(), sha256: "s".into() },
        player: PlayerId { id: "p".into() },
        mode: "BEAT_7K".into(),
        clear: ClearLamp::Normal,
        ex_score: 10,
        max_ex_score: 20,
        judge: JudgeBreakdown::default(),
        max_combo: 5,
        total_notes: 10,
        passnotes: 10,
        minbp: 0,
        gauge_value: 50.0,
        options: PlayOptions {
            gauge: GaugeType::Normal,
            random: RandomOption::Off,
            random_p2: None,
            scratch_auto: false,
            lntype: 0,
            input_device: "keyboard".into(),
            assist: vec![],
            option: 0,
            judge_rate: 100,
            offset_ms: 0,
            constant: false,
            hispeed: 1.0,
            lift: 0.0,
            lane_cover: 0.0,
            total_override: 0.0,
            autoplay: false,
            auto_offset: false,
            scratch_left: false,
            green_number: 0.0,
        },
        played_at: 1,
        client: "rbms/test".into(),
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

fn network(server_url: Option<&str>, profiles: Vec<IrProfile>) -> NetworkOptions {
    NetworkOptions { server_url: server_url.map(str::to_string), ir_token: Some("legacy-token".to_string()), ir_profiles: profiles, ..Default::default() }
}

fn labels_of(results: &[ProfileResult<SubmitResponse>]) -> Vec<&str> {
    results.iter().map(|(label, _)| label.as_str()).collect()
}

#[test]
fn one_failing_profile_leaves_the_other_results_intact() {
    let servers: Vec<Arc<dyn ScoreServer>> = vec![StubServer::accepting(), StubServer::new(Behaviour::Fail), StubServer::accepting()];
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true), profile("C", "http://c", true)]);

    let results = multi.submit_all(&servers, &submission());

    assert_eq!(labels_of(&results), vec!["A", "B", "C"], "results come back in profile order");
    assert!(results[0].1.as_ref().is_ok_and(|r| r.accepted), "the first profile is unaffected by the failure next to it");
    assert!(matches!(results[1].1, Err(IrError::Network(_))), "the failing profile reports its own error");
    assert!(results[2].1.as_ref().is_ok_and(|r| r.accepted), "the third profile is unaffected too");
}

#[test]
fn a_panicking_profile_is_reported_without_taking_the_fan_out_down() {
    let servers: Vec<Arc<dyn ScoreServer>> = vec![StubServer::new(Behaviour::Panic), StubServer::accepting()];
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true)]);

    let results = multi.submit_all(&servers, &submission());

    assert_eq!(results.len(), 2);
    assert!(matches!(&results[0].1, Err(IrError::Network(message)) if message == PROFILE_PANIC_MESSAGE));
    assert!(results[1].1.as_ref().is_ok_and(|r| r.accepted));
}

#[test]
fn a_disabled_profile_is_skipped_entirely() {
    let enabled = StubServer::accepting();
    let disabled = StubServer::accepting();
    let servers: Vec<Arc<dyn ScoreServer>> = vec![Arc::clone(&enabled) as Arc<dyn ScoreServer>, Arc::clone(&disabled) as Arc<dyn ScoreServer>];
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", false)]);

    let results = multi.submit_all(&servers, &submission());

    assert_eq!(labels_of(&results), vec!["A"]);
    assert_eq!(enabled.submissions().len(), 1, "the enabled profile still receives the score");
    assert!(disabled.submissions().is_empty(), "a disabled profile never receives the score");
}

#[test]
fn a_profile_without_a_server_reports_that_it_is_not_configured() {
    let servers: Vec<Arc<dyn ScoreServer>> = vec![StubServer::accepting()];
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true)]);

    let results = multi.submit_all(&servers, &submission());

    assert_eq!(results.len(), 2);
    assert!(matches!(results[1].1, Err(IrError::NotConfigured)));
}

#[test]
fn every_profile_receives_the_same_clamped_submission() {
    let first = StubServer::accepting();
    let second = StubServer::accepting();
    let servers: Vec<Arc<dyn ScoreServer>> = vec![Arc::clone(&first) as Arc<dyn ScoreServer>, Arc::clone(&second) as Arc<dyn ScoreServer>];
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true)]);
    let mut sub = submission();
    sub.played_at = -1;
    sub.gauge_value = f32::NAN;

    multi.submit_all(&servers, &sub);

    for stub in [&first, &second] {
        let sent = stub.submissions();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].played_at, rbms_ir::MIN_PLAYED_AT_MS, "a pre-epoch stamp would be a 400 on every profile");
        assert_eq!(sent[0].gauge_value, rbms_ir::FALLBACK_GAUGE_VALUE, "a NaN gauge would serialise to null");
    }
}

#[test]
fn profiles_are_submitted_to_in_parallel() {
    let arrived = Arc::new(AtomicUsize::new(0));
    let expected = 3;
    let servers: Vec<Arc<dyn ScoreServer>> =
        (0..expected).map(|_| StubServer::new(Behaviour::Rendezvous { arrived: Arc::clone(&arrived), expected }) as Arc<dyn ScoreServer>).collect();
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true), profile("C", "http://c", true)]);

    let results = multi.submit_all(&servers, &submission());

    assert_eq!(results.len(), expected);
    for (label, result) in &results {
        assert!(result.as_ref().is_ok_and(|r| r.accepted), "profile {label} did not meet the others");
    }
}

#[test]
fn a_spawned_fan_out_delivers_every_profile_result() {
    let servers: Vec<Arc<dyn ScoreServer>> = vec![StubServer::accepting(), StubServer::new(Behaviour::Fail)];
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true)]);

    let rx = spawn_submit_all(multi, servers, submission());
    let results = rx.recv_timeout(RECV_WAIT).expect("worker replied");

    assert_eq!(labels_of(&results), vec!["A", "B"]);
    assert!(results[0].1.is_ok());
    assert!(results[1].1.is_err());
}

#[test]
fn the_legacy_server_becomes_the_first_profile() {
    let multi = MultiIr::from_network(&network(Some("http://main"), vec![profile("SECOND", "http://second", true)]));

    assert_eq!(multi.labels(), vec![LEGACY_PROFILE_NAME.to_string(), "SECOND".to_string()]);
    assert_eq!(multi.profiles[0].base_url, "http://main");
    assert_eq!(multi.profiles[0].token.as_deref(), Some("legacy-token"), "the stored bearer token follows the legacy server");
    assert_eq!(multi.legacy_index(), Some(0));
    assert_eq!(multi.primary_server(), Some(0), "the panel keeps reading the legacy server by default");
}

#[test]
fn without_a_server_url_only_the_configured_profiles_are_listed() {
    let multi = MultiIr::from_network(&network(None, vec![profile("ONLY", "http://only", true)]));

    assert_eq!(multi.labels(), vec!["ONLY".to_string()]);
    assert_eq!(multi.legacy_index(), None);
    assert_eq!(multi.primary_server(), Some(0));
}

#[test]
fn an_offline_config_has_no_profiles_at_all() {
    let multi = MultiIr::from_network(&network(None, vec![]));

    assert!(multi.is_empty());
    assert_eq!(multi.primary_server(), None);
    assert!(multi.submit_all(&[], &submission()).is_empty());
}

#[test]
fn an_empty_profile_list_keeps_the_single_server_behaviour() {
    let multi = MultiIr::from_network(&network(Some("http://main"), vec![]));

    assert_eq!(multi.profiles.len(), 1, "one profile means one submission, exactly as before");
    assert_eq!(multi.primary_server(), Some(0));
}

#[test]
fn the_panel_falls_back_when_the_primary_profile_is_disabled() {
    let mut multi = multi(vec![profile("A", "http://a", false), profile("B", "http://b", true)]);
    multi.primary = 0;

    assert_eq!(multi.primary_server(), Some(1), "a disabled tab must not blank a panel that has a live server");
}

#[test]
fn the_panel_reads_the_selected_primary_when_it_is_enabled() {
    let mut multi = multi(vec![profile("A", "http://a", true), profile("B", "http://b", true)]);
    assert!(multi.set_primary(1));

    assert_eq!(multi.primary_server(), Some(1));
}

#[test]
fn no_enabled_profile_means_no_panel_source() {
    let multi = multi(vec![profile("A", "http://a", false), profile("B", "http://b", false)]);

    assert_eq!(multi.primary_server(), None);
}

#[test]
fn a_primary_index_past_the_end_is_rejected() {
    let mut multi = multi(vec![profile("A", "http://a", true)]);

    assert!(!multi.set_primary(1), "a stale tab index must not move the panel out of range");
    assert_eq!(multi.primary, 0);
}

#[test]
fn an_unnamed_profile_is_labelled_by_its_position() {
    assert_eq!(profile_label(&profile("", "http://a", true), 0), "IR 1");
    assert_eq!(profile_label(&profile("   ", "http://b", true), 2), "IR 3");
    assert_eq!(profile_label(&profile("MOCHA", "http://c", true), 1), "MOCHA");
}

#[test]
fn one_client_is_built_per_profile_in_order() {
    let multi = multi(vec![profile("A", "http://a", true), profile("B", "", true), profile("C", "http://c", false)]);

    let servers = multi.build_servers();

    assert_eq!(servers.len(), multi.profiles.len(), "the client vector stays index-aligned with the profiles");
    assert!(matches!(servers[1].health(), Err(IrError::NotConfigured)), "a blank URL gets the offline stub");
    assert!(matches!(servers[2].health(), Err(IrError::NotConfigured)), "a disabled profile gets the offline stub");
}

#[test]
fn the_already_built_main_server_is_reused_for_the_legacy_profile() {
    let main = StubServer::accepting();
    let multi = MultiIr::from_network(&network(Some("http://main"), vec![profile("SECOND", "http://second", true)]));

    let servers = multi.build_servers_reusing(Some(Arc::clone(&main) as Arc<dyn ScoreServer>));

    assert_eq!(servers.len(), 2);
    let results = multi.submit_all(&servers, &submission());
    assert_eq!(results[0].1.as_ref().expect("the reused client answered").score_id.as_deref(), Some(STUB_SCORE_ID));
    assert_eq!(main.submissions().len(), 1, "the score went through the client the app already had open");
}

#[test]
fn reusing_without_a_main_server_leaves_the_built_client_in_place() {
    let multi = MultiIr::from_network(&network(Some("http://main"), vec![]));

    let servers = multi.build_servers_reusing(None);

    assert_eq!(servers.len(), 1);
}

#[test]
fn a_summary_of_an_all_clear_fan_out_names_no_profile() {
    let results = vec![
        ("A".to_string(), Ok(SubmitResponse { accepted: true, ..Default::default() })),
        ("B".to_string(), Ok(SubmitResponse { accepted: true, ..Default::default() })),
    ];

    assert_eq!(submit_summary(&results), "IR 2/2");
}

#[test]
fn a_summary_names_the_first_failing_profile() {
    let results = vec![
        ("A".to_string(), Ok(SubmitResponse { accepted: true, ..Default::default() })),
        ("B".to_string(), Err(IrError::Network(STUB_FAILURE.to_string()))),
    ];

    let summary = submit_summary(&results);

    assert!(summary.starts_with("IR 1/2 — B: "), "got {summary}");
    assert!(summary.contains(STUB_FAILURE), "the server's own words reach the player: {summary}");
}

#[test]
fn a_summary_names_a_profile_that_answered_without_taking_the_score() {
    let results = vec![("A".to_string(), Ok(SubmitResponse::default()))];

    assert_eq!(submit_summary(&results), "IR 0/1 — A: rejected");
}

#[test]
fn a_summary_of_nothing_says_so() {
    assert_eq!(submit_summary(&[]), "no IR profiles");
}

#[test]
fn the_secondary_view_leaves_the_legacy_server_to_the_single_server_path() {
    let main = StubServer::accepting();
    let second = StubServer::accepting();
    let multi = MultiIr::from_network(&network(Some("http://main"), vec![profile("SECOND", "http://second", true)]));
    let servers: Vec<Arc<dyn ScoreServer>> = vec![Arc::clone(&main) as Arc<dyn ScoreServer>, Arc::clone(&second) as Arc<dyn ScoreServer>];

    let results = multi.secondary().submit_all(&servers, &submission());

    assert_eq!(labels_of(&results), vec!["SECOND"], "the legacy profile is not submitted to twice");
    assert!(main.submissions().is_empty(), "the legacy server hears only from the single-server path");
    assert_eq!(second.submissions().len(), 1);
}

#[test]
fn the_secondary_view_keeps_every_index_and_label_in_place() {
    let multi = MultiIr::from_network(&network(Some("http://main"), vec![profile("", "http://second", true)]));

    let secondary = multi.secondary();

    assert_eq!(secondary.labels(), multi.labels(), "labels are positional, so they must not shift");
    assert_eq!(secondary.profiles.len(), multi.profiles.len());
    assert_eq!(secondary.enabled_profiles().map(|(index, _)| index).collect::<Vec<_>>(), vec![1]);
}

#[test]
fn without_a_legacy_server_the_secondary_view_is_the_whole_list() {
    let multi = MultiIr::from_network(&network(None, vec![profile("A", "http://a", true), profile("B", "http://b", true)]));

    assert_eq!(multi.secondary(), multi);
}
