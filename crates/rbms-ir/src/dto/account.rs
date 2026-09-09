use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dto::PlayerId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub total_plays: u64,
    pub rank_points: f64,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Body of `PUT /players/{id}/rivals`: the complete rival list, by player id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RivalPutRequest {
    pub rivals: Vec<String>,
}

/// Register/login request. Superset of the reference implementation's `IRAccount{id,password,name}` with an optional
/// `email`. `name` is used on register; ignored on login. The server requires an email to register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    #[serde(default = "crate::dto::current_api_version")]
    pub api_version: u32,
    pub id: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

impl Default for AuthRequest {
    fn default() -> AuthRequest {
        AuthRequest { api_version: crate::API_VERSION, id: String::new(), password: String::new(), email: None, name: None }
    }
}

impl AuthRequest {
    /// Account creation: id, password and email are all required by the server; `name` defaults to
    /// the id when omitted.
    pub fn register(id: impl Into<String>, password: impl Into<String>, email: impl Into<String>, name: Option<String>) -> AuthRequest {
        AuthRequest { id: id.into(), password: password.into(), email: Some(email.into()), name, ..AuthRequest::default() }
    }

    /// Sign-in: only the id and password are read.
    pub fn login(id: impl Into<String>, password: impl Into<String>) -> AuthRequest {
        AuthRequest { id: id.into(), password: password.into(), ..AuthRequest::default() }
    }
}

/// Successful auth: a bearer token plus the resolved player identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub player: PlayerId,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use crate::dto::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn player_profile_round_trips() {
        let p = PlayerProfile { id: "p".into(), name: "Carol".into(), total_plays: 9999, rank_points: 12.5, extra: HashMap::new() };
        let back: PlayerProfile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.total_plays, 9999);
        assert!((back.rank_points - 12.5).abs() < 1e-12);
        assert!(back.extra.is_empty());
    }

    #[test]
    fn player_profile_decodes_without_extra() {
        let j = json!({"id": "p", "name": "D", "total_plays": 0, "rank_points": 0.0});
        let p: PlayerProfile = serde_json::from_value(j).unwrap();
        assert!(p.extra.is_empty());
    }

    #[test]
    fn auth_request_full_round_trips() {
        let r = AuthRequest::register("user", "secret", "u@e.com", Some("Nick".into()));
        let back: AuthRequest = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.id, "user");
        assert_eq!(back.email.as_deref(), Some("u@e.com"));
        assert_eq!(back.name.as_deref(), Some("Nick"));
    }

    #[test]
    fn auth_request_minimal_defaults_email_and_name_none() {
        let j = json!({"id": "user", "password": "pw"});
        let r: AuthRequest = serde_json::from_value(j).unwrap();
        assert_eq!(r.email, None);
        assert_eq!(r.name, None);
    }

    #[test]
    fn auth_request_missing_password_fails() {
        let j = json!({"id": "user"});
        assert!(serde_json::from_value::<AuthRequest>(j).is_err());
    }

    #[test]
    fn auth_response_round_trips() {
        let r = AuthResponse { token: "bearer-abc".into(), player: PlayerId { id: "p42".into() }, name: "Eve".into() };
        let back: AuthResponse = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back.token, "bearer-abc");
        assert_eq!(back.player.id, "p42");
        assert_eq!(back.name, "Eve");
    }

    #[test]
    fn serialization_is_deterministic_for_structs_without_maps() {
        let r = AuthResponse { token: "t".into(), player: PlayerId { id: "p".into() }, name: "n".into() };
        let a = serde_json::to_string(&r).unwrap();
        let b = serde_json::to_string(&r).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rival_put_request_serialises_the_id_list() {
        let body = RivalPutRequest { rivals: vec!["a".into(), "b".into()] };
        assert_eq!(serde_json::to_string(&body).unwrap(), r#"{"rivals":["a","b"]}"#);
        assert_eq!(serde_json::to_string(&RivalPutRequest::default()).unwrap(), r#"{"rivals":[]}"#);
    }

    #[test]
    fn auth_request_register_fills_the_account_schema() {
        let r = AuthRequest::register("gkn", "hunter2hunter2", "gkn@example.com", Some("gkn".into()));
        assert_eq!(r.api_version, crate::API_VERSION);
        assert_eq!(r.email.as_deref(), Some("gkn@example.com"));
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""api_version":1"#), "got {json}");
        assert!(json.contains(r#""password":"hunter2hunter2""#));
    }

    #[test]
    fn auth_request_login_leaves_the_optional_fields_null() {
        let r = AuthRequest::login("gkn", "pw");
        assert!(r.email.is_none() && r.name.is_none());
        assert_eq!(r.api_version, crate::API_VERSION);
    }

    #[test]
    fn auth_request_without_an_api_version_decodes_to_the_current_one() {
        let r: AuthRequest = serde_json::from_value(json!({"id": "gkn", "password": "pw"})).unwrap();
        assert_eq!(r.api_version, crate::API_VERSION);
    }

    #[test]
    fn auth_response_decodes_the_server_body() {
        let r: AuthResponse = serde_json::from_value(json!({"token": "t", "player": {"id": "gkn"}, "name": "gkn"})).unwrap();
        assert_eq!(r.token, "t");
        assert_eq!(r.player.id, "gkn");
        assert_eq!(r.name, "gkn");
    }
}
