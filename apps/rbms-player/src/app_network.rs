//! `App` wiring for the NETWORK settings tab: building the score server with the stored
//! credentials, the account actions (login / register / logout), settings sync, the rival list,
//! and the per-frame drain of every background reply.
//!
//! Nothing here blocks the frame loop: each action hands one blocking call to
//! [`rbms_ir::spawn_query`] and the frame loop polls the receiver.
#![allow(clippy::wildcard_imports)]
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SendError, TryRecvError};
use std::time::Duration;

use rbms_config::{OFF_VALUE, ON_VALUE, SettingId};
use rbms_ir::{AuthRequest, AuthResponse, IrError, NullScoreServer, PlayerId, ScoreServer, spawn_query};

use crate::ir_panel::*;
use crate::ir_session::{AuthAction, GUEST_PLAYER_ID, auth_request, run_auth, run_revoke};
use crate::ir_sync::{
    SETTINGS_BLOB_NAME, SyncOutcome, SyncPayload, build_blob, merge_downloaded, parse_blob, sanitise_for_upload, sync_error_message, upload_outcome,
};
use crate::record_sync::ACCOUNT_NOT_CONFIRMED;
use crate::*;

/// What the status line says when a change of account or server let go of a request in flight.
const ACCOUNT_REQUESTS_DROPPED: &str = "account or server changed: the request in flight was dropped";

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
                notify(Level::Warn, format!("score server unavailable ({e}) — playing offline"));
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

/// What the message bus says when a logout could not tell the server to refuse the token. The token
/// is gone from this machine either way; it stays valid on the server until it is deleted from the
/// account page of the web site.
const TOKEN_NOT_REVOKED: &str = "logged out, but the server did not revoke the token (delete it on the web site)";

/// Tell the server to refuse the token `server` carries, on a thread of its own: a logout finishes
/// on this machine whether or not the server can be reached, and nothing waits on the answer.
fn revoke_token_in_background(server: Arc<dyn ScoreServer>) {
    std::thread::spawn(move || {
        if let Err(error) = run_revoke(server.as_ref()) {
            notify(Level::Warn, format!("{TOKEN_NOT_REVOKED}: {}", crate::ir_outcome::short_error(&error)));
        }
    });
}

/// Run one login or register on a thread of its own and hand its answer to the receiver.
///
/// When nobody takes the answer any more -- the request was let go of because the account or the
/// server changed while it ran -- a token the server issued for it is known to nobody on this
/// machine, so it is handed to `revoke_unclaimed` rather than left valid on the server.
fn spawn_auth(
    server: Arc<dyn ScoreServer>,
    action: AuthAction,
    request: AuthRequest,
    revoke_unclaimed: impl FnOnce(String) + Send + 'static,
) -> Receiver<Result<AuthResponse, IrError>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Err(SendError(Ok(response))) = tx.send(run_auth(server.as_ref(), action, &request)) {
            revoke_unclaimed(response.token);
        }
    });
    rx
}

/// Tell the server at `server_url` to refuse `token`, a token it issued for a login or register this
/// machine let go of. Runs on the auth worker's own thread.
fn revoke_unclaimed_token(server_url: Option<String>, token: String) {
    let Some(url) = server_url else {
        return;
    };
    match rbms_ir::HttpScoreServer::try_new(url, Some(token)) {
        Ok(server) => {
            if let Err(error) = run_revoke(&server) {
                notify(Level::Warn, format!("{UNCLAIMED_TOKEN_NOT_REVOKED}: {}", crate::ir_outcome::short_error(&error)));
            }
        }
        Err(error) => notify(Level::Warn, format!("{UNCLAIMED_TOKEN_NOT_REVOKED}: {error}")),
    }
}

/// What the message bus says when a token issued for a login or register this machine let go of
/// could not be revoked.
const UNCLAIMED_TOKEN_NOT_REVOKED: &str = "a login that was let go of left its token valid on the server (delete it on the web site)";

/// Wall-clock milliseconds since the epoch, the stamp the sync blob is versioned by.
fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

impl AppShared {
    /// Swap in a server built from the current URL and the live session token, stopping the probe
    /// thread of the previous one. The song list shows the folders of whoever that server now knows
    /// the player as ([`crate::user_folders::follow_account`]).
    ///
    /// A records sync in flight is stopped, since what it sends would go to the server and account
    /// it started with, and the states the record screens read are read again for the server now set.
    ///
    /// Every request in flight that was made as an account is let go of: its answer is for the
    /// server and the token it was made with, not for these.
    pub(crate) fn rebuild_server(&mut self) {
        self.cancel_record_sync();
        self.drop_account_requests();
        self.server_probe_stop.store(true, Ordering::Relaxed);
        let built = build_server(&self.config, self.session.token().map(str::to_string));
        self.server = built.server;
        self.server_connected = built.connected;
        self.server_probe_stop = built.probe_stop;
        self.multi_ir = MultiIr::from_network(&self.config.network);
        self.profile_servers = self.multi_ir.build_servers_reusing(Some(self.server.clone()));
        self.reset_ranking();
        self.reload_record_sync_states();
        crate::user_folders::follow_account(self);
    }

    /// Let go of every request in flight that was made as an account -- the identity probe, the
    /// settings sync calls and the rival list -- so an answer that comes late is never applied to
    /// the account that is signed in by then. The worker threads run to their end and their answers
    /// are dropped with the receivers.
    ///
    /// A login or register in flight is let go of too: its answer would sign the player in to an
    /// account they have just left, or to the server they have just left, and save its token. The
    /// session goes back to what it was before the request, and a token the server issued for it is
    /// revoked by the worker that receives it ([`spawn_auth`]).
    fn drop_account_requests(&mut self) {
        let in_flight =
            [self.sync_upload_rx.is_some(), self.sync_base_rx.is_some(), self.sync_download_rx.is_some(), self.rivals_rx.is_some(), self.auth_rx.is_some()];
        self.auth_rx = None;
        self.session.abandon();
        self.whoami_rx = None;
        self.rivals_rx = None;
        self.sync_upload_rx = None;
        self.sync_base_rx = None;
        self.sync_download_rx = None;
        if in_flight.contains(&true) {
            self.net_status = ACCOUNT_REQUESTS_DROPPED.to_string();
        }
    }

    /// Forget what this machine holds of the account that was signed in: its rival list, the lock
    /// its settings were last synced under, and every request in flight as it. Called wherever the
    /// account behind the session changes, so the next one starts from nothing of the last one's.
    fn forget_account_data(&mut self) {
        self.config.network.rivals.clear();
        self.sync_lock.apply(SyncOutcome::SignedOut);
        self.drop_account_requests();
    }

    /// The account requests are made as: the one the token in hand belongs to ([`AccountSession::signed_in_as`]),
    /// never the PLAYER ID typed into the settings, which the server refuses with 403 when the two differ.
    fn account_player(&self) -> Option<PlayerId> {
        self.session.signed_in_as().map(|id| PlayerId { id: id.to_string() })
    }

    /// Make the PLAYER ID row name the account that is signed in, which is what the next LOGIN is
    /// made as. Returns whether the row changed.
    fn follow_session_player_id(&mut self) -> bool {
        let Some(id) = self.session.signed_in_as() else {
            return false;
        };
        if self.config.network.player_id == id {
            return false;
        }
        self.config.network.player_id = id.to_string();
        true
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
            SettingId::IrProfiles => self.config.network.ir_profiles.len().to_string(),
            SettingId::SyncRecords => self.sync_records_value().unwrap_or_else(|| ACTION_VALUE.to_string()),
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

    /// The account a request needs to be made as, reporting why there is none when there is none.
    fn require_account(&mut self) -> Option<PlayerId> {
        if !self.require_server() {
            return None;
        }
        if !self.session.is_logged_in() {
            self.net_status = "log in first".to_string();
            return None;
        }
        let player = self.account_player();
        if player.is_none() {
            self.net_status = ACCOUNT_NOT_CONFIRMED.to_string();
        }
        player
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
        let server_url = self.config.network.server_url.clone();
        self.auth_rx = Some((action, spawn_auth(self.server.clone(), action, request, move |token| revoke_unclaimed_token(server_url, token))));
    }

    /// Drop the session and rebuild the server without credentials.
    ///
    /// The player id goes back to [`GUEST_PLAYER_ID`] with it: a tokenless submission under the old
    /// account id is rejected 401 by the server, so leaving the id behind would silently drop every
    /// score played after a logout. The account's own folders leave the song list, and the copy of
    /// them is deleted: nothing of an account that signed out is kept on the machine.
    ///
    /// A records sync in flight stops with the session: nothing more is sent for an account that
    /// signed out. The token is revoked on the server on a thread of its own, and the rival list and
    /// the settings lock of the account are forgotten with it.
    pub(crate) fn finish_logout(&mut self) {
        self.cancel_record_sync();
        let folders_of = crate::user_folders::FolderAccount::of(self);
        let had_session = self.session.logout();
        if had_session && self.config.network.server_url.is_some() {
            revoke_token_in_background(self.server.clone());
        }
        self.password.clear();
        self.config.network.player_id = GUEST_PLAYER_ID.to_string();
        self.forget_account_data();
        if had_session {
            self.rebuild_server();
        }
        if let Some(account) = folders_of {
            crate::user_folders::forget_account(self, &account);
        }
        self.net_status = "logged out".to_string();
        self.save_settings();
    }

    /// Push the local settings + key config to the account, guarded by the optimistic lock from the
    /// last download.
    pub(crate) fn start_settings_upload(&mut self) {
        if self.sync_upload_rx.is_some() || self.sync_base_rx.is_some() {
            return;
        }
        let Some(player) = self.require_account() else {
            return;
        };
        let payload = SyncPayload { settings: sanitise_for_upload(&self.config), keyconfig: self.keyconfig.clone() };
        let updated_at = now_ms();
        let blob = match build_blob(&payload, updated_at, self.sync_lock.base()) {
            Ok(blob) => blob,
            Err(message) => {
                self.net_status = message;
                return;
            }
        };
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
        if self.sync_base_rx.is_some() || self.config.network.server_url.is_none() {
            return;
        }
        let Some(player) = self.account_player() else {
            return;
        };
        self.sync_base_rx = Some(spawn_query(self.server.clone(), move |server| server.get_settings(&player, SETTINGS_BLOB_NAME)));
    }

    /// Fetch the account's settings blob and apply it to this machine.
    pub(crate) fn start_settings_download(&mut self) {
        if self.sync_download_rx.is_some() {
            return;
        }
        let Some(player) = self.require_account() else {
            return;
        };
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
        self.active_reverse_keys = self.keyconfig.scratch_reverse_keys(self.mode);
        self.rebuild_skin();
        self.sync_lock.apply(SyncOutcome::Read(blob.updated_at));
        self.save_settings();
        self.net_status = "settings downloaded".to_string();
    }

    /// Send the current rival list to the server (and keep the local cache either way).
    pub(crate) fn push_rivals(&mut self) {
        self.save_settings();
        if self.rivals_rx.is_some() || self.config.network.server_url.is_none() {
            return;
        }
        let Some(player) = self.account_player() else {
            return;
        };
        let rivals = self.config.network.rivals.clone();
        self.net_status = "saving rivals...".to_string();
        self.rivals_rx = Some(spawn_query(self.server.clone(), move |server| server.put_rivals(&player, &rivals)));
    }

    /// Pull the rival list the server holds, replacing the local cache.
    pub(crate) fn refresh_rivals(&mut self) {
        if self.rivals_rx.is_some() || self.config.network.server_url.is_none() {
            return;
        }
        let Some(player) = self.account_player() else {
            return;
        };
        self.rivals_rx = Some(spawn_query(self.server.clone(), move |server| server.rivals(&player)));
    }

    /// Drain every background network reply. Called once per frame, before rendering.
    ///
    /// The first frame of a run signed in to a server confirms the session, and fetches the rivals
    /// and the player's own folders, without saying anything of the folders unless they changed: a
    /// server that does not answer is no news at the start of a run.
    pub(crate) fn poll_network(&mut self) {
        if self.startup_whoami {
            self.startup_whoami = false;
            if self.session.is_logged_in() && self.config.network.server_url.is_some() {
                self.whoami_rx = Some(spawn_query(self.server.clone(), |server| server.whoami().map(|profile| profile.id)));
                self.refresh_rivals();
                crate::user_folders::start_fetch(self, false);
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
                let account_before = self.session.login_id().map(str::to_string);
                let token_changed = self.session.finish(action, result);
                if succeeded {
                    if account_before.is_some() && self.session.login_id() != account_before.as_deref() {
                        self.forget_account_data();
                    }
                    self.follow_session_player_id();
                    if token_changed {
                        self.rebuild_server();
                    }
                    self.refresh_rivals();
                    crate::user_folders::start_fetch(self, true);
                }
                self.net_status = self.session.status_text();
                self.save_settings();
                if succeeded {
                    self.start_record_sync(crate::record_sync::SyncScope::All);
                }
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
                let confirmed = result.is_ok();
                let account_before = self.session.login_id().map(str::to_string);
                let dropped = self.session.finish_whoami(result);
                let account_named = self.session.login_id() != account_before.as_deref();
                let other_account = account_named && account_before.is_some();
                if other_account {
                    self.forget_account_data();
                }
                let player_id_followed = self.follow_session_player_id();
                if dropped || other_account {
                    self.rebuild_server();
                }
                if account_named {
                    self.refresh_rivals();
                    crate::user_folders::start_fetch(self, false);
                }
                if dropped || account_named || player_id_followed {
                    self.save_settings();
                }
                self.net_status = self.session.status_text();
                if confirmed {
                    self.start_record_sync(crate::record_sync::SyncScope::All);
                }
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

#[cfg(test)]
mod tests;
