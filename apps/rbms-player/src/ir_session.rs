//! The IR account session the NETWORK settings tab drives: which identity the client is signed in
//! as, the bearer token that proves it, and the login / register / logout state machine.
//!
//! The state lives here rather than in `App` so the whole flow can be exercised against a
//! [`ScoreServer`] double: `begin` arms a request, the worker thread runs [`run_auth`], and
//! `finish` folds the result back in.

use rbms_ir::{AuthRequest, AuthResponse, IrError, ScoreServer};

use crate::ir_outcome::{short_error, truncate};

/// Player id used when no account is signed in. The server accepts a tokenless submission under
/// this id alone and rejects every other one with 401, so it is the wire contract, not a label.
pub(crate) use rbms_ir::GUEST_PLAYER_ID;

/// Which credential exchange is in flight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AuthAction {
    Login,
    Register,
}

impl AuthAction {
    /// The ACCOUNT row text while the request is running.
    pub(crate) fn pending_text(self) -> &'static str {
        match self {
            AuthAction::Login => "logging in...",
            AuthAction::Register => "registering...",
        }
    }

    /// Prefix of the ACCOUNT row text after the request failed.
    pub(crate) fn failure_prefix(self) -> &'static str {
        match self {
            AuthAction::Login => "login failed",
            AuthAction::Register => "register failed",
        }
    }
}

/// The ACCOUNT row's four states.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum AccountState {
    Guest,
    Busy(AuthAction),
    LoggedIn,
    Failed(String),
}

/// The signed-in identity plus the state of the last credential exchange.
///
/// The password is deliberately absent: it is held by the caller only long enough to build one
/// [`AuthRequest`], and never reaches this struct, the settings file or a log line.
#[derive(Clone, Debug)]
pub(crate) struct AccountSession {
    token: Option<String>,
    login_id: Option<String>,
    state: AccountState,
}

impl AccountSession {
    /// Restore a session from the persisted token/login id. A stored token is treated as signed in
    /// until the server says otherwise (see [`AccountSession::finish_whoami`]).
    pub(crate) fn restored(token: Option<String>, login_id: Option<String>) -> AccountSession {
        let state = if token.is_some() { AccountState::LoggedIn } else { AccountState::Guest };
        AccountSession { token, login_id, state }
    }

    pub(crate) fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub(crate) fn login_id(&self) -> Option<&str> {
        self.login_id.as_deref()
    }

    /// The raw state, for the tests that pin the state machine.
    #[cfg(test)]
    pub(crate) fn state(&self) -> &AccountState {
        &self.state
    }

    pub(crate) fn is_logged_in(&self) -> bool {
        self.token.is_some()
    }

    pub(crate) fn is_busy(&self) -> bool {
        matches!(self.state, AccountState::Busy(_))
    }

    /// Arm a credential exchange. The previous session is kept until it either succeeds (replacing
    /// it) or fails (leaving it untouched), so a mistyped password never signs the user out.
    pub(crate) fn begin(&mut self, action: AuthAction) {
        self.state = AccountState::Busy(action);
    }

    /// Fold a finished exchange back in. Returns `true` when the stored token changed, which is the
    /// caller's signal to rebuild the score server with the new credentials.
    pub(crate) fn finish(&mut self, action: AuthAction, result: Result<AuthResponse, IrError>) -> bool {
        match result {
            Ok(response) => {
                let changed = self.token.as_deref() != Some(response.token.as_str());
                self.token = Some(response.token);
                self.login_id = Some(response.player.id);
                self.state = AccountState::LoggedIn;
                changed
            }
            Err(error) => {
                self.state = AccountState::Failed(format!("{}: {}", action.failure_prefix(), short_error(&error)));
                false
            }
        }
    }

    /// Fold a `whoami` probe back in: it confirms the restored identity, or reports that the stored
    /// token is no longer accepted. Returns `true` when the token was dropped.
    pub(crate) fn finish_whoami(&mut self, result: Result<String, IrError>) -> bool {
        match result {
            Ok(id) => {
                self.login_id = Some(id);
                self.state = AccountState::LoggedIn;
                false
            }
            Err(error) if error.is_auth_failure() => {
                self.token = None;
                self.state = AccountState::Failed(format!("session expired: {}", short_error(&error)));
                true
            }
            Err(_) => false,
        }
    }

    /// Drop the token and identity. Returns `true` when there was a session to end.
    pub(crate) fn logout(&mut self) -> bool {
        let had_token = self.token.is_some();
        self.token = None;
        self.login_id = None;
        self.state = AccountState::Guest;
        had_token
    }

    /// The ACCOUNT row value.
    pub(crate) fn status_text(&self) -> String {
        match &self.state {
            AccountState::Guest => GUEST_PLAYER_ID.to_string(),
            AccountState::Busy(action) => action.pending_text().to_string(),
            AccountState::LoggedIn => match self.login_id() {
                Some(id) => format!("logged in as {id}"),
                None => "logged in".to_string(),
            },
            AccountState::Failed(message) => truncate(message),
        }
    }
}

/// The id a score is submitted under.
///
/// Only a signed-in client may submit under a real account id: without a bearer token the server
/// answers 401 for anything but [`GUEST_PLAYER_ID`], and the score is lost. So a configured id is
/// used only while a session holds a token, and every other run submits as a guest.
pub(crate) fn submission_player_id(session: &AccountSession, configured: &str) -> String {
    let configured = configured.trim();
    if session.is_logged_in() && !configured.is_empty() {
        return configured.to_string();
    }
    GUEST_PLAYER_ID.to_string()
}

/// Run one credential exchange. Blocking: call it from a worker thread.
pub(crate) fn run_auth(server: &dyn ScoreServer, action: AuthAction, request: &AuthRequest) -> Result<AuthResponse, IrError> {
    match action {
        AuthAction::Login => server.login(request),
        AuthAction::Register => server.register(request),
    }
}

/// Build the request for `action` from the NETWORK tab's fields. Register carries the email and
/// uses the login id as the display name; login sends only the credentials.
pub(crate) fn auth_request(action: AuthAction, login_id: &str, password: &str, email: &str) -> AuthRequest {
    match action {
        AuthAction::Login => AuthRequest::login(login_id, password),
        AuthAction::Register => AuthRequest::register(login_id, password, email, Some(login_id.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_ir::{ChartId, PlayerId, ScoreRecord, ServerInfo, SubmitResponse};

    /// A [`ScoreServer`] that answers auth calls from a script and nothing else.
    struct AuthDouble {
        login: fn() -> Result<AuthResponse, IrError>,
        register: fn() -> Result<AuthResponse, IrError>,
    }

    fn ok_response() -> Result<AuthResponse, IrError> {
        Ok(AuthResponse { token: "tok-new".into(), player: PlayerId { id: "dj".into() }, name: "DJ".into() })
    }

    fn unauthorized() -> Result<AuthResponse, IrError> {
        Err(IrError::Unauthorized("bad credentials".into()))
    }

    fn offline() -> Result<AuthResponse, IrError> {
        Err(IrError::Network("connection refused".into()))
    }

    impl ScoreServer for AuthDouble {
        fn health(&self) -> Result<ServerInfo, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_score(&self, _sub: &rbms_ir::ScoreSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
        }
        fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
            Err(IrError::Unsupported)
        }
        fn player_best(&self, _chart: &ChartId, _player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
            Err(IrError::Unsupported)
        }
        fn player_profile(&self, _player: &PlayerId) -> Result<rbms_ir::PlayerProfile, IrError> {
            Err(IrError::Unsupported)
        }
        fn rivals(&self, _player: &PlayerId) -> Result<Vec<rbms_ir::PlayerProfile>, IrError> {
            Err(IrError::Unsupported)
        }
        fn submit_course(&self, _sub: &rbms_ir::CourseSubmission) -> Result<SubmitResponse, IrError> {
            Err(IrError::Unsupported)
        }
        fn upload_replay(&self, _chart: &ChartId, _replay: &rbms_ir::ReplayData) -> Result<String, IrError> {
            Err(IrError::Unsupported)
        }
        fn login(&self, _req: &AuthRequest) -> Result<AuthResponse, IrError> {
            (self.login)()
        }
        fn register(&self, _req: &AuthRequest) -> Result<AuthResponse, IrError> {
            (self.register)()
        }
    }

    fn double(login: fn() -> Result<AuthResponse, IrError>, register: fn() -> Result<AuthResponse, IrError>) -> AuthDouble {
        AuthDouble { login, register }
    }

    #[test]
    fn a_fresh_session_is_a_guest() {
        let session = AccountSession::restored(None, None);
        assert!(!session.is_logged_in());
        assert_eq!(session.status_text(), "guest");
        assert_eq!(session.state(), &AccountState::Guest);
    }

    #[test]
    fn a_restored_token_reads_as_signed_in() {
        let session = AccountSession::restored(Some("tok".into()), Some("dj".into()));
        assert!(session.is_logged_in());
        assert_eq!(session.status_text(), "logged in as dj");
    }

    #[test]
    fn login_shows_progress_then_stores_the_token_and_identity() {
        let server = double(ok_response, unauthorized);
        let mut session = AccountSession::restored(None, None);
        session.begin(AuthAction::Login);
        assert_eq!(session.status_text(), "logging in...");
        assert!(session.is_busy());

        let request = auth_request(AuthAction::Login, "dj", "hunter2", "");
        let changed = session.finish(AuthAction::Login, run_auth(&server, AuthAction::Login, &request));
        assert!(changed, "a new token means the score server has to be rebuilt");
        assert_eq!(session.token(), Some("tok-new"));
        assert_eq!(session.login_id(), Some("dj"));
        assert_eq!(session.status_text(), "logged in as dj");
    }

    #[test]
    fn register_reports_its_own_progress_text_and_sends_the_email() {
        let server = double(unauthorized, ok_response);
        let mut session = AccountSession::restored(None, None);
        session.begin(AuthAction::Register);
        assert_eq!(session.status_text(), "registering...");

        let request = auth_request(AuthAction::Register, "dj", "hunter2", "dj@example.test");
        assert_eq!(request.email.as_deref(), Some("dj@example.test"));
        assert_eq!(request.name.as_deref(), Some("dj"), "the login id doubles as the display name");
        assert_eq!(request.api_version, rbms_ir::API_VERSION, "a zero api_version is rejected by the server");

        session.finish(AuthAction::Register, run_auth(&server, AuthAction::Register, &request));
        assert_eq!(session.token(), Some("tok-new"));
    }

    #[test]
    fn a_rejected_login_reports_the_reason_and_keeps_the_previous_session() {
        let server = double(unauthorized, ok_response);
        let mut session = AccountSession::restored(Some("old".into()), Some("dj".into()));
        session.begin(AuthAction::Login);
        let request = auth_request(AuthAction::Login, "dj", "wrong", "");
        let changed = session.finish(AuthAction::Login, run_auth(&server, AuthAction::Login, &request));

        assert!(!changed, "a failed login never rebuilds the server");
        assert_eq!(session.token(), Some("old"), "the working session survives a bad password");
        let status = session.status_text();
        assert!(status.starts_with("login failed: "), "{status}");
        assert!(status.contains("bad credentials"), "{status}");
    }

    #[test]
    fn a_network_failure_is_reported_the_same_way_as_a_rejection() {
        let server = double(offline, offline);
        let mut session = AccountSession::restored(None, None);
        session.begin(AuthAction::Login);
        let request = auth_request(AuthAction::Login, "dj", "hunter2", "");
        session.finish(AuthAction::Login, run_auth(&server, AuthAction::Login, &request));
        let status = session.status_text();
        assert!(status.starts_with("login failed: "), "{status}");
        assert!(status.contains("connection refused"), "{status}");
        assert!(!session.is_logged_in(), "a failed first login leaves the client a guest");
    }

    #[test]
    fn register_failure_uses_the_register_prefix() {
        let server = double(ok_response, unauthorized);
        let mut session = AccountSession::restored(None, None);
        session.begin(AuthAction::Register);
        let request = auth_request(AuthAction::Register, "dj", "hunter2", "dj@example.test");
        session.finish(AuthAction::Register, run_auth(&server, AuthAction::Register, &request));
        assert!(session.status_text().starts_with("register failed: "));
    }

    #[test]
    fn logout_clears_the_token_and_identity() {
        let mut session = AccountSession::restored(Some("tok".into()), Some("dj".into()));
        assert!(session.logout(), "there was a session to end");
        assert_eq!(session.token(), None);
        assert_eq!(session.login_id(), None);
        assert_eq!(session.status_text(), "guest");
        assert!(!session.logout(), "logging out twice is a no-op");
    }

    #[test]
    fn re_login_with_the_same_token_reports_no_change() {
        let mut session = AccountSession::restored(Some("tok-new".into()), Some("dj".into()));
        let changed = session.finish(AuthAction::Login, ok_response());
        assert!(!changed, "an identical token needs no server rebuild");
        assert!(session.is_logged_in());
    }

    #[test]
    fn whoami_confirms_a_restored_identity() {
        let mut session = AccountSession::restored(Some("tok".into()), None);
        assert!(!session.finish_whoami(Ok("dj".to_string())));
        assert_eq!(session.status_text(), "logged in as dj");
    }

    #[test]
    fn whoami_drops_a_token_the_server_no_longer_accepts() {
        let mut session = AccountSession::restored(Some("stale".into()), Some("dj".into()));
        assert!(session.finish_whoami(Err(IrError::Unauthorized("token expired".into()))));
        assert_eq!(session.token(), None);
        assert!(session.status_text().starts_with("session expired: "));
    }

    #[test]
    fn whoami_keeps_the_token_when_the_server_is_merely_unreachable() {
        let mut session = AccountSession::restored(Some("tok".into()), Some("dj".into()));
        assert!(!session.finish_whoami(Err(IrError::Network("timeout".into()))));
        assert_eq!(session.token(), Some("tok"), "an offline server is not a revoked token");
    }

    #[test]
    fn a_signed_in_client_submits_under_its_account_id() {
        let session = AccountSession::restored(Some("tok".into()), Some("dj".into()));
        assert_eq!(submission_player_id(&session, "dj"), "dj");
    }

    #[test]
    fn a_client_that_logged_out_submits_as_a_guest_not_under_the_old_account_id() {
        let mut session = AccountSession::restored(Some("tok".into()), Some("dj".into()));
        assert!(session.logout());
        assert_eq!(submission_player_id(&session, "dj"), GUEST_PLAYER_ID, "a tokenless submission under a real id is rejected 401 and the score is lost");
    }

    #[test]
    fn a_client_whose_token_was_revoked_submits_as_a_guest() {
        let mut session = AccountSession::restored(Some("tok".into()), Some("dj".into()));
        assert!(session.finish_whoami(Err(IrError::Unauthorized("expired".into()))));
        assert_eq!(submission_player_id(&session, "dj"), GUEST_PLAYER_ID);
    }

    #[test]
    fn a_blank_configured_id_falls_back_to_the_guest_id() {
        let session = AccountSession::restored(Some("tok".into()), None);
        assert_eq!(submission_player_id(&session, "   "), GUEST_PLAYER_ID);
    }
}
