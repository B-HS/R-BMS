use serde::{Deserialize, Serialize};

/// What the server advertises on `/health`. Every field defaults to `false`, so a server that
/// omits the block, or a single flag, simply reads as "not supported".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerCapabilities {
    pub ranking: bool,
    pub player_best: bool,
    pub rivals: bool,
    pub courses: bool,
    pub replays: bool,
    pub tables: bool,
    /// Account-side settings blob sync (`/players/{id}/settings/{name}`).
    pub settings_sync: bool,
    /// Account creation and login over the IR itself (`/auth/register`, `/auth/login`).
    pub accounts: bool,
    /// The server also speaks the legacy LR2IR wire format.
    pub lr2ir_compat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub ir_compat: String,
    #[serde(default)]
    pub capabilities: ServerCapabilities,
}

/// The contract version and build a deployment answers with on `/version`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub api_version: u32,
    pub server: String,
    pub commit: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::dto::*;
    use serde_json::json;

    #[test]
    fn server_capabilities_default_is_all_false() {
        let c = ServerCapabilities::default();
        assert!(!c.ranking);
        assert!(!c.player_best);
        assert!(!c.rivals);
        assert!(!c.courses);
        assert!(!c.replays);
        assert!(!c.tables);
    }

    #[test]
    fn server_info_round_trips_with_capabilities() {
        let info = ServerInfo {
            name: "rbms-test".into(),
            version: "1.2.3".into(),
            ir_compat: "lr2".into(),
            capabilities: ServerCapabilities { ranking: true, player_best: true, courses: true, tables: true, ..Default::default() },
        };
        let back: ServerInfo = serde_json::from_str(&serde_json::to_string(&info).unwrap()).unwrap();
        assert_eq!(back.name, "rbms-test");
        assert!(back.capabilities.ranking);
        assert!(!back.capabilities.rivals);
        assert!(back.capabilities.tables);
    }

    #[test]
    fn server_info_missing_capabilities_defaults_to_all_false() {
        let j = json!({"name": "n", "version": "v", "ir_compat": "c"});
        let info: ServerInfo = serde_json::from_value(j).unwrap();
        assert!(!info.capabilities.ranking);
        assert!(!info.capabilities.tables);
    }

    #[test]
    fn server_capabilities_decode_the_full_server_block() {
        let j = json!({
            "ranking": true, "player_best": true, "rivals": true, "courses": true, "replays": true,
            "tables": true, "settings_sync": true, "accounts": true, "lr2ir_compat": false
        });
        let c: ServerCapabilities = serde_json::from_value(j).unwrap();
        assert!(c.settings_sync && c.accounts && c.replays);
        assert!(!c.lr2ir_compat);
    }

    #[test]
    fn server_capabilities_default_every_missing_flag() {
        let c: ServerCapabilities = serde_json::from_value(json!({})).unwrap();
        assert!(!c.ranking && !c.settings_sync && !c.accounts && !c.lr2ir_compat);
    }

    #[test]
    fn version_info_round_trips_with_and_without_a_commit() {
        let v: VersionInfo = serde_json::from_value(json!({"api_version": 1, "server": "rbms-web", "commit": "abc1234"})).unwrap();
        assert_eq!(v.api_version, crate::API_VERSION);
        assert_eq!(v.commit.as_deref(), Some("abc1234"));
        let headless: VersionInfo = serde_json::from_value(json!({"api_version": 1, "server": "rbms-web", "commit": null})).unwrap();
        assert!(headless.commit.is_none());
    }
}
