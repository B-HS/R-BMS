use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A named settings blob (settings/keyconfig/tables …) for account-side sync. `content` is the raw
/// serialised body (RON/JSON) the client wrote; the server stores it opaquely per `(player, name)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsBlob {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub updated_at: i64,
    /// Free-form tag describing `content` (`ron`, `json`, …); the server stores it unread.
    #[serde(default)]
    pub format: String,
    /// Optimistic lock: the `updated_at` this edit was based on. When set, the server rejects the
    /// write with a conflict if its copy has moved on. `None` overwrites unconditionally. Never
    /// sent back by the server, so it stays `None` on a fetched blob.
    #[serde(default)]
    pub base_updated_at: Option<i64>,
}

/// Body of `PUT /players/{id}/settings/{name}`, built from a [`SettingsBlob`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsPutRequest {
    pub api_version: u32,
    pub name: Option<String>,
    pub format: String,
    pub content: String,
    pub updated_at: Option<i64>,
    pub base_updated_at: Option<i64>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl SettingsPutRequest {
    /// Wire body for storing `blob`, carrying its optimistic-lock base when it has one.
    pub fn from_blob(blob: &SettingsBlob) -> SettingsPutRequest {
        SettingsPutRequest {
            api_version: crate::API_VERSION,
            name: Some(blob.name.clone()),
            format: blob.format.clone(),
            content: blob.content.clone(),
            updated_at: Some(blob.updated_at),
            base_updated_at: blob.base_updated_at,
            extra: HashMap::new(),
        }
    }
}

/// The 409 body of a settings write that lost the optimistic lock: the server's current copy, so
/// the client can merge or overwrite without a second round trip.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsConflict {
    #[serde(default)]
    pub conflict: bool,
    #[serde(default)]
    pub server: Option<SettingsBlob>,
}

#[cfg(test)]
mod tests {
    use crate::dto::*;
    use serde_json::json;

    #[test]
    fn settings_blob_round_trips() {
        let b = SettingsBlob { name: "keyconfig".into(), content: "(raw ron)".into(), updated_at: 1700, ..Default::default() };
        let back: SettingsBlob = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back.name, "keyconfig");
        assert_eq!(back.content, "(raw ron)");
        assert_eq!(back.updated_at, 1700);
    }

    #[test]
    fn settings_blob_missing_updated_at_defaults_zero() {
        let j = json!({"name": "tables", "content": "[]"});
        let b: SettingsBlob = serde_json::from_value(j).unwrap();
        assert_eq!(b.updated_at, 0);
    }

    #[test]
    fn settings_blob_opaque_content_preserved_verbatim() {
        let raw = "{\"a\":1,\"nested\":{\"b\":[1,2,3]},\"unicode\":\"日本語\"}";
        let b = SettingsBlob { name: "settings".into(), content: raw.into(), ..Default::default() };
        let back: SettingsBlob = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back.content, raw);
    }

    #[test]
    fn settings_blob_decodes_the_server_response_shape() {
        let j = json!({"name": "keyconfig", "format": "ron", "content": "(k:1)", "updated_at": 1700});
        let b: SettingsBlob = serde_json::from_value(j).unwrap();
        assert_eq!(b.format, "ron");
        assert_eq!(b.updated_at, 1700);
        assert!(b.base_updated_at.is_none(), "the response never carries the lock base");
    }

    #[test]
    fn settings_put_request_mirrors_the_put_schema() {
        let blob = SettingsBlob { name: "keyconfig".into(), content: "(k:1)".into(), updated_at: 1700, format: "ron".into(), base_updated_at: Some(1600) };
        let req = SettingsPutRequest::from_blob(&blob);
        assert_eq!(req.api_version, crate::API_VERSION);
        assert_eq!(req.name.as_deref(), Some("keyconfig"));
        assert_eq!(req.format, "ron");
        assert_eq!(req.updated_at, Some(1700));
        assert_eq!(req.base_updated_at, Some(1600));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""base_updated_at":1600"#), "got {json}");
        assert!(json.contains(r#""extra":{}"#));
    }

    #[test]
    fn settings_put_request_without_a_lock_base_serialises_null() {
        let req = SettingsPutRequest::from_blob(&SettingsBlob { name: "settings".into(), ..Default::default() });
        assert!(req.base_updated_at.is_none());
        assert!(serde_json::to_string(&req).unwrap().contains(r#""base_updated_at":null"#));
    }

    #[test]
    fn settings_conflict_decodes_the_409_body() {
        let j = json!({"conflict": true, "server": {"name": "keyconfig", "format": "ron", "content": "(k:2)", "updated_at": 1800}});
        let c: SettingsConflict = serde_json::from_value(j).unwrap();
        assert!(c.conflict);
        let remote = c.server.expect("server copy present");
        assert_eq!(remote.content, "(k:2)");
        assert_eq!(remote.updated_at, 1800);
    }

    #[test]
    fn settings_conflict_accepts_a_missing_server_copy() {
        let c: SettingsConflict = serde_json::from_value(json!({"conflict": true, "server": null})).unwrap();
        assert!(c.server.is_none());
    }
}
