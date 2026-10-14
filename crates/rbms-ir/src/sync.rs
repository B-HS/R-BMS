//! What one answer of the score server means for a run a client keeps sending until the server has
//! it: taken, refused for a reason the run itself carries, or not answered yet.
//!
//! Pure classification, with no thread and no state. A record sync walks the plays a machine holds
//! and sends each one; whether it moves on, waits or stops, and what it writes down of each play,
//! is decided off these verdicts, so every branch can be checked without a server.

use crate::IrError;
use crate::dto::SubmitResponse;
use crate::error::ServerReason;
use crate::worker::is_replay_upload_warranted;

/// Lowest HTTP status that is the server's own failure rather than the request's.
const SERVER_FAILURE_STATUS_FLOOR: u16 = 500;

/// What the server refused a request over: the HTTP status, the error code and the field and reason
/// it named, and one readable line of the whole answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rejection {
    pub status: Option<u16>,
    pub code: Option<String>,
    pub field: Option<String>,
    pub reason: Option<String>,
    pub detail: String,
}

impl Rejection {
    fn of_error(error: &IrError) -> Rejection {
        let ServerReason { code, field, reason } = error.server_reason().unwrap_or_default();
        Rejection { status: error.status(), code: Some(code).filter(|code| !code.is_empty()), field, reason, detail: error.to_string() }
    }
}

/// What a score submission's answer means for the run that was sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitVerdict {
    /// The server holds the run: it stored it now, or `already` held it from an earlier send.
    /// `replay_warranted` is whether its replay belongs on the server too
    /// ([`is_replay_upload_warranted`]).
    Accepted { score_id: Option<String>, already: bool, replay_warranted: bool },
    /// The server refused the run itself. Sending the same run again gets the same answer until the
    /// server's rules change, so it is sent again only on a later sync.
    Rejected(Rejection),
    /// The account sent faster than the server takes; the same run may be sent again after a wait.
    RateLimited,
    /// The server could not be reached or failed on its side; the run is sent again on a later sync.
    Transient { detail: String },
    /// Nothing more can be sent this sync: the account is not signed in, or there is no server to
    /// send to.
    StopSync { detail: String },
}

/// Read a score submission's answer.
pub fn submit_verdict(result: &Result<SubmitResponse, IrError>) -> SubmitVerdict {
    match result {
        Ok(response) if response.accepted => SubmitVerdict::Accepted {
            score_id: response.score_id.clone(),
            already: response.is_duplicate(),
            replay_warranted: is_replay_upload_warranted(response),
        },
        Ok(response) => SubmitVerdict::Rejected(Rejection {
            detail: response.message.as_deref().map(str::trim).filter(|message| !message.is_empty()).unwrap_or(REFUSED_WITHOUT_A_MESSAGE).to_string(),
            ..Rejection::default()
        }),
        Err(error) => match failure_of(error) {
            Failure::RateLimited => SubmitVerdict::RateLimited,
            Failure::Transient => SubmitVerdict::Transient { detail: error.to_string() },
            Failure::Stop => SubmitVerdict::StopSync { detail: error.to_string() },
            Failure::Refused => SubmitVerdict::Rejected(Rejection::of_error(error)),
        },
    }
}

/// What a replay upload's answer means for the replay that was sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadVerdict {
    /// The server holds the replay, under this id.
    Uploaded { replay_id: String },
    /// The server refused the replay itself.
    Rejected(Rejection),
    /// The account sent faster than the server takes.
    RateLimited,
    /// The server could not be reached or failed on its side.
    Transient { detail: String },
    /// Nothing more can be sent this sync.
    StopSync { detail: String },
}

/// Read a replay upload's answer, by the same rules as [`submit_verdict`].
pub fn upload_verdict(result: &Result<String, IrError>) -> UploadVerdict {
    match result {
        Ok(replay_id) => UploadVerdict::Uploaded { replay_id: replay_id.clone() },
        Err(error) => match failure_of(error) {
            Failure::RateLimited => UploadVerdict::RateLimited,
            Failure::Transient => UploadVerdict::Transient { detail: error.to_string() },
            Failure::Stop => UploadVerdict::StopSync { detail: error.to_string() },
            Failure::Refused => UploadVerdict::Rejected(Rejection::of_error(error)),
        },
    }
}

/// The detail of a `2xx` answer that did not take the run and said nothing about why.
const REFUSED_WITHOUT_A_MESSAGE: &str = "rejected";

/// The four ways a failed request ends.
enum Failure {
    RateLimited,
    Transient,
    Stop,
    Refused,
}

/// Sort a failure: a rate limit waits, a transport failure, an unreadable answer or a failure of
/// the server's own comes back later, a missing session or server stops the sync, and anything else
/// is the server refusing the request.
fn failure_of(error: &IrError) -> Failure {
    match error {
        IrError::RateLimited(_) => Failure::RateLimited,
        IrError::Network(_) | IrError::Decode(_) => Failure::Transient,
        IrError::Server(status, _) if *status >= SERVER_FAILURE_STATUS_FLOOR => Failure::Transient,
        IrError::Unauthorized(_) | IrError::NotConfigured | IrError::Unsupported => Failure::Stop,
        IrError::Server(..)
        | IrError::Forbidden(_)
        | IrError::NotFound(_)
        | IrError::Conflict(_)
        | IrError::SettingsConflict(_)
        | IrError::PayloadTooLarge(_) => Failure::Refused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DUPLICATE_SUBMISSION_MESSAGE;
    use crate::dto::ScoreFlag;
    use crate::error;

    /// The status a request the server finds malformed is refused with.
    const BAD_REQUEST_STATUS: u16 = 400;

    fn envelope(code: &str, details: &str) -> String {
        format!(r#"{{"success":false,"error":{{"code":"{code}","message":"m","details":{details}}}}}"#)
    }

    #[test]
    fn a_duplicate_submission_is_accepted_as_already_on_server() {
        let response = SubmitResponse {
            accepted: true,
            ranked: true,
            score_id: Some("sc_old".into()),
            message: Some(DUPLICATE_SUBMISSION_MESSAGE.into()),
            ..Default::default()
        };
        assert_eq!(submit_verdict(&Ok(response)), SubmitVerdict::Accepted { score_id: Some("sc_old".into()), already: true, replay_warranted: true });
        let fresh = SubmitResponse { accepted: true, score_id: Some("sc_new".into()), message: Some("saved".into()), ..Default::default() };
        assert_eq!(submit_verdict(&Ok(fresh)), SubmitVerdict::Accepted { score_id: Some("sc_new".into()), already: false, replay_warranted: true });
    }

    #[test]
    fn a_validation_error_is_a_rejection_with_its_field() {
        let body = envelope("VALIDATION_ERROR", r#"{"field":"judge","reason":"judge count exceeds total_notes"}"#);
        let SubmitVerdict::Rejected(rejection) = submit_verdict(&Err(IrError::Server(BAD_REQUEST_STATUS, body))) else {
            panic!("a 400 is a rejection");
        };
        assert_eq!(rejection.status, Some(BAD_REQUEST_STATUS));
        assert_eq!(rejection.code.as_deref(), Some("VALIDATION_ERROR"));
        assert_eq!(rejection.field.as_deref(), Some("judge"));
        assert_eq!(rejection.reason.as_deref(), Some("judge count exceeds total_notes"));
        assert_eq!(rejection.detail, "server error 400: VALIDATION_ERROR: judge (judge count exceeds total_notes)");

        let forbidden = submit_verdict(&Err(IrError::Forbidden(envelope("FORBIDDEN", "{}"))));
        assert!(matches!(forbidden, SubmitVerdict::Rejected(Rejection { status: Some(error::STATUS_FORBIDDEN), .. })), "got {forbidden:?}");
        let not_found = submit_verdict(&Err(IrError::NotFound("<html>".into())));
        assert!(matches!(not_found, SubmitVerdict::Rejected(Rejection { code: None, field: None, .. })), "a body that is no envelope names nothing");
        let refused = SubmitResponse { accepted: false, message: Some(" ".into()), ..Default::default() };
        assert_eq!(submit_verdict(&Ok(refused)), SubmitVerdict::Rejected(Rejection { detail: REFUSED_WITHOUT_A_MESSAGE.into(), ..Rejection::default() }));
    }

    #[test]
    fn a_rate_limited_submission_is_neither_accepted_nor_rejected() {
        assert_eq!(submit_verdict(&Err(IrError::RateLimited(envelope("RATE_LIMITED", "{}")))), SubmitVerdict::RateLimited);
        assert_eq!(upload_verdict(&Err(IrError::RateLimited(String::new()))), UploadVerdict::RateLimited);
    }

    #[test]
    fn network_and_server_errors_are_transient() {
        for error in [IrError::Network("reset".into()), IrError::Decode("eof".into()), IrError::Server(500, "boom".into()), IrError::Server(503, String::new())]
        {
            let verdict = submit_verdict(&Err(error));
            assert!(matches!(verdict, SubmitVerdict::Transient { .. }), "got {verdict:?}");
        }
        assert!(matches!(upload_verdict(&Err(IrError::Server(502, String::new()))), UploadVerdict::Transient { .. }));
    }

    #[test]
    fn an_unauthorized_submission_stops_the_sync() {
        for error in [IrError::Unauthorized(String::new()), IrError::NotConfigured, IrError::Unsupported] {
            let verdict = submit_verdict(&Err(error));
            assert!(matches!(verdict, SubmitVerdict::StopSync { .. }), "got {verdict:?}");
        }
        assert!(matches!(upload_verdict(&Err(IrError::Unauthorized(String::new()))), UploadVerdict::StopSync { .. }));
    }

    #[test]
    fn an_unranked_acceptance_warrants_no_replay() {
        let guest = SubmitResponse { accepted: true, flags: vec![ScoreFlag::Assist.as_wire().into()], score_id: Some("sc".into()), ..Default::default() };
        assert_eq!(submit_verdict(&Ok(guest)), SubmitVerdict::Accepted { score_id: Some("sc".into()), already: false, replay_warranted: false });
    }

    #[test]
    fn an_upload_is_uploaded_or_refused() {
        assert_eq!(upload_verdict(&Ok("rp_1".into())), UploadVerdict::Uploaded { replay_id: "rp_1".into() });
        let too_big = upload_verdict(&Err(IrError::PayloadTooLarge(envelope("IR_PAYLOAD_TOO_LARGE", "{}"))));
        assert!(matches!(too_big, UploadVerdict::Rejected(Rejection { status: Some(error::STATUS_PAYLOAD_TOO_LARGE), .. })), "got {too_big:?}");
    }
}
