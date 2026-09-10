//! `AppShared` wiring for the IR ranking fetch, the replay download it can start, and the score
//! submission the result screen reports.
//!
//! The panel itself lives in `stage::select`; what is here is the part that keeps running while
//! other screens are up — the worker drains and the caches they feed.
#![allow(clippy::wildcard_imports)]
use std::sync::mpsc::TryRecvError;

use rbms_ir::{ChartId, PlayerId, ReplayData, ScoreSubmission, SubmitJob, spawn_query, spawn_submit};

use crate::ir_outcome::{IrStatus, format_submit_outcome, short_error};
use crate::ir_ranking::{RankingFetch, RankingState, accept_fetch, fetch_board};
use crate::ir_replay::{replay_download_is_applicable, to_ir_replay};
use crate::stage::StageId;
use crate::*;

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

    /// The server a finished course is submitted to. Courses go to the primary profile only.
    pub(crate) fn course_ir_server(&self) -> Arc<dyn ScoreServer> {
        self.primary_ir_server()
    }

    /// Hand a finished score submission to the IR worker, uploading the run's replay alongside it
    /// when AUTO UPLOAD REPLAY is on and the account can store one.
    pub(crate) fn spawn_score_submit(&mut self, submission: ScoreSubmission, replay: Option<ReplayData>) {
        self.ir_status = IrStatus::Sending;
        self.submit_rx = Some(spawn_submit(self.server.clone(), SubmitJob { submission, replay }));
    }

    /// Submit the same run to every extra IR profile, in parallel and off the frame thread.
    ///
    /// `server_url` itself is left to [`AppShared::spawn_score_submit`] — it is the profile that
    /// also uploads the replay — so this fans out over [`MultiIr::secondary`] and never submits
    /// there twice. One profile failing does not touch the others' results.
    pub(crate) fn spawn_profile_submits(&mut self, submission: &ScoreSubmission) {
        let secondary = self.multi_ir.secondary();
        if secondary.enabled_profiles().next().is_none() {
            return;
        }
        self.profile_submit_rx = Some(spawn_submit_all(secondary, self.profile_servers.clone(), submission.clone()));
    }

    /// The replay payload to upload with this run, or `None` when the setting is off, no replay was
    /// recorded, or the client is a guest (the server rejects anonymous replay uploads).
    pub(crate) fn replay_upload_payload(&self, replay: &Replay, chart: &ChartId, lntype: i32) -> Option<ReplayData> {
        if !self.config.network.auto_upload_replay || !self.session.is_logged_in() || replay.events.is_empty() {
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
            Ok(results) => self.net_status = submit_summary(&results),
            Err(TryRecvError::Empty) => self.profile_submit_rx = Some(rx),
            Err(TryRecvError::Disconnected) => self.net_status = "profile submit worker stopped".to_string(),
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
        if self.ir_status.accepts_report() {
            self.ir_status = IrStatus::Reported(format_submit_outcome(&outcome));
        }
    }
}
