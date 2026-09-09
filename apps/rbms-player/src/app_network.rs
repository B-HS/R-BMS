//! `App` wiring for the NETWORK settings tab: building the score server with the stored
//! credentials, the account actions (login / register / logout), settings sync, the rival list,
//! and the per-frame drain of every background reply.
//!
//! Nothing here blocks the frame loop: each action hands one blocking call to
//! [`rbms_ir::spawn_query`] and the frame loop polls the receiver.
#![allow(clippy::wildcard_imports)]
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use rbms_config::{OFF_VALUE, ON_VALUE, SettingId};
use rbms_ir::{IrError, NullScoreServer, PlayerId, ScoreServer, spawn_query};

use crate::ir_panel::*;
use crate::ir_session::{AuthAction, GUEST_PLAYER_ID, auth_request, run_auth};
use crate::ir_sync::{
    SETTINGS_BLOB_NAME, SyncOutcome, SyncPayload, build_blob, merge_downloaded, parse_blob, sanitise_for_upload, sync_error_message, upload_outcome,
};
use crate::*;

/// Gap between connection probes of the configured server.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// The score server plus the background probe that keeps the connection indicator fresh.
pub(crate) struct BuiltServer {
    pub(crate) server: Arc<dyn ScoreServer>,
    pub(crate) connected: Arc<AtomicBool>,
    /// Set to stop the probe thread when the server is replaced.
    pub(crate) probe_stop: Arc<AtomicBool>,
}

/// Build the score server from config (`HttpScoreServer` when a URL is set, else the offline
/// `NullScoreServer`), authenticated with `token` when one is stored, plus the probe thread that
/// keeps the connection flag fresh. Used at startup and after every credential or URL change.
pub(crate) fn build_server(config: &Config, token: Option<String>) -> BuiltServer {
    let server: Arc<dyn ScoreServer> = match &config.network.server_url {
        Some(url) => match rbms_ir::HttpScoreServer::try_new(url.clone(), token) {
            Ok(s) => {
                println!("score server: {url}");
                Arc::new(s)
            }
            Err(e) => {
                eprintln!("score server unavailable ({e}) — playing offline");
                Arc::new(NullScoreServer)
            }
        },
        None => Arc::new(NullScoreServer),
    };
    let connected = Arc::new(AtomicBool::new(false));
    let probe_stop = Arc::new(AtomicBool::new(false));
    if config.network.server_url.is_some() {
        let server = server.clone();
        let connected = connected.clone();
        let stop = probe_stop.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                connected.store(server.health().is_ok(), Ordering::Relaxed);
                std::thread::sleep(HEALTH_POLL_INTERVAL);
            }
        });
    }
    BuiltServer { server, connected, probe_stop }
}

/// Wall-clock milliseconds since the epoch, the stamp the sync blob is versioned by.
fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

impl AppShared {
    /// Swap in a server built from the current URL and the live session token, stopping the probe
    /// thread of the previous one.
    pub(crate) fn rebuild_server(&mut self) {
        self.server_probe_stop.store(true, Ordering::Relaxed);
        let built = build_server(&self.config, self.session.token().map(str::to_string));
        self.server = built.server;
        self.server_connected = built.connected;
        self.server_probe_stop = built.probe_stop;
    }

    /// Label and value of a NETWORK row, or `None` when the row belongs to another tab.
    pub(crate) fn network_setting_line(&self, id: SettingId) -> Option<(&'static str, String)> {
        let label = network_row_label(id)?;
        let on_off = |on: bool| {
            if on { ON_VALUE.to_string() } else { OFF_VALUE.to_string() }
        };
        let value = match id {
            SettingId::ServerUrl => optional_value(self.config.network.server_url.as_deref()),
            SettingId::PlayerId => self.config.network.player_id.clone(),
            SettingId::Account => self.session.status_text(),
            SettingId::Email => optional_value(self.config.network.ir_email.as_deref()),
            SettingId::Password => password_value(self.password.chars().count()),
            SettingId::SyncSettings => on_off(self.config.network.sync_settings),
            SettingId::AutoUploadReplay => on_off(self.config.network.auto_upload_replay),
            SettingId::Rivals => self.config.network.rivals.len().to_string(),
            _ => ACTION_VALUE.to_string(),
        };
        Some((label, value))
    }

    /// Whether a request needs a configured server, reporting why it cannot run when it does not.
    fn require_server(&mut self) -> bool {
        if self.config.network.server_url.is_none() {
            self.net_status = "set SERVER URL first".to_string();
            return false;
        }
        true
    }

    /// Whether a request needs a signed-in account, reporting why it cannot run when it does not.
    fn require_account(&mut self) -> bool {
        if !self.require_server() {
            return false;
        }
        if !self.session.is_logged_in() {
            self.net_status = "log in first".to_string();
            return false;
        }
        true
    }

    /// Start a login or register on a worker thread, consuming the held password.
    pub(crate) fn start_auth(&mut self, action: AuthAction) {
        if self.session.is_busy() || self.auth_rx.is_some() || !self.require_server() {
            return;
        }
        let login_id = self.config.network.player_id.trim().to_string();
        if login_id.is_empty() || login_id == GUEST_PLAYER_ID {
            self.net_status = "set PLAYER ID to your account id first".to_string();
            return;
        }
        if self.password.is_empty() {
            self.net_status = "set PASSWORD first".to_string();
            return;
        }
        let email = self.config.network.ir_email.clone().unwrap_or_default();
        if action == AuthAction::Register && email.trim().is_empty() {
            self.net_status = "set EMAIL first".to_string();
            return;
        }
        let request = auth_request(action, &login_id, &self.password, &email);
        self.password.clear();
        self.session.begin(action);
        self.net_status = action.pending_text().to_string();
        self.auth_rx = Some((action, spawn_query(self.server.clone(), move |server| run_auth(server, action, &request))));
    }

    /// Drop the session and rebuild the server without credentials.
    ///
    /// The player id goes back to [`GUEST_PLAYER_ID`] with it: a tokenless submission under the old
    /// account id is rejected 401 by the server, so leaving the id behind would silently drop every
    /// score played after a logout.
    pub(crate) fn finish_logout(&mut self) {
        let had_session = self.session.logout();
        self.password.clear();
        self.config.network.player_id = GUEST_PLAYER_ID.to_string();
        self.sync_lock.apply(SyncOutcome::SignedOut);
        if had_session {
            self.rebuild_server();
        }
        self.net_status = "logged out".to_string();
        self.save_settings();
    }

    /// Push the local settings + key config to the account, guarded by the optimistic lock from the
    /// last download.
    pub(crate) fn start_settings_upload(&mut self) {
        if self.sync_upload_rx.is_some() || self.sync_base_rx.is_some() || !self.require_account() {
            return;
        }
        let payload = SyncPayload { settings: sanitise_for_upload(&self.config), keyconfig: self.keyconfig.clone() };
        let updated_at = now_ms();
        let blob = match build_blob(&payload, updated_at, self.sync_lock.base()) {
            Ok(blob) => blob,
            Err(message) => {
                self.net_status = message;
                return;
            }
        };
        let player = PlayerId { id: self.config.network.player_id.clone() };
        self.net_status = "uploading settings...".to_string();
        self.sync_upload_rx = Some(spawn_query(self.server.clone(), move |server| server.put_settings(&player, &blob)));
    }

    /// Re-read the stored blob purely to learn the `updated_at` the server stamped on it.
    ///
    /// Only needed against a deployment that still answers `204 No Content`: it ignores the
    /// `updated_at` the client sent, so the new optimistic lock exists only server-side and without
    /// this read-back the second save of a session would always lose the lock. A current server
    /// reports the stored stamp in the `PUT` body and the lock moves without a second round trip.
    fn refresh_sync_base(&mut self) {
        if self.sync_base_rx.is_some() || !self.session.is_logged_in() || self.config.network.server_url.is_none() {
            return;
        }
        let player = PlayerId { id: self.config.network.player_id.clone() };
        self.sync_base_rx = Some(spawn_query(self.server.clone(), move |server| server.get_settings(&player, SETTINGS_BLOB_NAME)));
    }

    /// Fetch the account's settings blob and apply it to this machine.
    pub(crate) fn start_settings_download(&mut self) {
        if self.sync_download_rx.is_some() || !self.require_account() {
            return;
        }
        let player = PlayerId { id: self.config.network.player_id.clone() };
        self.net_status = "downloading settings...".to_string();
        self.sync_download_rx = Some(spawn_query(self.server.clone(), move |server| server.get_settings(&player, SETTINGS_BLOB_NAME)));
    }

    /// Apply a downloaded blob: play preferences and key config come from the server, this
    /// machine's session and paths stay put.
    fn apply_downloaded_settings(&mut self, blob: &rbms_ir::SettingsBlob) {
        let payload = match parse_blob(blob) {
            Ok(payload) => payload,
            Err(message) => {
                self.net_status = format!("settings blob unreadable: {message}");
                return;
            }
        };
        let mut merged = merge_downloaded(&self.config, payload.settings);
        merged.sanitise();
        self.config = merged;
        self.keyconfig = payload.keyconfig;
        self.keyconfig.save(&self.keyconfig_path);
        self.active_keys = self.keyconfig.lane_keys(self.mode);
        self.rebuild_skin();
        self.sync_lock.apply(SyncOutcome::Read(blob.updated_at));
        self.save_settings();
        self.net_status = "settings downloaded".to_string();
    }

    /// Send the current rival list to the server (and keep the local cache either way).
    pub(crate) fn push_rivals(&mut self) {
        self.save_settings();
        if self.rivals_rx.is_some() || !self.session.is_logged_in() || self.config.network.server_url.is_none() {
            return;
        }
        let player = PlayerId { id: self.config.network.player_id.clone() };
        let rivals = self.config.network.rivals.clone();
        self.net_status = "saving rivals...".to_string();
        self.rivals_rx = Some(spawn_query(self.server.clone(), move |server| server.put_rivals(&player, &rivals)));
    }

    /// Pull the rival list the server holds, replacing the local cache.
    pub(crate) fn refresh_rivals(&mut self) {
        if self.rivals_rx.is_some() || !self.session.is_logged_in() || self.config.network.server_url.is_none() {
            return;
        }
        let player = PlayerId { id: self.config.network.player_id.clone() };
        self.rivals_rx = Some(spawn_query(self.server.clone(), move |server| server.rivals(&player)));
    }

    /// Drain every background network reply. Called once per frame, before rendering.
    pub(crate) fn poll_network(&mut self) {
        if self.startup_whoami {
            self.startup_whoami = false;
            if self.session.is_logged_in() && self.config.network.server_url.is_some() {
                self.whoami_rx = Some(spawn_query(self.server.clone(), |server| server.whoami().map(|profile| profile.id)));
                self.refresh_rivals();
            }
        }
        self.poll_auth();
        self.poll_whoami();
        self.poll_settings_sync();
        self.poll_rivals();
    }

    fn poll_auth(&mut self) {
        let Some((action, rx)) = self.auth_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                let succeeded = result.is_ok();
                let token_changed = self.session.finish(action, result);
                if succeeded {
                    if let Some(id) = self.session.login_id() {
                        self.config.network.player_id = id.to_string();
                    }
                    if token_changed {
                        self.rebuild_server();
                    }
                    self.refresh_rivals();
                }
                self.net_status = self.session.status_text();
                self.save_settings();
            }
            Err(TryRecvError::Empty) => self.auth_rx = Some((action, rx)),
            Err(TryRecvError::Disconnected) => {
                self.session.finish(action, Err(IrError::Network("auth worker stopped".into())));
                self.net_status = self.session.status_text();
            }
        }
    }

    fn poll_whoami(&mut self) {
        let Some(rx) = self.whoami_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                let dropped = self.session.finish_whoami(result);
                if dropped {
                    self.rebuild_server();
                    self.save_settings();
                }
                self.net_status = self.session.status_text();
            }
            Err(TryRecvError::Empty) => self.whoami_rx = Some(rx),
            Err(TryRecvError::Disconnected) => {}
        }
    }

    fn poll_settings_sync(&mut self) {
        if let Some(rx) = self.sync_base_rx.take() {
            match rx.try_recv() {
                Ok(Ok(blob)) => self.sync_lock.apply(SyncOutcome::Read(blob.updated_at)),
                Ok(Err(_)) => self.net_status = "settings uploaded (lock not refreshed; download before the next save)".to_string(),
                Err(TryRecvError::Empty) => self.sync_base_rx = Some(rx),
                Err(TryRecvError::Disconnected) => {}
            }
        }
        if let Some(rx) = self.sync_upload_rx.take() {
            match rx.try_recv() {
                Ok(Ok(stored)) => {
                    self.net_status = "settings uploaded".to_string();
                    match upload_outcome(stored) {
                        Some(outcome) => self.sync_lock.apply(outcome),
                        None => self.refresh_sync_base(),
                    }
                }
                Ok(Err(error)) => {
                    self.sync_lock.apply(SyncOutcome::Conflict);
                    self.net_status = match crate::ir_sync::conflict_server_copy(&error) {
                        Some(server_copy) => format!("{} (server copy {})", sync_error_message(&error), server_copy.updated_at),
                        None => sync_error_message(&error),
                    };
                }
                Err(TryRecvError::Empty) => self.sync_upload_rx = Some(rx),
                Err(TryRecvError::Disconnected) => self.net_status = "settings upload worker stopped".to_string(),
            }
        }
        if let Some(rx) = self.sync_download_rx.take() {
            match rx.try_recv() {
                Ok(Ok(blob)) => self.apply_downloaded_settings(&blob),
                Ok(Err(error)) => self.net_status = sync_error_message(&error),
                Err(TryRecvError::Empty) => self.sync_download_rx = Some(rx),
                Err(TryRecvError::Disconnected) => self.net_status = "settings download worker stopped".to_string(),
            }
        }
    }

    fn poll_rivals(&mut self) {
        let Some(rx) = self.rivals_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(profiles)) => {
                self.config.network.rivals = profiles.into_iter().map(|profile| profile.id).collect();
                self.net_status = format!("rivals: {}", self.config.network.rivals.len());
                self.save_settings();
            }
            Ok(Err(error)) => self.net_status = format!("rivals: {}", crate::ir_outcome::short_error(&error)),
            Err(TryRecvError::Empty) => self.rivals_rx = Some(rx),
            Err(TryRecvError::Disconnected) => {}
        }
    }
}
