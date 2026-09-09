use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::*;
    use crate::null::NullScoreServer;
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
