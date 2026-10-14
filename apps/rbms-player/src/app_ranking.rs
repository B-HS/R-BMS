//! `AppShared` wiring for the IR ranking fetch, the replay download it can start, and the score
//! submission the result screen reports.
//!
//! The panel itself lives in `stage::select`; what is here is the part that keeps running while
//! other screens are up — the worker drains and the caches they feed.
#![allow(clippy::wildcard_imports)]
use std::sync::mpsc::TryRecvError;

use rbms_ir::{
    ChartId, PlayerId, ReplayData, ScoreSubmission, SubmitJob, SubmitVerdict, UploadVerdict, spawn_query, spawn_submit, submit_verdict, upload_verdict,
};
use rbms_store::scoredb::ReplayState;

use crate::ir_ext::PrimaryProfileDirection;
use crate::ir_outcome::{IrReport, IrStatus, format_submit_outcome, short_error};
use crate::ir_ranking::{RankingFetch, RankingState, accept_fetch, fetch_board};
use crate::ir_replay::{replay_download_is_applicable, to_ir_replay, wire_carries_judge};
use crate::record_sync::{RATE_LIMITED_DETAIL, ScoreOutcome, sync_row};
use crate::stage::StageId;
use crate::*;

/// The play a submission on its way to the main server was made for, and where to write its answer
/// down: the server and the account it was sent under, and what is to become of its replay when the
/// server takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LiveSubmit {
    pub(crate) log_id: i64,
    pub(crate) server: String,
    pub(crate) login_id: String,
    /// [`ReplayState::None`] for a run that kept no replay, [`ReplayState::Skipped`] for one the
    /// upload cannot state the widths of, and [`ReplayState::Pending`] otherwise.
    pub(crate) replay_plan: ReplayState,
}

/// Why a run was not sent to the main score server: nobody is signed in to it, and the server takes
/// a score only from an account (a tokenless submission is refused 401). The run is kept locally.
const NOT_SIGNED_IN: &str = "not logged in";

impl AppShared {
    /// Chart id of the focused row, for the IR (only the md5 addresses a chart on the wire).
    pub(crate) fn focused_chart_id(&self) -> Option<ChartId> {
        self.focused_md5().map(|md5| ChartId { md5, sha256: String::new() })
    }

    /// Ask the server for a chart's board on a worker thread, unless the cache already holds a
    /// settled answer for it.
    ///
    /// Only one fetch is ever in flight: a second one would drop the first receiver, stranding that
    /// chart's `Loading` placeholder in the cache and leaving the panel on LOADING forever.
    pub(crate) fn start_ranking_fetch(&mut self, chart: ChartId) {
        self.ranking_requested = Some(chart.md5.clone());
        self.ranking_generation = self.ranking_generation.wrapping_add(1);
        if self.ranking_cache.has_settled_answer(&chart.md5) {
            return;
        }
        let generation = self.ranking_generation;
        let md5 = chart.md5.clone();
        let player = PlayerId { id: self.config.network.player_id.clone() };
        let rivals = self.config.network.rivals.clone();
        self.ranking_cache.insert(md5.clone(), RankingState::Loading);
        let source = self.primary_ir_server();
        let rx = spawn_query(source, move |server| {
            let state = fetch_board(server, &chart, &player, &rivals);
            Ok::<RankingFetch, rbms_ir::IrError>(RankingFetch { generation, md5, state })
        });
        self.ranking_rx = Some(rx);
    }

    /// The server the ranking panel reads and a course is submitted to: the primary profile, or the
    /// legacy single server when no profile is enabled.
    pub(crate) fn primary_ir_server(&self) -> Arc<dyn ScoreServer> {
        self.multi_ir.primary_server().and_then(|index| self.profile_servers.get(index).cloned()).unwrap_or_else(|| self.server.clone())
    }

    pub(crate) fn has_primary_ir_server(&self) -> bool {
        self.multi_ir.primary_server().is_some()
    }

    /// Whether a run or a course handed over now reaches the primary server. The legacy single
    /// server takes nothing while nobody is signed in ([`AppShared::spawn_score_submit`]); an
    /// `ir_profiles` entry carries its own token and is always sent to.
    pub(crate) fn sends_to_primary_ir_server(&self) -> bool {
        match self.multi_ir.primary_server() {
            None => false,
            Some(primary) if self.multi_ir.legacy_index() == Some(primary) => self.session.is_logged_in(),
            Some(_) => true,
        }
    }

    pub(crate) fn primary_ir_profile(&self) -> Option<(String, usize, usize)> {
        self.multi_ir.primary_profile()
    }

    pub(crate) fn can_switch_primary_ir(&self) -> bool {
        self.multi_ir.can_switch_primary()
    }

    pub(crate) fn shift_primary_ir(&mut self, direction: PrimaryProfileDirection) -> bool {
        if !self.multi_ir.shift_primary(direction) {
            return false;
        }
        self.reset_ranking();
        true
    }

    pub(crate) fn reset_ranking(&mut self) {
        self.ranking_cache.clear();
        self.ranking_requested = None;
        self.ranking_rx = None;
        self.ranking_generation = self.ranking_generation.wrapping_add(1);
    }

    /// The server a finished course is submitted to. Courses go to the primary profile only.
    pub(crate) fn course_ir_server(&self) -> Arc<dyn ScoreServer> {
        self.primary_ir_server()
    }

    /// Hand a finished score submission to the IR worker, uploading the run's replay alongside it
    /// when AUTO UPLOAD REPLAY is on and the account can store one.
    ///
    /// Nothing is sent while nobody is signed in: the server would refuse it, and every run would end
    /// on a failed submission. A player who has the server set and has not signed in -- which is
    /// every new install, the server being set by default -- sees the run skipped for that reason.
    ///
    /// `played` names the stored play the run is and what its replay plan is, so the server's answer
    /// is written down against it ([`AppShared::poll_submit`]); a run that was not stored has none.
    pub(crate) fn spawn_score_submit(&mut self, submission: ScoreSubmission, replay: Option<ReplayData>, played: Option<(i64, ReplayState)>) {
        self.submit_log = None;
        if !self.session.is_logged_in() {
            self.ir_status = IrStatus::Skipped(NOT_SIGNED_IN);
            return;
        }
        self.submit_log =
            played.and_then(|(log_id, replay_plan)| self.record_sync_target().map(|(server, login_id)| LiveSubmit { log_id, server, login_id, replay_plan }));
        self.ir_status = IrStatus::Sending;
        self.submit_rx = Some(spawn_submit(self.server.clone(), SubmitJob { submission, replay }));
    }

    /// Submit the same run to every extra IR profile, in parallel and off the frame thread.
    ///
    pub(crate) fn spawn_profile_submits(&mut self, submission: &ScoreSubmission) {
        let secondary = self.multi_ir.secondary();
        if secondary.enabled_profiles().next().is_none() {
            return;
        }
        self.profile_submit_rx = Some(spawn_submit_all(secondary, self.profile_servers.clone(), submission.clone()));
    }

    /// The replay payload to upload with this run, or `None` when the setting is off, no replay was
    /// recorded, the client is a guest (the server rejects anonymous replay uploads), or the run was
    /// judged by widths the upload cannot state ([`wire_carries_judge`]), which would download as
    /// another run.
    pub(crate) fn replay_upload_payload(&self, replay: &Replay, chart: &ChartId, lntype: i32) -> Option<ReplayData> {
        if !self.config.network.auto_upload_replay || !self.session.is_logged_in() || replay.events.is_empty() || !wire_carries_judge(&replay.judge) {
            return None;
        }
        Some(to_ir_replay(replay, chart, self.config.play.constant_speed, lntype, self.build_sha256.clone()))
    }

    /// Drain the ranking and submission workers, and drop a replay download the player has walked
    /// away from. Called once per frame; the download itself is taken by the select screen, the
    /// only place it can be started.
    pub(crate) fn poll_ir_jobs(&mut self, stage: StageId) {
        self.poll_ranking();
        if !replay_download_is_applicable(stage) && self.replay_download_rx.take().is_some() {
            self.replay_download_target = None;
            self.net_status = "replay download dropped: left song select".to_string();
        }
        self.poll_submit();
        self.poll_profile_submits();
        self.poll_record_sync();
    }

    fn poll_ranking(&mut self) {
        let Some(rx) = self.ranking_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(fetch)) => {
                accept_fetch(&mut self.ranking_cache, self.ranking_generation, fetch);
            }
            Ok(Err(error)) => {
                self.forget_pending_ranking();
                self.net_status = format!("ranking: {}", short_error(&error));
            }
            Err(TryRecvError::Empty) => self.ranking_rx = Some(rx),
            Err(TryRecvError::Disconnected) => {
                self.forget_pending_ranking();
                self.net_status = "ranking worker stopped".to_string();
            }
        }
    }

    /// Drop the `Loading` placeholder of the fetch that just failed or vanished, so the chart is
    /// asked for again the next time it is focused instead of showing LOADING for the rest of the
    /// session.
    fn forget_pending_ranking(&mut self) {
        if let Some(md5) = self.ranking_requested.take() {
            self.ranking_cache.remove(&md5);
        }
    }

    /// Collect the extra profiles' fan-out and report how many of them took the score.
    fn poll_profile_submits(&mut self) {
        let Some(rx) = self.profile_submit_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(results) => {
                let report = profile_submit_report(&results);
                self.net_status = report.score.clone();
                if self.submit_rx.is_none() && self.ir_status.accepts_report() {
                    self.ir_status = IrStatus::Reported(report);
                }
            }
            Err(TryRecvError::Empty) => self.profile_submit_rx = Some(rx),
            Err(TryRecvError::Disconnected) => {
                let report = IrReport { score: "IR ERROR: profile submit worker stopped".to_string(), new_best: false, replay: None, failed: true };
                self.net_status = report.score.clone();
                if self.submit_rx.is_none() && self.ir_status.accepts_report() {
                    self.ir_status = IrStatus::Reported(report);
                }
            }
        }
    }

    fn poll_submit(&mut self) {
        let Some(rx) = self.submit_rx.take() else {
            return;
        };
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => {
                self.submit_rx = Some(rx);
                return;
            }
            Err(TryRecvError::Disconnected) => rbms_ir::SubmitOutcome { submit: Err(rbms_ir::IrError::Network("submit worker stopped".into())), replay: None },
        };
        if let Some(live) = self.submit_log.take() {
            self.write_live_outcome(&live, &outcome);
        }
        if self.ir_status.accepts_report() {
            self.ir_status = IrStatus::Reported(format_submit_outcome(&outcome));
        }
    }
}

impl AppShared {
    /// Write down what the main server said of a run just played, as a records sync writes down what
    /// it said of a play sent again, so the next sync sends the run only if the server does not hold
    /// it. A failure that would stop a sync is a run to send again here.
    pub(crate) fn write_live_outcome(&mut self, live: &LiveSubmit, outcome: &rbms_ir::SubmitOutcome) {
        let score = match submit_verdict(&outcome.submit) {
            SubmitVerdict::Accepted { score_id, already, replay_warranted } => {
                let replay = match (&outcome.replay, live.replay_plan) {
                    (Some(result), _) => match upload_verdict(result) {
                        UploadVerdict::Uploaded { .. } => ReplayState::Uploaded,
                        UploadVerdict::Rejected(_) => ReplayState::Rejected,
                        UploadVerdict::RateLimited | UploadVerdict::Transient { .. } | UploadVerdict::StopSync { .. } => ReplayState::Failed,
                    },
                    (None, ReplayState::Pending) if !replay_warranted => ReplayState::Skipped,
                    (None, plan) => plan,
                };
                ScoreOutcome::Accepted { score_id, already, replay }
            }
            SubmitVerdict::Rejected(rejection) => ScoreOutcome::Rejected(rejection),
            SubmitVerdict::RateLimited => ScoreOutcome::Retry { detail: RATE_LIMITED_DETAIL.to_string() },
            SubmitVerdict::Transient { detail } | SubmitVerdict::StopSync { detail } => ScoreOutcome::Retry { detail },
        };
        let mut row = sync_row(live.log_id, &live.server, &live.login_id, &score);
        match &outcome.replay {
            Some(Ok(id)) => row.replay_id = Some(id.clone()),
            Some(Err(error)) => row.replay_error = Some(error.to_string()),
            None => {}
        }
        self.write_sync_row(row);
    }
}

fn profile_submit_report(results: &[ProfileResult<rbms_ir::SubmitResponse>]) -> IrReport {
    IrReport {
        score: submit_summary(results),
        new_best: false,
        replay: None,
        failed: results.is_empty() || results.iter().any(|(_, result)| !matches!(result, Ok(response) if response.accepted)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared(config: Config) -> AppShared {
        let dir = std::env::temp_dir().join(format!("rbms-app-ranking-tests-{}", std::process::id()));
        App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron")).shared
    }

    #[test]
    fn no_enabled_ir_profile_keeps_the_player_offline() {
        let shared = shared(Config::default());

        assert!(!shared.has_primary_ir_server());
    }

    /// A player who has not signed in sends nothing to the main server, which would refuse it, and
    /// the result says why rather than reporting a failure.
    #[test]
    fn a_guest_run_is_not_sent_to_the_main_server() {
        let mut shared = shared(Config::default());
        assert!(!shared.session.is_logged_in());
        let submission = crate::ir_ext::tests::submission();
        shared.spawn_score_submit(submission.clone(), None, None);
        assert_eq!(shared.ir_status, IrStatus::Skipped(NOT_SIGNED_IN));
        assert!(shared.submit_rx.is_none(), "a guest run was handed to the submit worker");

        shared.session = crate::ir_session::AccountSession::restored(Some("token".into()), Some("dj".into()));
        shared.spawn_score_submit(submission, None, None);
        assert_eq!(shared.ir_status, IrStatus::Sending, "a signed-in run is sent");
    }

    #[test]
    fn an_extra_only_profile_is_the_ranking_primary() {
        let mut config = Config::default();
        config.network.ir_profiles.push(rbms_config::IrProfile {
            name: "EXTRA".to_string(),
            base_url: "http://extra.invalid".to_string(),
            token: None,
            enabled: true,
        });
        let shared = shared(config);

        assert!(shared.has_primary_ir_server());
        assert!(Arc::ptr_eq(&shared.primary_ir_server(), &shared.profile_servers[0]));
    }

    #[test]
    fn the_legacy_server_remains_the_ranking_primary_with_extra_profiles() {
        let mut config = Config::default();
        config.network.server_url = Some("http://main.invalid".to_string());
        config.network.ir_profiles.push(rbms_config::IrProfile {
            name: "EXTRA".to_string(),
            base_url: "http://extra.invalid".to_string(),
            token: None,
            enabled: true,
        });
        let shared = shared(config);

        assert!(shared.has_primary_ir_server());
        assert!(Arc::ptr_eq(&shared.primary_ir_server(), &shared.server));
    }

    #[test]
    fn changing_the_primary_drops_every_ranking_answer_and_worker() {
        let mut config = Config::default();
        config.network.server_url = Some("http://main.invalid".to_string());
        config.network.ir_profiles.push(rbms_config::IrProfile {
            name: "EXTRA".to_string(),
            base_url: "http://extra.invalid".to_string(),
            token: None,
            enabled: true,
        });
        let mut shared = shared(config);
        shared.ranking_cache.insert("loading".to_string(), RankingState::Loading);
        shared.ranking_cache.insert("ready".to_string(), RankingState::Failed("old server".to_string()));
        shared.ranking_requested = Some("loading".to_string());
        let (_tx, rx) = std::sync::mpsc::channel::<Result<RankingFetch, rbms_ir::IrError>>();
        shared.ranking_rx = Some(rx);
        let generation = shared.ranking_generation;

        assert!(shared.shift_primary_ir(PrimaryProfileDirection::Next));

        assert!(Arc::ptr_eq(&shared.primary_ir_server(), &shared.profile_servers[1]));
        assert!(shared.ranking_cache.peek("loading").is_none());
        assert!(shared.ranking_cache.peek("ready").is_none());
        assert!(shared.ranking_requested.is_none());
        assert!(shared.ranking_rx.is_none());
        assert_eq!(shared.ranking_generation, generation.wrapping_add(1));
    }

    #[test]
    fn rebuilding_the_server_drops_every_ranking_answer_and_worker() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("loading".to_string(), RankingState::Loading);
        shared.ranking_cache.insert("ready".to_string(), RankingState::Failed("old server".to_string()));
        shared.ranking_requested = Some("loading".to_string());
        let (_tx, rx) = std::sync::mpsc::channel::<Result<RankingFetch, rbms_ir::IrError>>();
        shared.ranking_rx = Some(rx);
        let generation = shared.ranking_generation;

        shared.rebuild_server();

        assert!(shared.ranking_cache.peek("loading").is_none());
        assert!(shared.ranking_cache.peek("ready").is_none());
        assert!(shared.ranking_requested.is_none());
        assert!(shared.ranking_rx.is_none());
        assert_eq!(shared.ranking_generation, generation.wrapping_add(1));
    }

    #[test]
    fn changing_the_primary_needs_two_enabled_profiles() {
        let mut shared = shared(Config::default());
        let generation = shared.ranking_generation;

        assert!(!shared.shift_primary_ir(PrimaryProfileDirection::Next));
        assert_eq!(shared.ranking_generation, generation);
    }

    #[test]
    fn an_extra_only_profile_submission_reports_its_fan_out() {
        let results = vec![("EXTRA".to_string(), Ok(rbms_ir::SubmitResponse { accepted: true, ..Default::default() }))];

        let report = profile_submit_report(&results);

        assert_eq!(report.score, "IR 1/1");
        assert!(!report.failed);
    }

    #[test]
    fn a_rejected_extra_profile_does_not_report_success() {
        let results = vec![("EXTRA".to_string(), Ok(rbms_ir::SubmitResponse::default()))];

        let report = profile_submit_report(&results);

        assert!(report.failed);
        assert_eq!(report.score, "IR 0/1 — EXTRA: rejected");
    }

    #[test]
    fn an_extra_only_submit_finishes_its_sending_status() {
        let mut shared = shared(Config::default());
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(vec![("EXTRA".to_string(), Ok(rbms_ir::SubmitResponse { accepted: true, ..Default::default() }))]).expect("profile worker receiver is open");
        shared.ir_status = IrStatus::Sending;
        shared.profile_submit_rx = Some(rx);

        shared.poll_profile_submits();

        assert!(matches!(shared.ir_status, IrStatus::Reported(report) if report.score == "IR 1/1" && !report.failed));
    }

    #[test]
    fn a_legacy_submit_remains_authoritative_while_extra_profiles_finish() {
        let mut shared = shared(Config::default());
        let (profile_tx, profile_rx) = std::sync::mpsc::channel();
        let (_legacy_tx, legacy_rx) = std::sync::mpsc::channel::<rbms_ir::SubmitOutcome>();
        profile_tx
            .send(vec![("EXTRA".to_string(), Ok(rbms_ir::SubmitResponse { accepted: true, ..Default::default() }))])
            .expect("profile worker receiver is open");
        shared.ir_status = IrStatus::Sending;
        shared.submit_rx = Some(legacy_rx);
        shared.profile_submit_rx = Some(profile_rx);

        shared.poll_profile_submits();

        assert!(matches!(shared.ir_status, IrStatus::Sending));
        assert_eq!(shared.net_status, "IR 1/1");
    }
}
