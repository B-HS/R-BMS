//! `AppShared` wiring for the IR ranking fetch, the replay download it can start, and the score
//! submission the result screen reports.
//!
//! The panel itself lives in `stage::select`; what is here is the part that keeps running while
//! other screens are up — the worker drains and the caches they feed.
#![allow(clippy::wildcard_imports)]
use std::sync::mpsc::TryRecvError;

use std::sync::mpsc::Receiver;

use rbms_ir::{
    ChartId, CourseSubmission, IrError, PlayerId, ReplayData, ScoreSubmission, SubmitJob, SubmitResponse, SubmitVerdict, UploadVerdict, spawn_query,
    spawn_submit, submit_verdict, upload_verdict,
};
use rbms_store::scoredb::ReplayState;

use crate::ir_ext::PrimaryProfileDirection;
use crate::ir_outcome::{IrReport, IrStatus, format_submit_outcome, short_error};
use crate::ir_ranking::{RankingFetch, RankingState, accept_fetch, fetch_board, fetch_board_page};
use crate::ir_replay::{replay_download_is_applicable, to_ir_replay, wire_carries_judge};
use crate::notify::notify_text;
use crate::record_sync::{RATE_LIMITED_DETAIL, ScoreOutcome, course_outcome, course_sync_row, sync_row};
use crate::stage::StageId;
use crate::text::notices::{NoticeText, StatusText, installed, installed_with};
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
    /// Whether the play was stored with its trace, which goes up once the server takes the score.
    pub(crate) trace_kept: bool,
}

/// The stored play a run handed to the main server is: its row, what is to become of its replay,
/// and whether it was stored with its trace ([`LiveSubmit`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlayedRun {
    pub(crate) log_id: i64,
    pub(crate) replay_plan: ReplayState,
    pub(crate) trace_kept: bool,
}

/// A finished course on its way to the main server: where its answer comes back, and the kept
/// course and the server and account its answer is written down against, when it was kept and the
/// main server is the one the records sync sends to.
pub(crate) struct CourseSubmit {
    rx: Receiver<Result<SubmitResponse, IrError>>,
    kept: Option<(i64, String, String)>,
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
    /// settled answer for it: a board fetched less than ten minutes ago, or a failure less than
    /// thirty seconds old.
    ///
    /// Only one fetch is ever in flight: a second one would drop the first receiver, stranding that
    /// chart's `Loading` placeholder in the cache and leaving the panel on LOADING forever.
    pub(crate) fn start_ranking_fetch(&mut self, chart: ChartId) {
        self.ranking_requested = Some(chart.md5.clone());
        self.ranking_generation = self.ranking_generation.wrapping_add(1);
        if self.ranking_cache.has_settled_answer(&chart.md5, Instant::now()) {
            return;
        }
        let generation = self.ranking_generation;
        let md5 = chart.md5.clone();
        let player = PlayerId { id: crate::ir_session::submission_player_id(&self.session) };
        let rivals = self.config.network.rivals.clone();
        self.ranking_cache.insert(md5.clone(), RankingState::Loading);
        let source = self.primary_ir_server();
        let rx = spawn_query(source, move |server| {
            let state = fetch_board(server, &chart, &player, &rivals);
            Ok::<RankingFetch, rbms_ir::IrError>(RankingFetch { generation, md5, state, extends: false })
        });
        self.ranking_rx = Some(rx);
    }

    /// Ask the server for one more page of a chart's leaderboard, to be added to the board the cache
    /// holds for it: page `board.pages + 1` for the rows that follow the ones held, or a page
    /// further down, which the board keeps apart (`your_page`). Returns whether a request was made.
    ///
    /// Nothing is asked while another fetch is in flight, while the chart has no finished board, or
    /// for a page the board holds or cannot be given ([`crate::ir_ranking::RankingBoard::wants_page`]).
    /// The board stays as it is while the page is on its way, and when the request fails: the failure
    /// is reported in the network status and the board is kept.
    pub(crate) fn start_ranking_page(&mut self, chart: ChartId, page: u32) -> bool {
        if self.ranking_rx.is_some() {
            return false;
        }
        let Some(RankingState::Ready(board)) = self.ranking_cache.peek(&chart.md5) else {
            return false;
        };
        if !board.wants_page(page) {
            return false;
        }
        let board = (**board).clone();
        self.ranking_requested = Some(chart.md5.clone());
        self.ranking_generation = self.ranking_generation.wrapping_add(1);
        let generation = self.ranking_generation;
        let md5 = chart.md5.clone();
        let source = self.primary_ir_server();
        let rx = spawn_query(source, move |server| {
            let grown = fetch_board_page(server, &chart, board, page)?;
            Ok(RankingFetch { generation, md5, state: RankingState::Ready(Box::new(grown)), extends: true })
        });
        self.ranking_rx = Some(rx);
        true
    }

    /// Fetch the page the local player's row is on, once the first page of a chart's board has come
    /// back without it.
    fn continue_ranking_toward_you(&mut self, md5: &str) {
        let Some(RankingState::Ready(board)) = self.ranking_cache.peek(md5) else {
            return;
        };
        let Some(page) = board.page_to_find_you() else {
            return;
        };
        self.start_ranking_page(ChartId { md5: md5.to_string(), sha256: String::new() }, page);
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
    /// `played` names the stored play the run is, what its replay plan is and whether it kept a
    /// trace, so the server's answer is written down against it ([`AppShared::poll_submit`]); a run
    /// that was not stored has none.
    pub(crate) fn spawn_score_submit(&mut self, submission: ScoreSubmission, replay: Option<ReplayData>, played: Option<PlayedRun>) {
        self.submit_log = None;
        if !self.session.is_logged_in() {
            self.ir_status = IrStatus::Skipped(NOT_SIGNED_IN);
            return;
        }
        self.submit_log = played.and_then(|played| {
            self.record_sync_target().map(|(server, login_id)| LiveSubmit {
                log_id: played.log_id,
                server,
                login_id,
                replay_plan: played.replay_plan,
                trace_kept: played.trace_kept,
            })
        });
        self.ir_status = IrStatus::Sending;
        self.ranking_cache.await_score_of(&submission.chart.id.md5);
        self.submit_rx = Some(spawn_submit(self.server.clone(), SubmitJob { submission, replay }));
    }

    /// Submit the same run to every extra IR profile, in parallel and off the frame thread.
    ///
    pub(crate) fn spawn_profile_submits(&mut self, submission: &ScoreSubmission) {
        let secondary = self.multi_ir.secondary();
        if secondary.enabled_profiles().next().is_none() {
            return;
        }
        self.ranking_cache.await_score_of(&submission.chart.id.md5);
        self.profile_submit_rx = Some(spawn_submit_all(secondary, self.profile_servers.clone(), submission.clone()));
    }

    /// Forget the board cached for the chart of the score the server has just taken, so the next
    /// time the chart is looked at its ranking is asked for again and shows the score. A fetch of
    /// that chart still on its way is dropped with it: it may have been answered before the score
    /// was stored.
    fn forget_ranking_of_taken_score(&mut self) {
        let Some(md5) = self.ranking_cache.awaited_score_chart().map(str::to_string) else {
            return;
        };
        self.ranking_cache.remove(&md5);
        if self.ranking_requested.as_deref() == Some(md5.as_str()) {
            self.ranking_requested = None;
            self.ranking_rx = None;
            self.ranking_generation = self.ranking_generation.wrapping_add(1);
        }
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
        self.poll_ranking(Instant::now());
        if !replay_download_is_applicable(stage) && self.replay_download_rx.take().is_some() {
            self.replay_download_target = None;
            self.net_status = installed(StatusText::ReplayDownloadDropped).to_string();
        }
        self.poll_submit();
        self.poll_profile_submits();
        self.poll_course_submits();
        self.poll_record_sync();
        self.poll_trace_backfill(stage);
        self.poll_saves();
    }

    /// Take the answer of the ranking fetch in flight, if it has come, and let a failed chart be
    /// asked for again once its retry delay is over.
    ///
    /// The first answer of a chart whose local player stands below the page it holds starts the fetch
    /// of the page they stand on.
    fn poll_ranking(&mut self, now: Instant) {
        let Some(rx) = self.ranking_rx.take() else {
            self.retry_failed_ranking(now);
            return;
        };
        match rx.try_recv() {
            Ok(Ok(fetch)) => {
                let (md5, first_answer) = (fetch.md5.clone(), !fetch.extends);
                if accept_fetch(&mut self.ranking_cache, self.ranking_generation, fetch, now) && first_answer {
                    self.continue_ranking_toward_you(&md5);
                }
            }
            Ok(Err(error)) => {
                self.forget_pending_ranking();
                self.net_status = installed_with(StatusText::RankingError, &[&short_error(&error)]);
            }
            Err(TryRecvError::Empty) => self.ranking_rx = Some(rx),
            Err(TryRecvError::Disconnected) => {
                self.forget_pending_ranking();
                self.net_status = installed(StatusText::RankingWorkerStopped).to_string();
            }
        }
    }

    /// Let the chart the last fetch was for be requested again when that fetch failed and the
    /// retry delay is over, so a chart that stays focused is not left on its failure.
    fn retry_failed_ranking(&mut self, now: Instant) {
        let due = self.ranking_requested.as_deref().is_some_and(|md5| self.ranking_cache.failure_is_due(md5, now));
        if due {
            self.ranking_requested = None;
        }
    }

    /// Drop the `Loading` placeholder of the fetch that just failed or vanished, so the chart is
    /// asked for again the next time it is focused instead of showing LOADING for the rest of the
    /// session. A board that was only to grow by a page is kept: it is a finished answer.
    fn forget_pending_ranking(&mut self) {
        if let Some(md5) = self.ranking_requested.take()
            && matches!(self.ranking_cache.peek(&md5), Some(RankingState::Loading))
        {
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
                if results.iter().any(|(_, result)| matches!(result, Ok(response) if response.accepted)) {
                    self.forget_ranking_of_taken_score();
                }
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
        if matches!(submit_verdict(&outcome.submit), SubmitVerdict::Accepted { .. }) {
            self.forget_ranking_of_taken_score();
        }
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
    ///
    /// A run the server took with its trace kept has the trace still to go up, and a records sync of
    /// that run alone sends it, while AUTO UPLOAD TRACE is on: the result screen's submission carries
    /// the score and the replay, and the trace follows the score it is linked to.
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
                let trace = if live.trace_kept { ReplayState::Pending } else { ReplayState::None };
                ScoreOutcome::Accepted { score_id, already, replay, trace }
            }
            SubmitVerdict::Rejected(rejection) => ScoreOutcome::Rejected(rejection),
            SubmitVerdict::RateLimited => ScoreOutcome::Retry { detail: RATE_LIMITED_DETAIL.to_string() },
            SubmitVerdict::Transient { detail } | SubmitVerdict::StopSync { detail } => ScoreOutcome::Retry { detail },
        };
        let refused_under = self.refusal_key();
        let mut row = sync_row(live.log_id, &live.server, &live.login_id, &score, &refused_under);
        match &outcome.replay {
            Some(Ok(id)) => row.replay_id = Some(id.clone()),
            Some(Err(error)) => row.replay_error = Some(error.to_string()),
            None => {}
        }
        let trace_to_send = row.trace_state == ReplayState::Pending && self.config.network.auto_upload_trace;
        self.write_sync_row(row);
        if trace_to_send {
            self.start_record_sync(crate::record_sync::SyncScope::Logs(vec![live.log_id]));
        }
    }
}

impl AppShared {
    /// Keep a finished course in the score database, and send it to the main server when somebody
    /// is signed in to it. The course is kept whether or not it is sent now: a course the server does
    /// not answer, or one finished signed out, is sent by a later records sync, exactly as it was
    /// first sent, until the server holds it.
    pub(crate) fn keep_and_send_course(&mut self, sub: CourseSubmission) {
        let kept = self.keep_course(&sub);
        if !self.sends_to_primary_ir_server() {
            return;
        }
        let main_is_synced = self.multi_ir.primary_server().is_some() && self.multi_ir.primary_server() == self.multi_ir.legacy_index();
        let kept = kept.filter(|_| main_is_synced).zip(self.record_sync_target()).map(|(id, (server, login_id))| (id, server, login_id));
        let server = self.course_ir_server();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(server.submit_course(&sub));
        });
        self.course_submits.push(CourseSubmit { rx, kept });
    }

    /// Keep a finished course in the score database for a records sync to send, and answer the row
    /// it was kept as. A course the database cannot take is reported and not kept.
    pub(crate) fn keep_course(&mut self, sub: &CourseSubmission) -> Option<i64> {
        let played_as = self.session.is_logged_in().then(|| self.session.login_id()).flatten().map(str::to_string);
        match (sub.to_json(), self.scoredb.as_mut()) {
            (Some(body), Some(db)) => match db.put_course(&sub.course_hash, sub.played_at, &body, played_as.as_deref()) {
                Ok(id) => Some(id),
                Err(e) => {
                    notify(Level::Error, format!("course not kept: {e}"));
                    None
                }
            },
            (None, _) => {
                notify(Level::Warn, "course not kept: it does not serialise");
                None
            }
            (Some(_), None) => {
                notify_text(Level::Error, NoticeText::CourseNotKeptScoreDatabaseClosed, &[]);
                None
            }
        }
    }

    /// Take the answers of the courses sent since the last frame: write each down against its kept
    /// course, and say what became of it.
    fn poll_course_submits(&mut self) {
        let mut answered = Vec::new();
        self.course_submits.retain(|submit| match submit.rx.try_recv() {
            Ok(result) => {
                answered.push((result, submit.kept.clone()));
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                answered.push((Err(IrError::Network("course submit worker stopped".into())), submit.kept.clone()));
                false
            }
        });
        for (result, kept) in answered {
            let outcome = course_outcome(&result);
            match &outcome {
                ScoreOutcome::Accepted { .. } => notify_text(Level::Info, NoticeText::CourseSent, &[]),
                ScoreOutcome::Rejected(rejection) => notify(Level::Warn, format!("course rejected: {}", rejection.detail)),
                ScoreOutcome::Retry { detail } => notify(Level::Warn, format!("course not sent yet ({detail}): sent again on the next records sync")),
                ScoreOutcome::Skipped { .. } => {}
            }
            if let Some((course_id, server, login_id)) = kept {
                let row = course_sync_row(course_id, &server, &login_id, &outcome, &self.refusal_key());
                self.write_course_sync_row(&row);
            }
        }
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
    use crate::ir_ranking::tests::{RankingDouble, record};
    use crate::ir_ranking::{RANKING_FAILED_RETRY_AFTER, RANKING_PAGE_LIMIT, RANKING_READY_LIFETIME, RankingBoard, build_board};

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

    fn chart_of(md5: &str) -> ChartId {
        ChartId { md5: md5.to_string(), sha256: String::new() }
    }

    /// An app whose primary server is `server`, signed in as `me`.
    fn shared_reading(server: RankingDouble) -> AppShared {
        let mut shared = shared(Config::default());
        shared.server = Arc::new(server);
        shared.session = crate::ir_session::AccountSession::restored(Some("token".into()), Some("me".into()));
        shared
    }

    /// Wait for the ranking fetch in flight to answer, then take its answer as the next frame would.
    fn take_the_answer_in_flight(shared: &mut AppShared, now: Instant) {
        let rx = shared.ranking_rx.take().expect("a fetch is in flight");
        let answer = rx.recv().expect("the worker answers");
        let (tx, delivered) = std::sync::mpsc::channel();
        tx.send(answer).expect("the receiver is open");
        shared.ranking_rx = Some(delivered);
        shared.poll_ranking(now);
    }

    fn ready_board_of(rows: u32) -> RankingState {
        let records: Vec<_> = (0..rows).map(|n| record(&format!("p{n}"), &format!("Player {n}"), 2000 - n, None)).collect();
        RankingState::Ready(Box::new(build_board(&records, None, &[])))
    }

    fn board_in(shared: &AppShared, md5: &str) -> RankingBoard {
        let Some(RankingState::Ready(board)) = shared.ranking_cache.peek(md5) else { panic!("{md5} has no finished board") };
        (**board).clone()
    }

    fn accepted_outcome() -> rbms_ir::SubmitOutcome {
        rbms_ir::SubmitOutcome { submit: Ok(rbms_ir::SubmitResponse { accepted: true, ..Default::default() }), replay: None }
    }

    fn submitted_through(shared: &mut AppShared, outcome: rbms_ir::SubmitOutcome) {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(outcome).expect("the receiver is open");
        shared.ir_status = IrStatus::Sending;
        shared.submit_rx = Some(rx);
        shared.poll_submit();
    }

    #[test]
    fn a_failed_ranking_is_requested_again_once_its_retry_delay_is_over() {
        let mut shared = shared(Config::default());
        let before = Instant::now();
        shared.ranking_cache.insert("a".into(), RankingState::Failed("down".into()));
        shared.ranking_requested = Some("a".into());

        shared.poll_ranking(before);
        assert_eq!(shared.ranking_requested.as_deref(), Some("a"), "the failure is fresh");

        shared.poll_ranking(before + RANKING_FAILED_RETRY_AFTER * 2);
        assert!(shared.ranking_requested.is_none(), "the chart can be asked for again");
        assert!(matches!(shared.ranking_cache.peek("a"), Some(RankingState::Failed(_))), "the failure shows until the new fetch replaces it");
    }

    #[test]
    fn a_board_is_not_requested_again_by_the_failure_retry() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("a".into(), ready_board_of(1));
        shared.ranking_requested = Some("a".into());

        shared.poll_ranking(Instant::now() + RANKING_FAILED_RETRY_AFTER * 2);

        assert_eq!(shared.ranking_requested.as_deref(), Some("a"));
    }

    #[test]
    fn a_failed_ranking_is_not_retried_while_a_fetch_is_in_flight() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("a".into(), RankingState::Failed("down".into()));
        shared.ranking_requested = Some("a".into());
        let (_tx, rx) = std::sync::mpsc::channel::<Result<RankingFetch, rbms_ir::IrError>>();
        shared.ranking_rx = Some(rx);

        shared.poll_ranking(Instant::now() + RANKING_FAILED_RETRY_AFTER * 2);

        assert_eq!(shared.ranking_requested.as_deref(), Some("a"));
        assert!(shared.ranking_rx.is_some(), "the fetch in flight is still waited for");
    }

    #[test]
    fn a_failed_fetch_forgets_its_loading_placeholder_but_not_a_board_that_was_to_grow() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("first".into(), RankingState::Loading);
        shared.ranking_requested = Some("first".into());
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Err(rbms_ir::IrError::Network("down".into()))).expect("the receiver is open");
        shared.ranking_rx = Some(rx);
        shared.poll_ranking(Instant::now());
        assert!(shared.ranking_cache.peek("first").is_none(), "a placeholder with no fetch behind it would read LOADING for ever");

        shared.ranking_cache.insert("grown".into(), ready_board_of(1));
        shared.ranking_requested = Some("grown".into());
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Err(rbms_ir::IrError::Network("down".into()))).expect("the receiver is open");
        shared.ranking_rx = Some(rx);
        shared.poll_ranking(Instant::now());
        assert!(shared.ranking_cache.peek("grown").is_some(), "the page failed, the board it was to grow is still good");
        assert!(shared.net_status.starts_with("ranking: "), "{}", shared.net_status);
    }

    #[test]
    fn a_page_is_asked_for_only_when_it_adds_rows_and_nothing_else_is_in_flight() {
        let mut shared = shared_reading(RankingDouble::new(Vec::new()));
        assert!(!shared.start_ranking_page(chart_of("m"), 2), "a chart with no board has no page to grow");

        shared.ranking_cache.insert("m".into(), RankingState::Loading);
        assert!(!shared.start_ranking_page(chart_of("m"), 2), "a board still on its way has no page to grow");

        shared.ranking_cache.insert("m".into(), ready_board_of(3));
        assert!(!shared.start_ranking_page(chart_of("m"), 2), "a short first page is the whole board");

        shared.ranking_cache.insert("m".into(), ready_board_of(RANKING_PAGE_LIMIT));
        assert!(!shared.start_ranking_page(chart_of("m"), 1), "the board holds the first page");

        let (_tx, rx) = std::sync::mpsc::channel::<Result<RankingFetch, rbms_ir::IrError>>();
        shared.ranking_rx = Some(rx);
        assert!(!shared.start_ranking_page(chart_of("m"), 2), "one fetch at a time");
        shared.ranking_rx = None;

        let generation = shared.ranking_generation;
        assert!(shared.start_ranking_page(chart_of("m"), 2));
        assert!(shared.ranking_rx.is_some());
        assert_eq!(shared.ranking_requested.as_deref(), Some("m"));
        assert_eq!(shared.ranking_generation, generation.wrapping_add(1));
    }

    #[test]
    fn the_next_page_grows_the_cached_board_and_keeps_its_age() {
        let mut server = RankingDouble::new((0..RANKING_PAGE_LIMIT).map(|n| record(&format!("p{n}"), &format!("Player {n}"), 2000 - n, None)).collect());
        server.later_pages = vec![vec![record("q0", "Player q0", 100, None)]];
        let mut shared = shared_reading(server);
        let before = Instant::now();

        shared.start_ranking_fetch(chart_of("m"));
        take_the_answer_in_flight(&mut shared, before);
        let first = board_in(&shared, "m");
        assert_eq!((first.pages, first.more, first.rows.len()), (1, true, RANKING_PAGE_LIMIT as usize));
        assert!(shared.ranking_rx.is_none(), "nothing below the page is looked for when the player has no rank");

        assert!(shared.start_ranking_page(chart_of("m"), first.pages + 1));
        take_the_answer_in_flight(&mut shared, before + RANKING_READY_LIFETIME - Duration::from_millis(1));
        let grown = board_in(&shared, "m");
        assert_eq!((grown.pages, grown.more, grown.rows.len()), (2, false, RANKING_PAGE_LIMIT as usize + 1));
        assert_eq!(grown.rows.last().and_then(|row| row.rank), Some(RANKING_PAGE_LIMIT + 1));
        assert!(!shared.start_ranking_page(chart_of("m"), 3), "the server has no page after a short one");
        assert!(!shared.ranking_cache.has_settled_answer("m", before + RANKING_READY_LIFETIME), "growing did not renew the board");
    }

    #[test]
    fn a_player_below_the_first_page_gets_the_page_they_stand_on() {
        let mut server = RankingDouble::new((0..RANKING_PAGE_LIMIT).map(|n| record(&format!("p{n}"), &format!("Player {n}"), 2000 - n, None)).collect());
        server.later_pages = vec![Vec::new(), (0..RANKING_PAGE_LIMIT).map(|n| record(&format!("q{n}"), &format!("Player q{n}"), 900 - n, None)).collect()];
        server.bests.insert("me".into(), record("me", "Me", 500, Some(250)));
        let mut shared = shared_reading(server);
        let now = Instant::now();

        shared.start_ranking_fetch(chart_of("m"));
        take_the_answer_in_flight(&mut shared, now);
        assert!(shared.ranking_rx.is_some(), "the page of rank 250 is asked for right after the first page");

        take_the_answer_in_flight(&mut shared, now);
        let board = board_in(&shared, "m");
        assert_eq!((board.pages, board.rows.len()), (1, RANKING_PAGE_LIMIT as usize), "the rows still run from the top");
        let held = board.your_page.expect("the page of the player is held");
        assert_eq!((held.page, held.rows.len(), held.rows[0].rank), (3, RANKING_PAGE_LIMIT as usize, Some(2 * RANKING_PAGE_LIMIT + 1)));
        assert!(shared.ranking_rx.is_none(), "one extra page, not a walk down the leaderboard");
    }

    #[test]
    fn handing_a_run_to_the_server_notes_its_chart_for_the_ranking() {
        let mut shared = shared(Config::default());
        let submission = crate::ir_ext::tests::submission();

        shared.spawn_score_submit(submission.clone(), None, None);
        assert_eq!(shared.ranking_cache.awaited_score_chart(), None, "a guest run is not sent, so no score is awaited");

        shared.session = crate::ir_session::AccountSession::restored(Some("token".into()), Some("dj".into()));
        shared.spawn_score_submit(submission.clone(), None, None);
        assert_eq!(shared.ranking_cache.awaited_score_chart(), Some(submission.chart.id.md5.as_str()));
    }

    #[test]
    fn handing_a_run_to_the_extra_profiles_notes_its_chart_for_the_ranking() {
        let mut config = Config::default();
        config.network.ir_profiles.push(rbms_config::IrProfile {
            name: "EXTRA".to_string(),
            base_url: "http://extra.invalid".to_string(),
            token: None,
            enabled: true,
        });
        let mut shared = shared(config);
        shared.profile_servers = vec![Arc::new(RankingDouble::new(Vec::new()))];
        let submission = crate::ir_ext::tests::submission();

        shared.spawn_profile_submits(&submission);

        assert_eq!(shared.ranking_cache.awaited_score_chart(), Some(submission.chart.id.md5.as_str()));
    }

    #[test]
    fn a_score_the_server_took_makes_its_chart_be_asked_for_again_and_no_other() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("scored".into(), ready_board_of(1));
        shared.ranking_cache.insert("other".into(), ready_board_of(1));
        shared.ranking_cache.await_score_of("scored");
        shared.ranking_requested = Some("scored".into());

        submitted_through(&mut shared, accepted_outcome());

        assert!(shared.ranking_cache.peek("scored").is_none(), "the board does not show the new score");
        assert!(shared.ranking_cache.peek("other").is_some());
        assert!(shared.ranking_requested.is_none(), "the panel asks for the chart again");
    }

    #[test]
    fn a_score_the_server_took_drops_a_fetch_of_its_chart_that_may_predate_it() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("scored".into(), RankingState::Loading);
        shared.ranking_cache.await_score_of("scored");
        shared.ranking_requested = Some("scored".into());
        let (_tx, rx) = std::sync::mpsc::channel::<Result<RankingFetch, rbms_ir::IrError>>();
        shared.ranking_rx = Some(rx);
        let generation = shared.ranking_generation;

        submitted_through(&mut shared, accepted_outcome());

        assert!(shared.ranking_rx.is_none());
        assert_eq!(shared.ranking_generation, generation.wrapping_add(1));
        assert!(shared.ranking_cache.peek("scored").is_none());
    }

    #[test]
    fn a_score_of_another_chart_leaves_the_fetch_in_flight_alone() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("looked-at".into(), RankingState::Loading);
        shared.ranking_cache.await_score_of("scored");
        shared.ranking_requested = Some("looked-at".into());
        let (_tx, rx) = std::sync::mpsc::channel::<Result<RankingFetch, rbms_ir::IrError>>();
        shared.ranking_rx = Some(rx);
        let generation = shared.ranking_generation;

        submitted_through(&mut shared, accepted_outcome());

        assert!(shared.ranking_rx.is_some());
        assert_eq!(shared.ranking_generation, generation);
        assert_eq!(shared.ranking_requested.as_deref(), Some("looked-at"));
        assert!(shared.ranking_cache.peek("looked-at").is_some());
    }

    #[test]
    fn a_score_the_server_did_not_take_leaves_the_ranking_as_it_was() {
        for submit in [Ok(rbms_ir::SubmitResponse::default()), Err(rbms_ir::IrError::Network("down".into()))] {
            let mut shared = shared(Config::default());
            shared.ranking_cache.insert("scored".into(), ready_board_of(1));
            shared.ranking_cache.await_score_of("scored");
            shared.ranking_requested = Some("scored".into());

            submitted_through(&mut shared, rbms_ir::SubmitOutcome { submit, replay: None });

            assert!(shared.ranking_cache.peek("scored").is_some());
            assert_eq!(shared.ranking_requested.as_deref(), Some("scored"));
        }
    }

    #[test]
    fn a_score_an_extra_profile_took_makes_its_chart_be_asked_for_again() {
        let mut shared = shared(Config::default());
        shared.ranking_cache.insert("scored".into(), ready_board_of(1));
        shared.ranking_cache.await_score_of("scored");
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(vec![
            ("REFUSED".to_string(), Ok(rbms_ir::SubmitResponse::default())),
            ("TOOK IT".to_string(), Ok(rbms_ir::SubmitResponse { accepted: true, ..Default::default() })),
        ])
        .expect("profile worker receiver is open");
        shared.profile_submit_rx = Some(rx);
        shared.poll_profile_submits();
        assert!(shared.ranking_cache.peek("scored").is_none());

        shared.ranking_cache.insert("scored".into(), ready_board_of(1));
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(vec![("REFUSED".to_string(), Ok(rbms_ir::SubmitResponse::default()))]).expect("profile worker receiver is open");
        shared.profile_submit_rx = Some(rx);
        shared.poll_profile_submits();
        assert!(shared.ranking_cache.peek("scored").is_some(), "no profile took it");
    }
}
