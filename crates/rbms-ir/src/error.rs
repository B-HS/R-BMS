use std::fmt;

use serde::{Deserialize, Serialize};

use crate::dto::SettingsConflict;

/// HTTP status the IR server uses for "no credentials, or the bearer token is unknown".
pub const STATUS_UNAUTHORIZED: u16 = 401;
/// HTTP status the IR server uses for "authenticated, but this is not your resource".
pub const STATUS_FORBIDDEN: u16 = 403;
/// HTTP status the IR server uses for an unknown chart, player, score, replay or settings blob.
pub const STATUS_NOT_FOUND: u16 = 404;
/// HTTP status the IR server uses for an optimistic-lock clash or a duplicate account.
pub const STATUS_CONFLICT: u16 = 409;
/// HTTP status the IR server uses when a submission or replay exceeds its byte budget.
pub const STATUS_PAYLOAD_TOO_LARGE: u16 = 413;
/// HTTP status the IR server uses when a per-ip or per-account rate limit is exhausted.
pub const STATUS_RATE_LIMITED: u16 = 429;

/// Longest server-supplied detail kept in a rendered [`IrError`] message. Bodies are attacker- and
/// accident-shaped (HTML error pages, stack traces), and the text lands in the game UI.
pub const ERROR_DETAIL_MAX_CHARS: usize = 200;

const TRUNCATION_SUFFIX: &str = "…";

/// The `error` object inside the server's failure envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

/// The failure envelope every non-2xx rbms IR response carries: `{"success":false,"error":{…}}`.
/// Parsing is best-effort — a proxy or a crash can answer with something else entirely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorEnvelope {
    #[serde(default)]
    pub success: bool,
    pub error: ErrorBody,
}

impl ErrorEnvelope {
    /// Parse a response body as the server's failure envelope, or `None` when it is not one.
    pub fn parse(body: &str) -> Option<ErrorEnvelope> {
        serde_json::from_str(body).ok()
    }
}

/// Every way an IR call can fail. The HTTP statuses the rbms IR server assigns meaning to get their
/// own variant so callers can branch without re-parsing a status code; anything else stays in
/// [`IrError::Server`]. Each carrying variant keeps the raw response body verbatim.
#[derive(Debug)]
pub enum IrError {
    NotConfigured,
    Network(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    SettingsConflict(Box<SettingsConflict>),
    PayloadTooLarge(String),
    RateLimited(String),
    Server(u16, String),
    Decode(String),
    Unsupported,
}

impl IrError {
    /// Map a failed HTTP status and its body onto the matching variant.
    pub fn from_status(status: u16, body: String) -> IrError {
        match status {
            STATUS_UNAUTHORIZED => IrError::Unauthorized(body),
            STATUS_FORBIDDEN => IrError::Forbidden(body),
            STATUS_NOT_FOUND => IrError::NotFound(body),
            STATUS_CONFLICT => IrError::Conflict(body),
            STATUS_PAYLOAD_TOO_LARGE => IrError::PayloadTooLarge(body),
            STATUS_RATE_LIMITED => IrError::RateLimited(body),
            other => IrError::Server(other, body),
        }
    }

    /// The HTTP status behind this failure, when it came from a response at all.
    pub fn status(&self) -> Option<u16> {
        match self {
            IrError::Unauthorized(_) => Some(STATUS_UNAUTHORIZED),
            IrError::Forbidden(_) => Some(STATUS_FORBIDDEN),
            IrError::NotFound(_) => Some(STATUS_NOT_FOUND),
            IrError::Conflict(_) | IrError::SettingsConflict(_) => Some(STATUS_CONFLICT),
            IrError::PayloadTooLarge(_) => Some(STATUS_PAYLOAD_TOO_LARGE),
            IrError::RateLimited(_) => Some(STATUS_RATE_LIMITED),
            IrError::Server(status, _) => Some(*status),
            IrError::NotConfigured | IrError::Network(_) | IrError::Decode(_) | IrError::Unsupported => None,
        }
    }

    /// The raw response (or transport) detail, empty when the variant carries none.
    pub fn detail(&self) -> &str {
        match self {
            IrError::Network(detail)
            | IrError::Unauthorized(detail)
            | IrError::Forbidden(detail)
            | IrError::NotFound(detail)
            | IrError::Conflict(detail)
            | IrError::PayloadTooLarge(detail)
            | IrError::RateLimited(detail)
            | IrError::Server(_, detail)
            | IrError::Decode(detail) => detail,
            IrError::NotConfigured | IrError::SettingsConflict(_) | IrError::Unsupported => "",
        }
    }

    /// The server's machine-readable error code (`IR_ACCOUNT_EXISTS`, `RATE_LIMITED`, …) when the
    /// body was a failure envelope.
    pub fn error_code(&self) -> Option<String> {
        ErrorEnvelope::parse(self.detail()).map(|envelope| envelope.error.code)
    }

    /// True when the call failed because the client is signed out or not entitled — the app should
    /// send the player to the NETWORK login flow rather than retry.
    pub fn is_auth_failure(&self) -> bool {
        matches!(self, IrError::Unauthorized(_) | IrError::Forbidden(_))
    }

    /// True when the same request may succeed later without any change from the player.
    pub fn is_retryable(&self) -> bool {
        match self {
            IrError::Network(_) | IrError::RateLimited(_) => true,
            IrError::Server(status, _) => *status >= SERVER_ERROR_STATUS_FLOOR,
            _ => false,
        }
    }
}

const SERVER_ERROR_STATUS_FLOOR: u16 = 500;

fn readable_detail(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let text = match ErrorEnvelope::parse(trimmed) {
        Some(envelope) => format!("{} ({})", envelope.error.message, envelope.error.code),
        None => trimmed.to_string(),
    };
    if text.chars().count() <= ERROR_DETAIL_MAX_CHARS {
        return text;
    }
    let head: String = text.chars().take(ERROR_DETAIL_MAX_CHARS).collect();
    format!("{head}{TRUNCATION_SUFFIX}")
}

fn write_labelled(f: &mut fmt::Formatter<'_>, label: &str, raw_detail: &str) -> fmt::Result {
    let detail = readable_detail(raw_detail);
    if detail.is_empty() {
        return write!(f, "{label}");
    }
    write!(f, "{label}: {detail}")
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrError::NotConfigured => write!(f, "score server not configured"),
            IrError::Network(detail) => write_labelled(f, "network error", detail),
            IrError::Unauthorized(detail) => write_labelled(f, "not signed in (401)", detail),
            IrError::Forbidden(detail) => write_labelled(f, "not allowed (403)", detail),
            IrError::NotFound(detail) => write_labelled(f, "not found (404)", detail),
            IrError::Conflict(detail) => write_labelled(f, "conflict (409)", detail),
            IrError::SettingsConflict(conflict) => match &conflict.server {
                Some(blob) => write!(f, "settings conflict (409): server copy '{}' updated_at={}", blob.name, blob.updated_at),
                None => write!(f, "settings conflict (409): server has no copy"),
            },
            IrError::PayloadTooLarge(detail) => write_labelled(f, "payload too large (413)", detail),
            IrError::RateLimited(detail) => write_labelled(f, "rate limited (429)", detail),
            IrError::Server(status, detail) => write_labelled(f, &format!("server error {status}"), detail),
            IrError::Decode(detail) => write_labelled(f, "decode error", detail),
            IrError::Unsupported => write!(f, "operation not supported by server"),
        }
    }
}

impl std::error::Error for IrError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::SettingsBlob;

    fn envelope_body(code: &str, message: &str) -> String {
        format!(r#"{{"success":false,"error":{{"code":"{code}","message":"{message}"}}}}"#)
    }

    #[test]
    fn display_not_configured() {
        assert_eq!(IrError::NotConfigured.to_string(), "score server not configured");
    }

    #[test]
    fn display_unsupported() {
        assert_eq!(IrError::Unsupported.to_string(), "operation not supported by server");
    }

    #[test]
    fn display_network_includes_detail() {
        assert_eq!(IrError::Network("timed out".into()).to_string(), "network error: timed out");
    }

    #[test]
    fn display_server_includes_code_and_body() {
        assert_eq!(IrError::Server(503, "down".into()).to_string(), "server error 503: down");
    }

    #[test]
    fn display_decode_includes_detail() {
        assert_eq!(IrError::Decode("eof".into()).to_string(), "decode error: eof");
    }

    #[test]
    fn display_omits_the_separator_when_the_body_is_empty() {
        assert_eq!(IrError::Unauthorized(String::new()).to_string(), "not signed in (401)");
        assert_eq!(IrError::Server(500, "   ".into()).to_string(), "server error 500");
    }

    #[test]
    fn display_unfolds_the_server_error_envelope() {
        let err = IrError::Conflict(envelope_body("IR_ACCOUNT_EXISTS", "이미 존재하는 계정입니다"));
        assert_eq!(err.to_string(), "conflict (409): 이미 존재하는 계정입니다 (IR_ACCOUNT_EXISTS)");
    }

    #[test]
    fn display_truncates_an_oversized_body() {
        let long = "x".repeat(ERROR_DETAIL_MAX_CHARS * 2);
        let rendered = IrError::Server(500, long).to_string();
        assert!(rendered.ends_with(TRUNCATION_SUFFIX), "got {rendered}");
        assert_eq!(rendered.chars().count(), "server error 500: ".len() + ERROR_DETAIL_MAX_CHARS + 1);
    }

    #[test]
    fn display_settings_conflict_names_the_server_copy() {
        let conflict = SettingsConflict { conflict: true, server: Some(SettingsBlob { name: "keyconfig".into(), updated_at: 42, ..Default::default() }) };
        assert_eq!(IrError::SettingsConflict(Box::new(conflict)).to_string(), "settings conflict (409): server copy 'keyconfig' updated_at=42");
    }

    #[test]
    fn display_settings_conflict_without_a_server_copy() {
        let conflict = SettingsConflict { conflict: true, server: None };
        assert_eq!(IrError::SettingsConflict(Box::new(conflict)).to_string(), "settings conflict (409): server has no copy");
    }

    #[test]
    fn from_status_maps_every_documented_status() {
        let cases = [
            (STATUS_UNAUTHORIZED, "u"),
            (STATUS_FORBIDDEN, "f"),
            (STATUS_NOT_FOUND, "n"),
            (STATUS_CONFLICT, "c"),
            (STATUS_PAYLOAD_TOO_LARGE, "p"),
            (STATUS_RATE_LIMITED, "r"),
        ];
        for (status, body) in cases {
            let err = IrError::from_status(status, body.into());
            assert_eq!(err.status(), Some(status), "got {err:?}");
            assert_eq!(err.detail(), body);
        }
        assert!(matches!(IrError::from_status(401, String::new()), IrError::Unauthorized(_)));
        assert!(matches!(IrError::from_status(403, String::new()), IrError::Forbidden(_)));
        assert!(matches!(IrError::from_status(404, String::new()), IrError::NotFound(_)));
        assert!(matches!(IrError::from_status(409, String::new()), IrError::Conflict(_)));
        assert!(matches!(IrError::from_status(413, String::new()), IrError::PayloadTooLarge(_)));
        assert!(matches!(IrError::from_status(429, String::new()), IrError::RateLimited(_)));
    }

    #[test]
    fn from_status_keeps_unmapped_statuses_in_the_server_variant() {
        let err = IrError::from_status(418, "teapot".into());
        assert!(matches!(err, IrError::Server(418, ref body) if body == "teapot"), "got {err:?}");
        assert!(matches!(IrError::from_status(400, "bad".into()), IrError::Server(400, _)));
        assert!(matches!(IrError::from_status(503, "down".into()), IrError::Server(503, _)));
    }

    #[test]
    fn status_is_absent_for_non_response_failures() {
        assert_eq!(IrError::NotConfigured.status(), None);
        assert_eq!(IrError::Unsupported.status(), None);
        assert_eq!(IrError::Network("x".into()).status(), None);
        assert_eq!(IrError::Decode("x".into()).status(), None);
    }

    #[test]
    fn error_code_reads_the_envelope_code() {
        let err = IrError::RateLimited(envelope_body("RATE_LIMITED", "too many requests"));
        assert_eq!(err.error_code().as_deref(), Some("RATE_LIMITED"));
    }

    #[test]
    fn error_code_is_absent_for_a_non_envelope_body() {
        assert_eq!(IrError::Server(500, "boom".into()).error_code(), None);
        assert_eq!(IrError::NotConfigured.error_code(), None);
    }

    #[test]
    fn auth_failures_are_distinguished_from_the_rest() {
        assert!(IrError::Unauthorized(String::new()).is_auth_failure());
        assert!(IrError::Forbidden(String::new()).is_auth_failure());
        assert!(!IrError::NotFound(String::new()).is_auth_failure());
        assert!(!IrError::RateLimited(String::new()).is_auth_failure());
    }

    #[test]
    fn retryable_covers_transport_rate_limit_and_5xx_only() {
        assert!(IrError::Network("reset".into()).is_retryable());
        assert!(IrError::RateLimited(String::new()).is_retryable());
        assert!(IrError::Server(500, String::new()).is_retryable());
        assert!(IrError::Server(503, String::new()).is_retryable());
        assert!(!IrError::Server(400, String::new()).is_retryable());
        assert!(!IrError::Unauthorized(String::new()).is_retryable());
        assert!(!IrError::Unsupported.is_retryable());
    }

    #[test]
    fn envelope_parse_rejects_a_plain_json_object() {
        assert!(ErrorEnvelope::parse("{\"error\":\"nope\"}").is_none());
        assert!(ErrorEnvelope::parse("not json").is_none());
    }

    #[test]
    fn envelope_parse_accepts_the_server_shape_with_details() {
        let body = r#"{"success":false,"error":{"code":"VALIDATION_ERROR","message":"invalid","details":{"field":"played_at"}}}"#;
        let envelope = ErrorEnvelope::parse(body).expect("server envelope parses");
        assert!(!envelope.success);
        assert_eq!(envelope.error.code, "VALIDATION_ERROR");
        assert_eq!(envelope.error.details.expect("details survive")["field"], "played_at");
    }

    #[test]
    fn ir_error_is_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let err = IrError::Unsupported;
        assert_error(&err);
        assert!(std::error::Error::source(&err).is_none());
    }
}
