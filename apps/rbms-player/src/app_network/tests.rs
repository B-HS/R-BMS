//! The NETWORK tab's account wiring against a score server that lives in the test: which account a
//! request is made as, what a logout tells the server, and what is let go of when the account
//! behind the session changes.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use rbms_ir::{
    AuthResponse, ChartId, CourseSubmission, PlayerProfile, ReplayData, ScoreRecord, ScoreSubmission, ServerInfo, SettingsPutResult, SubmitResponse,
};

use super::*;
use crate::ir_session::AccountSession;
use crate::{App, Config, LaunchOptions};

const SERVER_URL: &str = "https://account.invalid/api";
const TOKEN: &str = "token-of-dj";
const ACCOUNT: &str = "dj";
const OTHER_ACCOUNT: &str = "someone";
const TYPED_ID: &str = "dj2";
const OLD_FRIEND: &str = "friend-of-dj";
const LOCK_STAMP: i64 = 5;
const CALL_WAIT: Duration = Duration::from_secs(10);
const CALL_POLL: Duration = Duration::from_millis(2);

/// A score server that writes down which call it was made, and as whom.
#[derive(Default)]
struct AccountDouble {
    calls: Mutex<Vec<(&'static str, String)>>,
    unreachable: AtomicBool,
}

impl AccountDouble {
    fn note(&self, call: &'static str, who: &str) {
        self.calls.lock().expect("the calls").push((call, who.to_string()));
    }

    /// The accounts `call` was made as so far, in the order it was made.
    fn made(&self, call: &str) -> Vec<String> {
        self.calls.lock().expect("the calls").iter().filter(|(name, _)| *name == call).map(|(_, who)| who.clone()).collect()
    }

    /// The accounts `call` was made as, once it has been made `count` times.
    fn wait_for(&self, call: &str, count: usize) -> Vec<String> {
        let deadline = Instant::now() + CALL_WAIT;
        while self.made(call).len() < count {
            assert!(Instant::now() < deadline, "{call} was never made");
            std::thread::sleep(CALL_POLL);
        }
        self.made(call)
    }
}

impl ScoreServer for AccountDouble {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
    }
    fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        Ok(Vec::new())
    }
    fn player_best(&self, _chart: &ChartId, player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
        self.note("player_best", &player.id);
        Ok(None)
    }
    fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
        Err(IrError::Unsupported)
    }
    fn rivals(&self, player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        self.note("rivals", &player.id);
        Ok(Vec::new())
    }
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::Unsupported)
    }
    fn get_settings(&self, player: &PlayerId, _name: &str) -> Result<rbms_ir::SettingsBlob, IrError> {
        self.note("get_settings", &player.id);
        Err(IrError::NotFound("no blob".into()))
    }
    fn put_settings(&self, player: &PlayerId, _blob: &rbms_ir::SettingsBlob) -> Result<SettingsPutResult, IrError> {
        self.note("put_settings", &player.id);
        Ok(SettingsPutResult { updated_at: LOCK_STAMP, from_server: true })
    }
    fn put_rivals(&self, player: &PlayerId, _rivals: &[String]) -> Result<Vec<PlayerProfile>, IrError> {
        self.note("put_rivals", &player.id);
        Ok(Vec::new())
    }
    fn revoke_current_token(&self) -> Result<(), IrError> {
        self.note("revoke_current_token", "");
        if self.unreachable.load(Ordering::SeqCst) { Err(IrError::Network("connection refused".into())) } else { Ok(()) }
    }
}

/// An app signed in to [`SERVER_URL`] as [`ACCOUNT`], whose server is a double, with the settings in
/// a folder of the test's own.
struct Fixture {
    shared: AppShared,
    server: Arc<AccountDouble>,
}

impl Fixture {
    fn signed_in(tag: &str) -> Fixture {
        let folder = std::env::temp_dir().join(format!("rbms-app-network-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let mut config = Config::default();
        config.network.server_url = Some(SERVER_URL.into());
        config.network.player_id = ACCOUNT.into();
        let mut shared = App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron")).shared;
        let server = Arc::new(AccountDouble::default());
        shared.server = server.clone();
        shared.profile_servers = shared.multi_ir.build_servers_reusing(Some(shared.server.clone()));
        shared.session = AccountSession::restored(Some(TOKEN.into()), Some(ACCOUNT.into()));
        Fixture { shared, server }
    }

    /// Answer the login in flight with `account` and a token of its own, and take the answer.
    fn log_in_as(&mut self, account: &str) {
        let (tx, rx) = channel();
        tx.send(Ok(AuthResponse { token: format!("token-of-{account}"), player: PlayerId { id: account.into() }, name: account.into() })).expect("the answer");
        self.shared.session.begin(AuthAction::Login);
        self.shared.auth_rx = Some((AuthAction::Login, rx));
        self.shared.poll_auth();
    }

    /// Hold `answer` as the reply to a probe of who owns the token, and take it.
    fn hear_whoami(&mut self, answer: Result<String, IrError>) {
        let (tx, rx) = channel();
        tx.send(answer).expect("the answer");
        self.shared.whoami_rx = Some(rx);
        self.shared.poll_whoami();
    }

    /// Give the app what an earlier account left on the machine.
    fn remember_the_account(&mut self) {
        self.shared.config.network.rivals = vec![OLD_FRIEND.to_string()];
        self.shared.sync_lock.apply(SyncOutcome::Read(LOCK_STAMP));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(folder) = self.shared.settings_path.parent() {
            let _ = std::fs::remove_dir_all(folder);
        }
    }
}

fn friend_profile() -> PlayerProfile {
    PlayerProfile { id: OLD_FRIEND.into(), name: OLD_FRIEND.into(), total_plays: 0, rank_points: 0.0, extra: Default::default() }
}

fn is_forgotten(shared: &AppShared) -> bool {
    shared.config.network.rivals.is_empty() && shared.sync_lock.base().is_none()
}

#[test]
fn a_settings_upload_is_made_as_the_account_the_token_belongs_to_not_the_player_id_typed_after_it() {
    let mut f = Fixture::signed_in("upload");
    f.shared.config.network.player_id = TYPED_ID.into();
    f.shared.start_settings_upload();
    assert_eq!(f.server.wait_for("put_settings", 1), [ACCOUNT], "the server refuses 403 an upload under any id but the token's owner");
}

#[test]
fn a_settings_download_is_made_as_the_account_the_token_belongs_to() {
    let mut f = Fixture::signed_in("download");
    f.shared.config.network.player_id = TYPED_ID.into();
    f.shared.start_settings_download();
    assert_eq!(f.server.wait_for("get_settings", 1), [ACCOUNT]);
}

#[test]
fn the_rival_list_is_read_and_written_as_the_account_the_token_belongs_to() {
    let mut f = Fixture::signed_in("rivals");
    f.shared.config.network.player_id = TYPED_ID.into();
    f.shared.refresh_rivals();
    assert_eq!(f.server.wait_for("rivals", 1), [ACCOUNT]);
    f.shared.rivals_rx = None;
    f.shared.push_rivals();
    assert_eq!(f.server.wait_for("put_rivals", 1), [ACCOUNT]);
}

#[test]
fn the_you_row_of_the_ranking_is_the_account_the_token_belongs_to() {
    let mut f = Fixture::signed_in("ranking");
    f.shared.config.network.player_id = TYPED_ID.into();
    f.shared.start_ranking_fetch(ChartId { md5: "a".repeat(32), sha256: String::new() });
    assert_eq!(f.server.wait_for("player_best", 1), [ACCOUNT]);
}

#[test]
fn a_token_whose_owner_is_not_named_yet_makes_no_request_as_anybody() {
    let mut f = Fixture::signed_in("unnamed");
    f.shared.session = AccountSession::restored(Some(TOKEN.into()), None);
    f.shared.start_settings_upload();
    assert_eq!(f.shared.net_status, ACCOUNT_NOT_CONFIRMED);
    f.shared.start_settings_download();
    f.shared.refresh_rivals();
    f.shared.push_rivals();
    assert!(f.shared.sync_upload_rx.is_none() && f.shared.sync_download_rx.is_none() && f.shared.rivals_rx.is_none(), "a request was made as nobody");
    assert!(f.server.calls.lock().expect("the calls").is_empty());
}

#[test]
fn a_confirmed_session_names_its_account_in_the_player_id_row() {
    let mut f = Fixture::signed_in("confirmed");
    f.shared.config.network.player_id = TYPED_ID.into();
    f.hear_whoami(Ok(ACCOUNT.into()));
    assert_eq!(f.shared.config.network.player_id, ACCOUNT, "after a restart the row still named an id the token does not belong to");
}

#[test]
fn a_probe_that_names_another_owner_than_the_stored_one_replaces_the_account_and_forgets_the_old_ones_data() {
    let mut f = Fixture::signed_in("other-owner");
    f.shared.session = AccountSession::restored(Some(TOKEN.into()), Some(OTHER_ACCOUNT.into()));
    f.remember_the_account();
    f.hear_whoami(Ok(ACCOUNT.into()));
    assert_eq!(f.shared.session.signed_in_as(), Some(ACCOUNT));
    assert_eq!(f.shared.config.network.player_id, ACCOUNT);
    assert!(is_forgotten(&f.shared), "the rivals and the settings lock of {OTHER_ACCOUNT} were kept for {ACCOUNT}");
}

#[test]
fn a_probe_that_only_learns_the_owner_of_a_restored_token_keeps_what_is_held() {
    let mut f = Fixture::signed_in("learns-owner");
    f.shared.session = AccountSession::restored(Some(TOKEN.into()), None);
    f.remember_the_account();
    f.hear_whoami(Ok(ACCOUNT.into()));
    assert_eq!(f.shared.session.signed_in_as(), Some(ACCOUNT));
    assert_eq!(f.shared.config.network.rivals, [OLD_FRIEND.to_string()], "the cached list is kept until the server's replaces it");
    assert_eq!(f.shared.sync_lock.base(), Some(LOCK_STAMP));
}

#[test]
fn a_firewall_refusing_the_identity_probe_does_not_sign_the_player_out() {
    let mut f = Fixture::signed_in("forbidden");
    f.hear_whoami(Err(IrError::Forbidden("blocked by the proxy".into())));
    assert!(f.shared.session.is_logged_in(), "a 403 from a proxy threw the token away");
    assert_eq!(f.shared.session.signed_in_as(), Some(ACCOUNT));
}

#[test]
fn a_probe_the_server_answers_401_signs_the_player_out_of_the_stored_token() {
    let mut f = Fixture::signed_in("unauthorized");
    f.hear_whoami(Err(IrError::Unauthorized("revoked".into())));
    assert!(!f.shared.session.is_logged_in());
}

#[test]
fn logging_out_forgets_the_rival_list_and_the_settings_lock() {
    let mut f = Fixture::signed_in("logout-forgets");
    f.remember_the_account();
    f.shared.finish_logout();
    assert!(is_forgotten(&f.shared), "the rivals of {ACCOUNT} stayed on the machine after the logout");
}

#[test]
fn a_logout_tells_the_server_to_revoke_the_token_it_carried() {
    let mut f = Fixture::signed_in("logout-revokes");
    f.shared.finish_logout();
    assert_eq!(f.server.wait_for("revoke_current_token", 1).len(), 1, "the token stayed valid on the server after the logout");
    assert!(!f.shared.session.is_logged_in());
}

#[test]
fn a_logout_finishes_here_even_when_the_server_cannot_be_reached() {
    let mut f = Fixture::signed_in("logout-offline");
    f.server.unreachable.store(true, Ordering::SeqCst);
    f.shared.finish_logout();
    f.server.wait_for("revoke_current_token", 1);
    assert!(!f.shared.session.is_logged_in());
    assert_eq!(f.shared.config.network.ir_token, None);
    assert_eq!(f.shared.config.network.player_id, GUEST_PLAYER_ID);
    assert_eq!(f.shared.net_status, "logged out");
}

#[test]
fn switching_straight_to_another_account_starts_it_from_nothing_of_the_last_ones() {
    let mut f = Fixture::signed_in("switch");
    f.remember_the_account();
    f.log_in_as(OTHER_ACCOUNT);
    assert_eq!(f.shared.session.signed_in_as(), Some(OTHER_ACCOUNT));
    assert!(is_forgotten(&f.shared), "the settings lock of {ACCOUNT} made every upload of {OTHER_ACCOUNT} conflict");
}

#[test]
fn logging_in_again_as_the_same_account_keeps_its_rivals_and_its_settings_lock() {
    let mut f = Fixture::signed_in("relogin");
    f.remember_the_account();
    f.log_in_as(ACCOUNT);
    assert_eq!(f.shared.config.network.rivals, [OLD_FRIEND.to_string()]);
    assert_eq!(f.shared.sync_lock.base(), Some(LOCK_STAMP));
}

#[test]
fn a_rival_list_that_arrives_after_the_account_changed_is_not_applied_to_the_new_one() {
    let mut f = Fixture::signed_in("late-rivals");
    let (tx, rx) = channel();
    tx.send(Ok(vec![friend_profile()])).expect("the answer");
    f.shared.rivals_rx = Some(rx);
    f.log_in_as(OTHER_ACCOUNT);
    f.shared.poll_rivals();
    assert!(f.shared.config.network.rivals.is_empty(), "the rivals of {ACCOUNT} were applied to {OTHER_ACCOUNT}");
}

#[test]
fn an_identity_probe_that_answers_after_the_token_was_replaced_is_not_applied() {
    let mut f = Fixture::signed_in("late-whoami");
    let (tx, rx) = channel();
    tx.send(Ok(ACCOUNT.to_string())).expect("the answer");
    f.shared.whoami_rx = Some(rx);
    f.log_in_as(OTHER_ACCOUNT);
    f.shared.poll_whoami();
    assert_eq!(f.shared.session.signed_in_as(), Some(OTHER_ACCOUNT), "the answer for the old token named the new session {ACCOUNT}");
}

#[test]
fn rebuilding_the_server_lets_go_of_every_request_made_as_the_account() {
    let mut f = Fixture::signed_in("rebuild");
    let (_whoami_tx, whoami_rx) = channel();
    let (_rivals_tx, rivals_rx) = channel();
    let (_upload_tx, upload_rx) = channel();
    let (_base_tx, base_rx) = channel();
    let (_download_tx, download_rx) = channel();
    f.shared.whoami_rx = Some(whoami_rx);
    f.shared.rivals_rx = Some(rivals_rx);
    f.shared.sync_upload_rx = Some(upload_rx);
    f.shared.sync_base_rx = Some(base_rx);
    f.shared.sync_download_rx = Some(download_rx);

    f.shared.rebuild_server();

    assert!(
        f.shared.whoami_rx.is_none()
            && f.shared.rivals_rx.is_none()
            && f.shared.sync_upload_rx.is_none()
            && f.shared.sync_base_rx.is_none()
            && f.shared.sync_download_rx.is_none(),
        "a request made to the old server or with the old token was kept"
    );
}
