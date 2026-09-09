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

use rbms_ir::{IrError, NullScoreServer, PlayerId, ScoreServer, spawn_query};
use winit::keyboard::KeyCode;

use crate::ir_panel::*;
use crate::ir_session::{AuthAction, GUEST_PLAYER_ID, auth_request, run_auth};
use crate::ir_sync::{SETTINGS_BLOB_NAME, SyncOutcome, SyncPayload, build_blob, merge_downloaded, parse_blob, sanitise_for_upload, sync_error_message};
use crate::settings_view::{RivalsScene, SettingsScene};
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
pub(crate) fn build_server(config: &PlayerConfig, token: Option<String>) -> BuiltServer {
    let server: Arc<dyn ScoreServer> = match &config.server_url {
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
    if config.server_url.is_some() {
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

impl App {
    /// Swap in a server built from the current URL and the live session token, stopping the probe
    /// thread of the previous one.
    pub(crate) fn rebuild_server(&mut self) {
        self.server_probe_stop.store(true, Ordering::Relaxed);
        let built = build_server(&self.config, self.session.token().map(str::to_string));
        self.server = built.server;
        self.server_connected = built.connected;
        self.server_probe_stop = built.probe_stop;
    }

    /// Label and value of a NETWORK row, or `None` when the index belongs to another tab.
    pub(crate) fn network_setting_line(&self, index: usize) -> Option<(&'static str, String)> {
        let label = network_row_label(index)?;
        let on_off = |on: bool| {
            if on { "ON".to_string() } else { "OFF".to_string() }
        };
        let value = match index {
            SETTING_SERVER_URL => optional_value(self.config.server_url.as_deref()),
            SETTING_PLAYER_ID => self.config.player_id.clone(),
            SETTING_ACCOUNT => self.session.status_text(),
            SETTING_EMAIL => optional_value(self.config.ir_email.as_deref()),
            SETTING_PASSWORD => password_value(self.password.chars().count()),
            SETTING_SYNC_SETTINGS => on_off(self.config.sync_settings),
            SETTING_AUTO_UPLOAD_REPLAY => on_off(self.config.auto_upload_replay),
            SETTING_RIVALS => self.config.rivals.len().to_string(),
            _ => ACTION_VALUE.to_string(),
        };
        Some((label, value))
    }

    /// Left/right on a NETWORK row. Only the two toggles respond; everything else is left to
    /// [`App::network_setting_enter`].
    pub(crate) fn network_setting_adjust(&mut self, index: usize) -> bool {
        match index {
            SETTING_SYNC_SETTINGS => self.config.sync_settings = !self.config.sync_settings,
            SETTING_AUTO_UPLOAD_REPLAY => self.config.auto_upload_replay = !self.config.auto_upload_replay,
            _ => return false,
        }
        self.save_settings();
        true
    }

    /// Enter (or a click) on a NETWORK row. Returns whether the row handled it.
    pub(crate) fn network_setting_enter(&mut self, index: usize) -> bool {
        if is_text_row(index) {
            self.begin_network_edit(index);
            return true;
        }
        match index {
            SETTING_LOGIN => self.start_auth(AuthAction::Login),
            SETTING_REGISTER => self.start_auth(AuthAction::Register),
            SETTING_LOGOUT => self.finish_logout(),
            SETTING_UPLOAD_SETTINGS => self.start_settings_upload(),
            SETTING_DOWNLOAD_SETTINGS => self.start_settings_download(),
            SETTING_RIVALS => {
                self.rivals_open = true;
                self.rivals_sel = 0;
                self.cancel_text_edit();
            }
            SETTING_ACCOUNT => self.net_status = self.session.status_text(),
            _ => return self.network_setting_adjust(index),
        }
        true
    }

    /// Open the in-place editor for a NETWORK text row, pre-filled with the current value. The
    /// password row starts empty and never shows what is already held.
    ///
    /// The row is remembered in `text_edit_row` so the commit writes the field the editor was
    /// opened on even if the selection has since moved (a mouse click can move it while an editor
    /// is open); committing a typed password into another row's field would persist it in clear.
    pub(crate) fn begin_network_edit(&mut self, index: usize) {
        self.text_secret = is_secret_row(index);
        self.text_edit_row = Some(index);
        self.text_input = Some(match index {
            SETTING_PLAYER_ID => self.config.player_id.clone(),
            SETTING_SERVER_URL => self.config.server_url.clone().unwrap_or_default(),
            SETTING_EMAIL => self.config.ir_email.clone().unwrap_or_default(),
            _ => String::new(),
        });
    }

    /// Close any open in-place editor, dropping what was typed. Called whenever the focus leaves
    /// the row the editor belongs to, so a half-typed secret never outlives its row.
    pub(crate) fn cancel_text_edit(&mut self) {
        self.text_input = None;
        self.text_secret = false;
        self.text_edit_row = None;
    }

    /// Commit an edited NETWORK text row. A blank SERVER URL means offline, a blank PLAYER ID means
    /// guest, and the password is held in memory only until the next LOGIN/REGISTER consumes it.
    /// An index that is not a text row writes nothing — there is no catch-all field.
    pub(crate) fn commit_network_edit(&mut self, index: usize, value: String) {
        let Some(field) = network_text_field(index) else {
            return;
        };
        match field {
            NetworkTextField::ServerUrl => {
                self.config.server_url = (!value.is_empty()).then_some(value);
                self.rebuild_server();
            }
            NetworkTextField::PlayerId => self.config.player_id = if value.is_empty() { GUEST_PLAYER_ID.to_string() } else { value },
            NetworkTextField::Email => self.config.ir_email = (!value.is_empty()).then_some(value),
            NetworkTextField::Password => {
                self.password = value;
                return;
            }
        }
        self.save_settings();
    }

    /// Whether a request needs a configured server, reporting why it cannot run when it does not.
    fn require_server(&mut self) -> bool {
        if self.config.server_url.is_none() {
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
        let login_id = self.config.player_id.trim().to_string();
        if login_id.is_empty() || login_id == GUEST_PLAYER_ID {
            self.net_status = "set PLAYER ID to your account id first".to_string();
            return;
        }
        if self.password.is_empty() {
            self.net_status = "set PASSWORD first".to_string();
            return;
        }
        let email = self.config.ir_email.clone().unwrap_or_default();
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
        self.config.player_id = GUEST_PLAYER_ID.to_string();
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
        let payload = SyncPayload { settings: sanitise_for_upload(&self.current_settings()), keyconfig: self.keyconfig.clone() };
        let updated_at = now_ms();
        let blob = match build_blob(&payload, updated_at, self.sync_lock.base()) {
            Ok(blob) => blob,
            Err(message) => {
                self.net_status = message;
                return;
            }
        };
        let player = PlayerId { id: self.config.player_id.clone() };
        self.net_status = "uploading settings...".to_string();
        self.sync_upload_rx = Some(spawn_query(self.server.clone(), move |server| server.put_settings(&player, &blob)));
    }

    /// Re-read the stored blob purely to learn the `updated_at` the server stamped on it.
    ///
    /// A successful `PUT` answers `204 No Content` and the server ignores the `updated_at` the
    /// client sent, so the new optimistic lock exists only server-side. Without this read-back the
    /// second save of a session would always lose the lock and report a conflict.
    fn refresh_sync_base(&mut self) {
        if self.sync_base_rx.is_some() || !self.session.is_logged_in() || self.config.server_url.is_none() {
            return;
        }
        let player = PlayerId { id: self.config.player_id.clone() };
        self.sync_base_rx = Some(spawn_query(self.server.clone(), move |server| server.get_settings(&player, SETTINGS_BLOB_NAME)));
    }

    /// Fetch the account's settings blob and apply it to this machine.
    pub(crate) fn start_settings_download(&mut self) {
        if self.sync_download_rx.is_some() || !self.require_account() {
            return;
        }
        let player = PlayerId { id: self.config.player_id.clone() };
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
        let merged = merge_downloaded(&self.current_settings(), payload.settings);
        self.autoplay = merged.autoplay;
        apply_settings(&mut self.config, &merged);
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
        if self.rivals_rx.is_some() || !self.session.is_logged_in() || self.config.server_url.is_none() {
            return;
        }
        let player = PlayerId { id: self.config.player_id.clone() };
        let rivals = self.config.rivals.clone();
        self.net_status = "saving rivals...".to_string();
        self.rivals_rx = Some(spawn_query(self.server.clone(), move |server| server.put_rivals(&player, &rivals)));
    }

    /// Pull the rival list the server holds, replacing the local cache.
    pub(crate) fn refresh_rivals(&mut self) {
        if self.rivals_rx.is_some() || !self.session.is_logged_in() || self.config.server_url.is_none() {
            return;
        }
        let player = PlayerId { id: self.config.player_id.clone() };
        self.rivals_rx = Some(spawn_query(self.server.clone(), move |server| server.rivals(&player)));
    }

    /// Keys inside the inline rival list. Typing an id runs through the same buffer as the settings
    /// rows, so only one editor is ever open.
    pub(crate) fn rivals_input(&mut self, code: KeyCode, typed: Option<&str>) {
        if self.text_input.is_some() {
            match code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let value = self.text_input.take().unwrap_or_default();
                    if let Some(id) = normalise_rival(&value, &self.config.rivals) {
                        self.config.rivals.push(id);
                        self.rivals_sel = self.config.rivals.len().saturating_sub(1);
                    }
                }
                KeyCode::Escape => self.text_input = None,
                KeyCode::Backspace => {
                    if let Some(buffer) = self.text_input.as_mut() {
                        buffer.pop();
                    }
                }
                _ => {
                    if let (Some(buffer), Some(text)) = (self.text_input.as_mut(), typed) {
                        buffer.extend(text.chars().filter(|c| !c.is_control()));
                    }
                }
            }
            return;
        }
        let rows = rival_rows(&self.config.rivals).len();
        let on_add = self.rivals_sel + 1 >= rows;
        match code {
            KeyCode::Escape => {
                self.rivals_open = false;
                self.push_rivals();
            }
            KeyCode::ArrowUp => self.rivals_sel = self.rivals_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.rivals_sel = (self.rivals_sel + 1).min(rows.saturating_sub(1)),
            KeyCode::Enter | KeyCode::NumpadEnter if on_add => {
                self.text_input = Some(String::new());
                self.text_secret = false;
                self.text_edit_row = None;
            }
            KeyCode::KeyD if !on_add && self.rivals_sel < self.config.rivals.len() => {
                self.config.rivals.remove(self.rivals_sel);
                self.rivals_sel = self.rivals_sel.min(rival_rows(&self.config.rivals).len().saturating_sub(1));
            }
            _ => {}
        }
    }

    /// Click inside the inline rival list: focus a row, or open the editor on the add row.
    pub(crate) fn rivals_click(&mut self, index: usize) {
        let rows = rival_rows(&self.config.rivals).len();
        self.rivals_sel = index.min(rows.saturating_sub(1));
        if self.rivals_sel + 1 >= rows {
            self.text_input = Some(String::new());
            self.text_secret = false;
            self.text_edit_row = None;
        }
    }

    /// Everything the settings screen draws this frame.
    pub(crate) fn settings_scene(&self) -> SettingsScene {
        let tab = self.set_tab.min(SETTING_TABS.len() - 1);
        let items = SETTING_TABS[tab].1;
        let sel = self.set_sel.min(items.len().saturating_sub(1));
        let editor = match (&self.text_input, self.rivals_open) {
            (Some(buffer), false) => Some(editor_display(buffer, self.text_secret)),
            _ => None,
        };
        let rivals = self.rivals_open.then(|| RivalsScene {
            rows: rival_rows(&self.config.rivals),
            sel: self.rivals_sel,
            editor: self.text_input.as_ref().map(|buffer| editor_display(buffer, false)),
        });
        SettingsScene {
            tabs: SETTING_TABS.iter().map(|(name, _)| *name).collect(),
            tab,
            rows: items.iter().map(|&index| self.setting_line(index)).collect(),
            sel,
            editor,
            status: self.net_status.clone(),
            rivals,
        }
    }

    /// Drain every background network reply. Called once per frame, before rendering.
    pub(crate) fn poll_network(&mut self) {
        if self.startup_whoami {
            self.startup_whoami = false;
            if self.session.is_logged_in() && self.config.server_url.is_some() {
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
                        self.config.player_id = id.to_string();
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
                Ok(Ok(())) => {
                    self.sync_lock.apply(SyncOutcome::Uploaded);
                    self.net_status = "settings uploaded".to_string();
                    self.refresh_sync_base();
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
                self.config.rivals = profiles.into_iter().map(|profile| profile.id).collect();
                self.rivals_sel = self.rivals_sel.min(rival_rows(&self.config.rivals).len().saturating_sub(1));
                self.net_status = format!("rivals: {}", self.config.rivals.len());
                self.save_settings();
            }
            Ok(Err(error)) => self.net_status = format!("rivals: {}", crate::ir_outcome::short_error(&error)),
            Err(TryRecvError::Empty) => self.rivals_rx = Some(rx),
            Err(TryRecvError::Disconnected) => {}
        }
    }
}
