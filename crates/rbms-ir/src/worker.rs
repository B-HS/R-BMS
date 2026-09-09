use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use crate::dto::{ReplayData, ScoreSubmission, SubmitResponse};
use crate::{IrError, ScoreServer};

/// Run one blocking [`ScoreServer`] call on a dedicated thread and deliver its result over a
/// channel.
///
/// Every method on the trait blocks, so a UI frame loop must never call one directly. This is the
/// smallest useful wrapper: the caller polls the returned [`Receiver`] with `try_recv` once per
/// frame and keeps ownership of any caching or retry policy. It mirrors the reference implementation's
/// `RankingData.load()` (`ir/RankingData.java:71-96`), which also spawns one thread per request
/// instead of holding a pool.
///
/// Everything around that thread stays with the caller, exactly as in the reference implementation: the re-entrancy
/// guard (`RankingData.java:75` sets `state = ACCESS` and consumers read the result only at
/// `state == FINISH`, `skin/property/IntegerPropertyFactory.java:242`) and the per-`(sha256,
/// lnmode)` cache that stops a moving selection cursor from firing one request per frame
/// (`ir/RankingDataCache.java:41-47`). This function has neither.
///
/// Provisional placement: the ranking state machine this serves belongs to the player app, so the
/// helper may move there once that code exists. Do not treat its position in this crate as settled.
///
/// The thread ends as soon as the call returns; dropping the receiver does not cancel the request.
/// The wait is bounded by whatever timeout the concrete [`ScoreServer`] enforces — for
/// [`crate::HttpScoreServer`] that is [`crate::http::REQUEST_TIMEOUT`], unless it was built with
/// `HttpScoreServer::try_with_timeout`.
pub fn spawn_query<T, F>(server: Arc<dyn ScoreServer>, op: F) -> Receiver<Result<T, IrError>>
where
    T: Send + 'static,
    F: FnOnce(&dyn ScoreServer) -> Result<T, IrError> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(op(server.as_ref()));
    });
    rx
}

/// One end-of-song hand-off to the IR: the score, and optionally the replay of the same run.
pub struct SubmitJob {
    pub submission: ScoreSubmission,
    /// Uploaded only after the score lands, so it can be linked to the stored score id. The
    /// server rejects replay uploads without a bearer token, so leave this `None` for guests.
    pub replay: Option<ReplayData>,
}

/// What a [`spawn_submit`] worker reports back: the submission result, plus the replay upload
/// result (the new replay id) when one was attempted.
pub struct SubmitOutcome {
    pub submit: Result<SubmitResponse, IrError>,
    pub replay: Option<Result<String, IrError>>,
}

impl SubmitOutcome {
    /// The server-side replay id, when a replay was uploaded and the upload succeeded.
    pub fn replay_id(&self) -> Option<&str> {
        match &self.replay {
            Some(Ok(id)) => Some(id.as_str()),
            _ => None,
        }
    }

    /// True when the score reached the server and it accepted it.
    pub fn is_accepted(&self) -> bool {
        matches!(&self.submit, Ok(response) if response.accepted)
    }
}

/// Whether a replay is worth uploading for this submission: the server took the score, and it did
/// not report a flag that keeps the run out of the ranking. A server that predates score flags
/// reports none, so the replay still goes up.
pub fn is_replay_upload_warranted(response: &SubmitResponse) -> bool {
    response.accepted && response.unranked_reasons().is_empty()
}

/// Submit a score on a dedicated thread, upload its replay if one is attached, and deliver the
/// whole outcome over a channel the caller polls with `try_recv` once per frame.
///
/// The replay is linked to the score the server just stored: the returned
/// [`SubmitResponse::score_id`] is copied into [`ReplayData::score_id`] before the upload, and the
/// submitted chart fills in [`ReplayData::chart`] when the caller left it unset, so the server can
/// resolve the chart even when it only knows one of the two digests. A failed submission, or one
/// that came back unranked, skips the upload and leaves [`SubmitOutcome::replay`] `None`.
///
/// [`ScoreSubmission::clamp_to_server_bounds`] runs first, so a run whose clock or gauge went out
/// of the server's accepted range is corrected rather than rejected with a 400.
///
/// The thread ends as soon as both calls return; dropping the receiver does not cancel them.
pub fn spawn_submit(server: Arc<dyn ScoreServer>, job: SubmitJob) -> Receiver<SubmitOutcome> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(run_submit(server.as_ref(), job));
    });
    rx
}

fn run_submit(server: &dyn ScoreServer, job: SubmitJob) -> SubmitOutcome {
    let SubmitJob { mut submission, replay } = job;
    submission.clamp_to_server_bounds();
    let submit = server.submit_score(&submission);
    let upload = match (&submit, replay) {
        (Ok(response), Some(mut data)) if is_replay_upload_warranted(response) => {
            if let Some(score_id) = &response.score_id {
                data.score_id = Some(score_id.clone());
            }
            if data.chart.is_none() {
                data.chart = Some(submission.chart.clone());
            }
            Some(server.upload_replay(&submission.chart, &data))
        }
        _ => None,
    };
    SubmitOutcome { submit, replay: upload }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::*;
    use crate::null::NullScoreServer;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    const RECV_WAIT: Duration = Duration::from_secs(10);
    /// Gap between checks while waiting for the detached worker to report in.
    const POLL_INTERVAL: Duration = Duration::from_millis(1);

    struct FixedRankingServer(Vec<ScoreRecord>);

    impl ScoreServer for FixedRankingServer {
        fn health(&self) -> Result<ServerInfo, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
        }
        fn chart_ranking(&self, _chart: &ChartId, limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
            Ok(self.0.iter().take(limit as usize).cloned().collect())
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

    fn record(id: &str, ex: u32) -> ScoreRecord {
        ScoreRecord {
            player: PlayerId { id: id.into() },
            player_name: id.to_uppercase(),
            clear: ClearLamp::Normal,
            ex_score: ex,
            max_combo: 0,
            minbp: 0,
            rank: None,
            played_at: 0,
            lntype: 0,
            option: 0,
            total_notes: 0,
            judge: None,
            extra: Default::default(),
        }
    }

    fn chart() -> ChartId {
        ChartId { md5: "m".into(), sha256: "s".into() }
    }

    fn submission() -> ScoreSubmission {
        ScoreSubmission {
            api_version: crate::API_VERSION,
            chart: chart(),
            player: PlayerId { id: "p".into() },
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

    fn replay() -> ReplayData {
        ReplayData { format: "rbms-us-v1".into(), events: vec![ReplayEvent { t_us: 0, lane: 1, press: true }], ..Default::default() }
    }

    const UPLOADED_REPLAY_ID: &str = "rp_1";

    /// Test double that captures what the worker sent and answers with a canned outcome.
    struct RecordingServer {
        response: Option<SubmitResponse>,
        upload_fails: bool,
        submitted: Mutex<Vec<ScoreSubmission>>,
        uploaded: Mutex<Vec<(ChartId, ReplayData)>>,
    }

    impl RecordingServer {
        fn answering(response: SubmitResponse) -> RecordingServer {
            RecordingServer { response: Some(response), upload_fails: false, submitted: Mutex::new(Vec::new()), uploaded: Mutex::new(Vec::new()) }
        }

        fn rejecting() -> RecordingServer {
            RecordingServer { response: None, upload_fails: false, submitted: Mutex::new(Vec::new()), uploaded: Mutex::new(Vec::new()) }
        }

        fn uploads(&self) -> Vec<(ChartId, ReplayData)> {
            self.uploaded.lock().expect("upload log").clone()
        }

        fn submissions(&self) -> Vec<ScoreSubmission> {
            self.submitted.lock().expect("submission log").clone()
        }
    }

    impl ScoreServer for RecordingServer {
        fn health(&self) -> Result<ServerInfo, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
            self.submitted.lock().expect("submission log").push(sub.clone());
            match &self.response {
                Some(response) => Ok(response.clone()),
                None => Err(IrError::Unauthorized("token required".into())),
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
        fn upload_replay(&self, chart: &ChartId, replay: &ReplayData) -> Result<String, IrError> {
            self.uploaded.lock().expect("upload log").push((chart.clone(), replay.clone()));
            if self.upload_fails {
                return Err(IrError::PayloadTooLarge("replay too big".into()));
            }
            Ok(UPLOADED_REPLAY_ID.to_string())
        }
    }

    fn ranked_response(score_id: &str) -> SubmitResponse {
        SubmitResponse { accepted: true, ranked: true, score_id: Some(score_id.into()), is_new_best: true, ..Default::default() }
    }

    fn run_job(server: Arc<RecordingServer>, job: SubmitJob) -> SubmitOutcome {
        let rx = spawn_submit(server, job);
        rx.recv_timeout(RECV_WAIT).expect("worker replied")
    }

    #[test]
    fn a_submitted_replay_is_linked_to_the_returned_score_id() {
        let server = Arc::new(RecordingServer::answering(ranked_response("sc_7")));
        let outcome = run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: Some(replay()) });
        assert!(outcome.is_accepted());
        assert_eq!(outcome.replay_id(), Some(UPLOADED_REPLAY_ID));
        let uploads = server.uploads();
        assert_eq!(uploads.len(), 1, "exactly one upload follows one accepted score");
        assert_eq!(uploads[0].0, chart(), "the upload addresses the chart that was played");
        assert_eq!(uploads[0].1.score_id.as_deref(), Some("sc_7"), "the replay is linked to the stored score");
        assert_eq!(uploads[0].1.chart, Some(chart()), "the chart identity travels in the body too");
    }

    #[test]
    fn a_submission_is_clamped_into_the_server_range_before_it_is_sent() {
        let server = Arc::new(RecordingServer::answering(ranked_response("sc_7")));
        let mut sub = submission();
        sub.played_at = -1;
        sub.gauge_value = f32::NAN;
        run_job(Arc::clone(&server), SubmitJob { submission: sub, replay: None });
        let sent = server.submissions();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].played_at, crate::MIN_PLAYED_AT_MS, "a pre-epoch stamp would be a 400");
        assert_eq!(sent[0].gauge_value, crate::FALLBACK_GAUGE_VALUE, "a NaN gauge would serialise to null and be a 400");
    }

    #[test]
    fn a_chart_the_caller_already_set_on_the_replay_is_kept() {
        let other = ChartId { md5: "other".into(), sha256: "other-sha".into() };
        let server = Arc::new(RecordingServer::answering(ranked_response("sc_7")));
        let replay = ReplayData { chart: Some(other.clone()), ..replay() };
        run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: Some(replay) });
        assert_eq!(server.uploads()[0].1.chart, Some(other));
    }

    #[test]
    fn a_server_without_a_score_id_still_uploads_the_replay_unlinked() {
        let response = SubmitResponse { accepted: true, ..Default::default() };
        let server = Arc::new(RecordingServer::answering(response));
        let outcome = run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: Some(replay()) });
        assert_eq!(outcome.replay_id(), Some(UPLOADED_REPLAY_ID));
        assert!(server.uploads()[0].1.score_id.is_none());
    }

    #[test]
    fn an_unranked_submission_skips_the_replay_upload() {
        let response = SubmitResponse { accepted: true, flags: vec!["GUEST".into()], score_id: Some("sc_8".into()), ..Default::default() };
        let server = Arc::new(RecordingServer::answering(response));
        let outcome = run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: Some(replay()) });
        assert!(outcome.is_accepted());
        assert!(outcome.replay.is_none(), "an unranked run has no leaderboard replay to store");
        assert!(server.uploads().is_empty());
    }

    #[test]
    fn a_failed_submission_skips_the_replay_upload() {
        let server = Arc::new(RecordingServer::rejecting());
        let outcome = run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: Some(replay()) });
        assert!(!outcome.is_accepted());
        assert!(matches!(outcome.submit, Err(IrError::Unauthorized(_))), "the submission error reaches the caller");
        assert!(outcome.replay.is_none());
        assert!(server.uploads().is_empty());
    }

    #[test]
    fn a_job_without_a_replay_reports_the_submission_only() {
        let server = Arc::new(RecordingServer::answering(ranked_response("sc_7")));
        let outcome = run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: None });
        assert!(outcome.is_accepted());
        assert!(outcome.replay.is_none());
        assert!(outcome.replay_id().is_none());
        assert!(server.uploads().is_empty());
    }

    #[test]
    fn a_failed_upload_is_reported_without_hiding_the_accepted_score() {
        let mut server = RecordingServer::answering(ranked_response("sc_7"));
        server.upload_fails = true;
        let server = Arc::new(server);
        let outcome = run_job(Arc::clone(&server), SubmitJob { submission: submission(), replay: Some(replay()) });
        assert!(outcome.is_accepted(), "the score still counts even when its replay fails to upload");
        assert!(matches!(outcome.replay, Some(Err(IrError::PayloadTooLarge(_)))), "the upload error reaches the caller");
        assert!(outcome.replay_id().is_none());
    }

    #[test]
    fn replay_upload_is_warranted_only_for_an_accepted_rankable_score() {
        assert!(is_replay_upload_warranted(&SubmitResponse { accepted: true, ..Default::default() }));
        assert!(!is_replay_upload_warranted(&SubmitResponse { accepted: false, ..Default::default() }));
        let unknown_build = SubmitResponse { accepted: true, flags: vec!["UNKNOWN_BUILD".into()], ..Default::default() };
        assert!(is_replay_upload_warranted(&unknown_build), "a non-blocking flag does not stop the upload");
        let future_flag = SubmitResponse { accepted: true, flags: vec!["FUTURE_FLAG".into()], ..Default::default() };
        assert!(is_replay_upload_warranted(&future_flag), "an unrecognised flag is not treated as rank-blocking");
        let assisted = SubmitResponse { accepted: true, flags: vec!["ASSIST".into()], ..Default::default() };
        assert!(!is_replay_upload_warranted(&assisted));
    }

    #[test]
    fn spawned_query_delivers_the_server_result() {
        let server: Arc<dyn ScoreServer> = Arc::new(FixedRankingServer(vec![record("a", 1500), record("b", 1200)]));
        let chart = ChartId { md5: "m".into(), sha256: "s".into() };
        let rx = spawn_query(server, move |s| s.chart_ranking(&chart, 1));
        let rows = rx.recv_timeout(RECV_WAIT).expect("worker replied").expect("query succeeded");
        assert_eq!(rows.len(), 1, "the limit reached the server through the worker");
        assert_eq!(rows[0].ex_score, 1500);
    }

    #[test]
    fn spawned_query_delivers_the_error_instead_of_panicking() {
        let server: Arc<dyn ScoreServer> = Arc::new(NullScoreServer);
        let rx = spawn_query(server, |s| s.health());
        let err = rx.recv_timeout(RECV_WAIT).expect("worker replied").expect_err("offline stub is not configured");
        assert!(matches!(err, IrError::NotConfigured), "got {err:?}");
    }

    /// A server the worker owns for the whole call. `spawn_query` holds the only `Arc`, so this
    /// drops when the worker closure ends — after the result has been handed to the channel, and
    /// mid-unwind if that handover panicked. `served` proves the query ran at all.
    struct WitnessServer {
        receiver_dropped: Arc<AtomicBool>,
        served: Arc<AtomicBool>,
        dropped: Arc<AtomicBool>,
        worker_unwound: Arc<AtomicBool>,
    }

    impl Drop for WitnessServer {
        fn drop(&mut self) {
            self.worker_unwound.store(std::thread::panicking(), Ordering::SeqCst);
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    impl ScoreServer for WitnessServer {
        fn health(&self) -> Result<ServerInfo, IrError> {
            let deadline = Instant::now() + RECV_WAIT;
            while !self.receiver_dropped.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(POLL_INTERVAL);
            }
            self.served.store(true, Ordering::SeqCst);
            Err(IrError::NotConfigured)
        }
        fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
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

    #[test]
    fn dropping_the_receiver_lets_the_worker_finish_without_unwinding() {
        let receiver_dropped = Arc::new(AtomicBool::new(false));
        let served = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let worker_unwound = Arc::new(AtomicBool::new(false));
        let server: Arc<dyn ScoreServer> = Arc::new(WitnessServer {
            receiver_dropped: Arc::clone(&receiver_dropped),
            served: Arc::clone(&served),
            dropped: Arc::clone(&dropped),
            worker_unwound: Arc::clone(&worker_unwound),
        });

        let rx = spawn_query(server, |s| s.health());
        drop(rx);
        receiver_dropped.store(true, Ordering::SeqCst);

        let deadline = Instant::now() + RECV_WAIT;
        while !dropped.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(POLL_INTERVAL);
        }
        assert!(served.load(Ordering::SeqCst), "the query must still run when nobody is listening");
        assert!(dropped.load(Ordering::SeqCst), "the worker never released the server, so it never finished");
        assert!(!worker_unwound.load(Ordering::SeqCst), "the worker unwound while delivering to a dropped receiver");
    }
}
