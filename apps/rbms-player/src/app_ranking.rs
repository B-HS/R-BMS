//! `App` wiring for the song-select IR ranking panel, the replay it can start, and the score
//! submission the result screen reports.
//!
//! The panel fetches on a worker thread once the select focus has settled (the same debounce the
//! heavy chart detail uses), caches per chart, and drops results whose generation has moved on.
#![allow(clippy::wildcard_imports)]
use std::sync::mpsc::TryRecvError;

use rbms_ir::{ChartId, PlayerId, ReplayData, ScoreSubmission, SubmitJob, spawn_query, spawn_submit};
use winit::keyboard::KeyCode;

use crate::ir_outcome::{IrStatus, format_submit_outcome, short_error};
use crate::ir_ranking::{RankingFetch, RankingState, accept_fetch, fetch_board};
use crate::ir_ranking_view::{PanelAction, PanelLine, offline_lines, panel_action, panel_lines};
use crate::ir_replay::{from_ir_replay, replay_download_is_applicable, to_ir_replay};
use crate::*;

impl App {
    /// Chart id of the focused row, for the IR (only the md5 addresses a chart on the wire).
    fn focused_chart_id(&self) -> Option<ChartId> {
        self.focused_md5().map(|md5| ChartId { md5, sha256: String::new() })
    }

    /// Show or hide the ranking panel. While it is open it also takes the arrow keys, so one key
    /// both opens it and gives it focus.
    pub(crate) fn toggle_ranking_panel(&mut self) {
        self.ranking_open = !self.ranking_open;
        self.ranking_sel = 0;
        if self.ranking_open {
            self.ranking_requested = None;
        }
    }

    /// The lines the panel is showing for the focused chart.
    pub(crate) fn ranking_lines(&self) -> Vec<PanelLine> {
        if self.config.server_url.is_none() {
            return offline_lines();
        }
        match self.focused_md5() {
            Some(md5) => panel_lines(self.ranking_cache.peek(&md5)),
            None => panel_lines(None),
        }
    }

    /// Keys while the ranking panel is open. Returns whether the panel consumed the key.
    pub(crate) fn ranking_panel_input(&mut self, code: KeyCode) -> bool {
        if !self.ranking_open {
            return false;
        }
        let Some(action) = panel_action(code) else {
            return false;
        };
        let lines = self.ranking_lines();
        match action {
            PanelAction::Close => self.toggle_ranking_panel(),
            PanelAction::Up => self.ranking_sel = self.ranking_sel.saturating_sub(1),
            PanelAction::Down => self.ranking_sel = (self.ranking_sel + 1).min(lines.len().saturating_sub(1)),
            PanelAction::PlayReplay => self.play_ranking_replay(),
        }
        true
    }

    /// Click on a panel row: focus it, or start its replay when it is already focused.
    pub(crate) fn ranking_click(&mut self, index: usize) {
        let lines = self.ranking_lines();
        if lines.is_empty() {
            return;
        }
        let index = index.min(lines.len() - 1);
        if self.ranking_sel == index {
            self.play_ranking_replay();
        } else {
            self.ranking_sel = index;
        }
    }

    /// Download and play the replay behind the focused panel row.
    fn play_ranking_replay(&mut self) {
        let Some(line) = self.ranking_lines().into_iter().nth(self.ranking_sel) else {
            return;
        };
        let Some(replay_id) = line.replay_id else {
            self.net_status = "no replay for that row".to_string();
            return;
        };
        let Some(index) = self.focused_song_index() else {
            return;
        };
        let Some(entry) = self.songs.get(index) else {
            return;
        };
        if self.replay_download_rx.is_some() {
            return;
        }
        self.replay_download_target = Some((entry.path.to_string_lossy().to_string(), entry.md5.clone()));
        self.net_status = format!("downloading replay {replay_id}...");
        self.replay_download_rx = Some(spawn_query(self.server.clone(), move |server| server.download_replay(&replay_id)));
    }

    /// Start a ranking fetch when the focus has settled on a new chart and the panel has no cached
    /// answer for it. Called once per frame while the select screen is up.
    ///
    /// Only one fetch is ever in flight: a second one would drop the first receiver, stranding that
    /// chart's `Loading` placeholder in the cache and leaving the panel on LOADING forever. While a
    /// fetch runs the request for the newly focused chart is simply retried next frame.
    pub(crate) fn update_ranking(&mut self) {
        if !self.ranking_open || self.config.server_url.is_none() || self.ranking_rx.is_some() {
            return;
        }
        let Some(chart) = self.focused_chart_id() else {
            return;
        };
        if self.focus_settle_at.elapsed() < FOCUS_DETAIL_DEBOUNCE {
            return;
        }
        if self.ranking_requested.as_deref() == Some(chart.md5.as_str()) {
            return;
        }
        self.ranking_requested = Some(chart.md5.clone());
        self.ranking_sel = 0;
        self.ranking_generation = self.ranking_generation.wrapping_add(1);
        if self.ranking_cache.has_settled_answer(&chart.md5) {
            return;
        }
        let generation = self.ranking_generation;
        let md5 = chart.md5.clone();
        let player = PlayerId { id: self.config.player_id.clone() };
        let rivals = self.config.rivals.clone();
        self.ranking_cache.insert(md5.clone(), RankingState::Loading);
        let rx = spawn_query(self.server.clone(), move |server| {
            let state = fetch_board(server, &chart, &player, &rivals);
            Ok::<RankingFetch, rbms_ir::IrError>(RankingFetch { generation, md5, state })
        });
        self.ranking_rx = Some(rx);
    }

    /// Hand a finished score submission to the IR worker, uploading the run's replay alongside it
    /// when AUTO UPLOAD REPLAY is on and the account can store one.
    pub(crate) fn spawn_score_submit(&mut self, submission: ScoreSubmission, replay: Option<ReplayData>) {
        self.ir_status = IrStatus::Sending;
        self.submit_rx = Some(spawn_submit(self.server.clone(), SubmitJob { submission, replay }));
    }

    /// The replay payload to upload with this run, or `None` when the setting is off, no replay was
    /// recorded, or the client is a guest (the server rejects anonymous replay uploads).
    pub(crate) fn replay_upload_payload(&self, replay: &Replay, chart: &ChartId) -> Option<ReplayData> {
        if !self.config.auto_upload_replay || !self.session.is_logged_in() || replay.events.is_empty() {
            return None;
        }
        Some(to_ir_replay(replay, chart, self.config.judge_rate, self.config.constant_speed, self.chart_lntype, self.build_sha256.clone()))
    }

    /// Drain the ranking, replay-download and submission workers. Called once per frame.
    pub(crate) fn poll_ir_jobs(&mut self) {
        self.poll_ranking();
        self.poll_replay_download();
        self.poll_submit();
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

    fn poll_replay_download(&mut self) {
        let Some(rx) = self.replay_download_rx.take() else {
            return;
        };
        if !replay_download_is_applicable(&self.stage) {
            self.replay_download_target = None;
            self.net_status = "replay download dropped: left song select".to_string();
            return;
        }
        match rx.try_recv() {
            Ok(Ok(data)) => {
                let Some((chart_path, md5)) = self.replay_download_target.take() else {
                    return;
                };
                let replay = from_ir_replay(&data, &chart_path, &md5);
                if replay.events.is_empty() {
                    self.net_status = "downloaded replay has no inputs".to_string();
                    return;
                }
                self.net_status = "replay downloaded".to_string();
                self.start_downloaded_replay(chart_path, replay);
            }
            Ok(Err(error)) => {
                self.replay_download_target = None;
                self.net_status = format!("replay download: {}", short_error(&error));
            }
            Err(TryRecvError::Empty) => self.replay_download_rx = Some(rx),
            Err(TryRecvError::Disconnected) => self.replay_download_target = None,
        }
    }

    /// Enter replay playback for a downloaded run, the same path a local record replay takes.
    fn start_downloaded_replay(&mut self, chart_path: String, replay: Replay) {
        self.ranking_open = false;
        self.record_modal = None;
        self.chart_path = chart_path;
        self.replay = Some(replay);
        self.replay_cursor = 0;
        self.result = None;
        self.loading_drawn = false;
        self.pending = None;
        if self.load() {
            self.after_load();
        } else {
            self.stage = Stage::Select;
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
