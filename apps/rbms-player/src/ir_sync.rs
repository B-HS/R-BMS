//! Account-side settings sync: what goes into the synced blob, what stays on this machine, and how
//! a lost optimistic lock is reported.
//!
//! One blob carries both the play settings and the key config, so a fresh install restores the
//! whole configuration in a single round trip.

use rbms_ir::{IrError, SettingsBlob, SettingsPutResult};
use serde::{Deserialize, Serialize};

use rbms_config::{BUNDLED_SKIN_SCHEMA_VERSION, CURRENT_SCHEMA_VERSION, Config, LEGACY_SCHEMA_VERSION, LegacyV0, SINGLE_JUDGE_WIDTH_SCHEMA_VERSION};

use crate::keyconfig::KeyConfig;

/// Blob name the client stores its configuration under (`/players/{id}/settings/{name}`).
pub(crate) const SETTINGS_BLOB_NAME: &str = "player";

/// Free-form format tag stored with the blob; the server keeps it unread.
pub(crate) const SETTINGS_BLOB_FORMAT: &str = "ron";

/// Shown when the server rejected an upload because its copy moved on since the last download.
pub(crate) const SYNC_CONFLICT_MESSAGE: &str = "conflict: server newer, download first";

/// Everything the account syncs. Both halves carry a serde default, so a blob written by an older
/// or newer build still loads with the missing half falling back to this machine's defaults.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SyncPayload {
    pub(crate) settings: Config,
    pub(crate) keyconfig: KeyConfig,
}

/// Fields that describe *this machine or this session*, not the player's preferences, and are
/// therefore never uploaded and never overwritten by a download: the bearer token and the identity
/// it belongs to, the server it was issued by, and the local file paths — the song folders, the
/// difficulty tables and the skin pack folder among them, since all three are paths into this
/// machine's disk.
fn keep_local(local: &Config, mut incoming: Config) -> Config {
    incoming.network.ir_token = local.network.ir_token.clone();
    incoming.network.ir_login_id = local.network.ir_login_id.clone();
    incoming.network.ir_email = local.network.ir_email.clone();
    incoming.network.player_id = local.network.player_id.clone();
    incoming.network.server_url = local.network.server_url.clone();
    incoming.library.songs_folder = local.library.songs_folder.clone();
    incoming.library.folders = local.library.folders.clone();
    incoming.library.tables = local.library.tables.clone();
    incoming.display.font_path = local.display.font_path.clone();
    incoming.skin.pack = local.skin.pack.clone();
    incoming
}

/// The settings as they should leave this machine: play preferences only, with the credential and
/// the machine-local paths blanked so a token can never reach the server's blob store.
pub(crate) fn sanitise_for_upload(local: &Config) -> Config {
    keep_local(&Config::default(), local.clone())
}

/// Fold a downloaded copy into the local settings, keeping this machine's session and paths.
pub(crate) fn merge_downloaded(local: &Config, remote: Config) -> Config {
    keep_local(local, remote)
}

/// Serialise the payload into a blob ready for `put_settings`. `base_updated_at` is the
/// `updated_at` of the copy this edit was based on; `None` overwrites the server unconditionally,
/// which is what a first upload does.
pub(crate) fn build_blob(payload: &SyncPayload, updated_at: i64, base_updated_at: Option<i64>) -> Result<SettingsBlob, String> {
    let content = ron::ser::to_string_pretty(payload, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?;
    Ok(SettingsBlob { name: SETTINGS_BLOB_NAME.to_string(), content, updated_at, format: SETTINGS_BLOB_FORMAT.to_string(), base_updated_at })
}

/// The same payload as it was written before the settings became one versioned document. A blob an
/// account already holds is still in this shape, so a download after the upgrade restores the
/// player's preferences instead of resetting them.
#[derive(Default, Deserialize)]
#[serde(default)]
struct LegacySyncPayload {
    settings: LegacyV0,
    keyconfig: KeyConfig,
}

/// Reads only the schema version out of the settings half of a blob, the same way the
/// configuration loader probes a file before deciding how to parse it.
#[derive(Default, Deserialize)]
#[serde(default)]
struct BlobSchemaProbe {
    settings: SettingsSchemaProbe,
}

#[derive(Deserialize)]
#[serde(default)]
struct SettingsSchemaProbe {
    schema_version: u32,
}

/// Reads the one JUDGE WIDTH percentage a schema-1 blob held out of the settings half, so the same
/// migration the file loader runs can be applied to a blob. Without it serde would drop the field it
/// no longer knows and the download would silently reset the width to 100%.
#[derive(Default, Deserialize)]
#[serde(default)]
struct SingleJudgeWidthBlobProbe {
    settings: SingleJudgeWidthSettings,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct SingleJudgeWidthSettings {
    judge: SingleJudgeWidthGroup,
}

#[derive(Deserialize)]
#[serde(default)]
struct SingleJudgeWidthGroup {
    judge_rate: i32,
}

impl Default for SingleJudgeWidthGroup {
    fn default() -> Self {
        SingleJudgeWidthGroup { judge_rate: rbms_config::JUDGE_RATE_DEFAULT_PERCENT }
    }
}

impl Default for SettingsSchemaProbe {
    fn default() -> Self {
        SettingsSchemaProbe { schema_version: LEGACY_SCHEMA_VERSION }
    }
}

/// Reported for a blob a newer build wrote. The blob is left on the server and nothing local is
/// touched, the same way the configuration loader refuses a file from a newer schema.
pub(crate) const SETTINGS_BLOB_TOO_NEW: &str = "settings blob is from a newer build";

/// Read a blob fetched with `get_settings` back into a payload, running the same migrations the
/// configuration loader runs on a file: a blob written before the settings were versioned is
/// converted, one old JUDGE WIDTH is spread over every tier, and what a blob kept of the retired
/// skin bundle is dropped. A download is persisted on the current schema straight away, so a step
/// skipped here would never run again for that copy.
///
/// A blob that declares a schema past this build's is an error rather than a downgrade: parsing it
/// as the current schema would drop every field the newer build moved or added, and the caller
/// would then persist that stripped copy locally and upload it over the account's real settings.
pub(crate) fn parse_blob(blob: &SettingsBlob) -> Result<SyncPayload, String> {
    let probe: BlobSchemaProbe = ron::from_str(&blob.content).map_err(|e| e.to_string())?;
    let schema_version = probe.settings.schema_version;
    if schema_version > CURRENT_SCHEMA_VERSION {
        return Err(format!("{SETTINGS_BLOB_TOO_NEW} (schema version {schema_version})"));
    }
    let mut payload = if schema_version == LEGACY_SCHEMA_VERSION {
        let legacy: LegacySyncPayload = ron::from_str(&blob.content).map_err(|e| e.to_string())?;
        SyncPayload { settings: Config::from(legacy.settings), keyconfig: legacy.keyconfig }
    } else {
        ron::from_str::<SyncPayload>(&blob.content).map_err(|e| e.to_string())?
    };
    if schema_version == SINGLE_JUDGE_WIDTH_SCHEMA_VERSION {
        let single: SingleJudgeWidthBlobProbe = ron::from_str(&blob.content).map_err(|e| e.to_string())?;
        payload.settings.judge.spread_uniform_judge_rate(single.settings.judge.judge_rate);
    }
    if schema_version <= BUNDLED_SKIN_SCHEMA_VERSION {
        payload.settings.retire_bundled_skin();
    }
    payload.settings.sanitise();
    Ok(payload)
}

/// One finished settings-sync call, as far as the optimistic lock is concerned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SyncOutcome {
    /// A copy with this `updated_at` was read from the server (a download, or the read-back an
    /// older server still forces after an upload).
    Read(i64),
    /// `PUT` succeeded and reported the `updated_at` it stored. Only a stamp that came from the
    /// server may be carried here; a body-less 204 teaches nothing and must go through
    /// [`SyncOutcome::Read`] once the read-back lands.
    Uploaded(i64),
    /// `PUT` lost the lock (409).
    Conflict,
    /// The account this lock belonged to is gone.
    SignedOut,
}

/// The `base_updated_at` the next conditional upload will send.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct SyncLock {
    base_updated_at: Option<i64>,
}

impl SyncLock {
    /// The lock to send with the next upload; `None` overwrites the server unconditionally, which
    /// is what a first upload does.
    pub(crate) fn base(self) -> Option<i64> {
        self.base_updated_at
    }

    /// Move the lock in response to one finished call.
    ///
    /// A conflict deliberately does **not** adopt the server copy's stamp. Adopting it would make
    /// the very next upload succeed and overwrite the newer server settings the conflict message
    /// just told the user to download first; leaving the stale lock in place makes the retry
    /// conflict again until a real download resolves it.
    pub(crate) fn apply(&mut self, outcome: SyncOutcome) {
        match outcome {
            SyncOutcome::Read(updated_at) | SyncOutcome::Uploaded(updated_at) => self.base_updated_at = Some(updated_at),
            SyncOutcome::Conflict => {}
            SyncOutcome::SignedOut => self.base_updated_at = None,
        }
    }
}

/// How one finished `put_settings` moves the lock: `Some(outcome)` when the server reported the
/// stamp it stored, `None` when it answered `204 No Content` and the caller must read the row back
/// instead. The echo the client would otherwise adopt is a guess — the server writes its own clock
/// — and adopting a guess would let the next upload overwrite a copy it never saw.
pub(crate) fn upload_outcome(stored: SettingsPutResult) -> Option<SyncOutcome> {
    stored.from_server.then_some(SyncOutcome::Uploaded(stored.updated_at))
}

/// The server's copy carried by a lost optimistic lock, so the client can show or apply it without
/// a second round trip.
pub(crate) fn conflict_server_copy(error: &IrError) -> Option<&SettingsBlob> {
    match error {
        IrError::SettingsConflict(conflict) => conflict.server.as_ref(),
        _ => None,
    }
}

/// Status-line text for a failed sync call. A lost lock gets the fixed instruction; anything else
/// reports the error itself.
pub(crate) fn sync_error_message(error: &IrError) -> String {
    match error {
        IrError::SettingsConflict(_) => SYNC_CONFLICT_MESSAGE.to_string(),
        other => crate::ir_outcome::short_error(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_config::{JUDGE_WIDTH_TIER_COUNT, TableSource};
    use rbms_ir::SettingsConflict;
    use rbms_judge::GaugeKind;

    use crate::{SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT};

    fn local_settings() -> Config {
        let mut c = Config::default();
        c.play.hispeed = 4.5;
        c.play.gauge = GaugeKind::Hard;
        c.judge.judge_rate_key = [90; JUDGE_WIDTH_TIER_COUNT];
        c.network.ir_token = Some("secret-token".into());
        c.network.ir_login_id = Some("dj".into());
        c.network.ir_email = Some("dj@example.test".into());
        c.network.player_id = "dj".into();
        c.network.server_url = Some("https://ir.example/api".into());
        c.library.songs_folder = Some("/local/songs".into());
        c.library.folders = vec!["/local/songs".into()];
        c.library.tables = vec![TableSource { name: "mine".into(), location: "/local/table.json".into() }];
        c.display.font_path = Some("/local/font.ttf".into());
        c.skin.pack = Some("/local/skins/pack".into());
        c
    }

    fn payload_of(settings: Config) -> SyncPayload {
        SyncPayload { settings, keyconfig: KeyConfig::default() }
    }

    #[test]
    fn a_blob_round_trips_through_serialisation() {
        let payload = payload_of(local_settings());
        let blob = build_blob(&payload, 1_700, None).unwrap();
        assert_eq!(blob.name, SETTINGS_BLOB_NAME);
        assert_eq!(blob.format, SETTINGS_BLOB_FORMAT);
        assert_eq!(blob.updated_at, 1_700);
        assert_eq!(blob.base_updated_at, None, "a first upload overwrites unconditionally");

        let back = parse_blob(&blob).unwrap();
        assert!((back.settings.play.hispeed - 4.5).abs() < 1e-9);
        assert_eq!(back.settings.judge.judge_rate_key, [90; JUDGE_WIDTH_TIER_COUNT]);
        assert_eq!(back.keyconfig.lanes.len(), KeyConfig::default().lanes.len(), "the key config rides along");
    }

    #[test]
    fn an_upload_never_carries_the_credential_or_a_machine_path() {
        let sanitised = sanitise_for_upload(&local_settings());
        assert_eq!(sanitised.network.ir_token, None, "the bearer token never leaves the machine");
        assert_eq!(sanitised.network.ir_login_id, None);
        assert_eq!(sanitised.network.ir_email, None);
        assert_eq!(sanitised.network.server_url, None);
        assert_eq!(sanitised.library.songs_folder, None);
        assert!(sanitised.library.folders.is_empty(), "the song folders are this machine's paths");
        assert!(sanitised.library.tables.is_empty());
        assert_eq!(sanitised.display.font_path, None);
        assert_eq!(sanitised.skin.pack, None, "the skin pack is a folder on this machine's disk");
        assert_eq!(sanitised.network.player_id, Config::default().network.player_id);
        assert!((sanitised.play.hispeed - 4.5).abs() < 1e-9, "the play preferences do go up");
        assert_eq!(sanitised.judge.judge_rate_key, [90; JUDGE_WIDTH_TIER_COUNT]);

        let blob = build_blob(&payload_of(sanitised), 1, None).unwrap();
        assert!(!blob.content.contains("secret-token"), "the serialised blob cannot leak the token");
        assert!(!blob.content.contains("/local/songs"), "the serialised blob cannot leak a local path");
        assert!(!blob.content.contains("/local/table.json"));
        assert!(!blob.content.contains("/local/skins/pack"), "the serialised blob cannot leak the skin pack folder");
    }

    #[test]
    fn a_download_applies_preferences_but_keeps_this_machine_session_and_paths() {
        let local = local_settings();
        let mut remote = Config::default();
        remote.play.hispeed = 1.25;
        remote.play.gauge = GaugeKind::Easy;
        remote.network.ir_token = Some("someone-elses-token".into());
        remote.network.player_id = "otherdj".into();
        remote.network.server_url = Some("https://elsewhere/api".into());
        remote.library.songs_folder = Some("/their/songs".into());
        remote.library.folders = vec!["/their/songs".into()];
        remote.library.tables = vec![TableSource { name: "theirs".into(), location: "/their/table.json".into() }];
        remote.display.font_path = Some("/their/font.ttf".into());
        remote.skin.pack = Some("/their/skins/pack".into());

        let merged = merge_downloaded(&local, remote);
        assert!((merged.play.hispeed - 1.25).abs() < 1e-9, "preferences come from the server");
        assert_eq!(merged.play.gauge, GaugeKind::Easy);
        assert_eq!(merged.judge.judge_rate_key, Config::default().judge.judge_rate_key);
        assert_eq!(merged.network.ir_token.as_deref(), Some("secret-token"), "a downloaded token is discarded");
        assert_eq!(merged.network.player_id, "dj");
        assert_eq!(merged.network.server_url.as_deref(), Some("https://ir.example/api"));
        assert_eq!(merged.library.songs_folder.as_deref(), Some("/local/songs"));
        assert_eq!(merged.library.folders, vec!["/local/songs".to_string()], "a downloaded folder list does not replace this machine's");
        assert_eq!(merged.library.tables.len(), 1);
        assert_eq!(merged.library.tables[0].location, "/local/table.json");
        assert_eq!(merged.display.font_path.as_deref(), Some("/local/font.ttf"));
        assert_eq!(merged.skin.pack.as_deref(), Some("/local/skins/pack"), "a downloaded pack folder does not replace this machine's");
    }

    #[test]
    fn a_download_onto_a_machine_without_a_pack_does_not_bring_one() {
        let mut remote = Config::default();
        remote.skin.pack = Some("/their/skins/pack".into());
        let merged = merge_downloaded(&Config::default(), remote);
        assert_eq!(merged.skin.pack, None, "another machine's pack folder was adopted");
    }

    #[test]
    fn an_upload_after_a_download_carries_the_optimistic_lock() {
        let blob = build_blob(&payload_of(Config::default()), 2_000, Some(1_900)).unwrap();
        assert_eq!(blob.base_updated_at, Some(1_900));
        assert_eq!(blob.updated_at, 2_000);
    }

    #[test]
    fn a_lost_lock_reports_the_fixed_instruction_and_hands_back_the_server_copy() {
        let server = SettingsBlob {
            name: SETTINGS_BLOB_NAME.into(),
            content: ron::ser::to_string(&payload_of(Config { play: rbms_config::PlayOptions { hispeed: 7.0, ..Default::default() }, ..Config::default() }))
                .unwrap(),
            updated_at: 2_500,
            format: SETTINGS_BLOB_FORMAT.into(),
            base_updated_at: None,
        };
        let error = IrError::SettingsConflict(Box::new(SettingsConflict { conflict: true, server: Some(server) }));

        assert_eq!(sync_error_message(&error), SYNC_CONFLICT_MESSAGE);
        let copy = conflict_server_copy(&error).expect("the 409 body carries the server copy");
        assert_eq!(copy.updated_at, 2_500);
        let payload = parse_blob(copy).expect("the server copy parses without a second round trip");
        assert!((payload.settings.play.hispeed - 7.0).abs() < 1e-9);
    }

    #[test]
    fn a_conflict_without_a_body_still_reports_the_instruction() {
        let error = IrError::SettingsConflict(Box::<SettingsConflict>::default());
        assert_eq!(sync_error_message(&error), SYNC_CONFLICT_MESSAGE);
        assert!(conflict_server_copy(&error).is_none());
    }

    #[test]
    fn other_failures_report_themselves_rather_than_the_conflict_instruction() {
        let error = IrError::Unauthorized("sign in first".into());
        let message = sync_error_message(&error);
        assert_ne!(message, SYNC_CONFLICT_MESSAGE);
        assert!(message.contains("sign in first"), "{message}");
        assert!(conflict_server_copy(&error).is_none());
    }

    #[test]
    fn a_malformed_blob_is_an_error_not_a_panic() {
        let blob = SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: ">>> not ron <<<".into(), ..Default::default() };
        assert!(parse_blob(&blob).is_err());
    }

    #[test]
    fn a_blob_written_before_the_settings_were_versioned_still_restores_the_preferences() {
        let legacy = r#"(settings: (hispeed: 3.0, gauge: "hard", judge_rate: 120, preview: false, rivals: ["friend"]))"#;
        let blob = SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: legacy.into(), ..Default::default() };
        let payload = parse_blob(&blob).expect("a pre-schema blob is migrated, not discarded");
        assert!((payload.settings.play.hispeed - 3.0).abs() < 1e-9);
        assert_eq!(payload.settings.play.gauge, GaugeKind::Hard);
        assert_eq!(payload.settings.judge.judge_rate_key, [120; JUDGE_WIDTH_TIER_COUNT], "the one width the flat blob held covers every tier");
        assert!(!payload.settings.library.preview);
        assert_eq!(payload.settings.network.rivals, vec!["friend".to_string()]);
        assert_eq!(payload.settings.schema_version, rbms_config::CURRENT_SCHEMA_VERSION);
        assert_eq!(payload.keyconfig.lanes.len(), KeyConfig::default().lanes.len(), "the key config half still defaults");
    }

    #[test]
    fn a_blob_with_the_one_old_judge_width_spreads_it_over_every_tier() {
        let single = format!("(settings: (schema_version: {SINGLE_JUDGE_WIDTH_SCHEMA_VERSION}, play: (hispeed: 3.0), judge: (judge_rate: 80)))");
        let blob = SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: single, ..Default::default() };
        let payload = parse_blob(&blob).expect("a schema-1 blob is migrated, not read as the current schema");
        assert_eq!(payload.settings.judge.judge_rate_key, [80; JUDGE_WIDTH_TIER_COUNT], "downloading must not reset the width the account holds");
        assert_eq!(payload.settings.judge.judge_rate_scratch, [80; JUDGE_WIDTH_TIER_COUNT]);
        assert!((payload.settings.play.hispeed - 3.0).abs() < 1e-9, "the rest of the blob still loads");
    }

    #[test]
    fn a_blob_at_the_current_schema_keeps_its_own_per_tier_widths() {
        let current = format!("(settings: (schema_version: {CURRENT_SCHEMA_VERSION}, judge: (judge_rate_key: (95, 90, 85))))");
        let blob = SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: current, ..Default::default() };
        let payload = parse_blob(&blob).expect("a current blob needs no migration");
        assert_eq!(payload.settings.judge.judge_rate_key, [95, 90, 85]);
    }

    /// The DISPLAY preset the retired skin bundle was switched on with, as the builds that offered
    /// it uploaded it.
    const RETIRED_BUNDLE_PRESET: &str = "STEEL NEON";

    /// A document of the retired bundle's last generation, below the skin folder a configuration
    /// that names none looks in.
    const RETIRED_BUNDLE_DOCUMENT: &str = "/home/p/.config/rbms/skin/steel-neon-v3/select.json5";

    /// A document of the retired bundle no screen was drawn with, which still had choices stored.
    const RETIRED_BUNDLE_UNSELECTED_DOCUMENT: &str = "/home/p/.config/rbms/skin/steel-neon/decide.json5";

    /// A document the player keeps outside every skin folder.
    const PLAYER_DOCUMENT: &str = "/elsewhere/mine/result.json5";

    /// A customisation property the fixtures store for a document, and the option id chosen for it.
    const STORED_PROPERTY: &str = "BACKGROUND";
    const STORED_PROPERTY_OPTION: i32 = 911;

    /// The settings half of a blob a build that still shipped the bundle uploaded: the bundle's
    /// preset, one screen drawn with a bundle document, one with the player's own, and choices
    /// stored for all three documents.
    fn bundled_skin_settings(schema_version: u32) -> String {
        format!(
            r#"(
                schema_version: {schema_version},
                play: (hispeed: 3.5),
                display: (skin: "{RETIRED_BUNDLE_PRESET}"),
                skin: (
                    selected: {{ {SKIN_TYPE_MUSIC_SELECT}: "{RETIRED_BUNDLE_DOCUMENT}", {SKIN_TYPE_RESULT}: "{PLAYER_DOCUMENT}" }},
                    custom: {{
                        "{RETIRED_BUNDLE_DOCUMENT}": (properties: {{ "{STORED_PROPERTY}": {STORED_PROPERTY_OPTION} }}),
                        "{RETIRED_BUNDLE_UNSELECTED_DOCUMENT}": (properties: {{ "{STORED_PROPERTY}": {STORED_PROPERTY_OPTION} }}),
                        "{PLAYER_DOCUMENT}": (properties: {{ "{STORED_PROPERTY}": {STORED_PROPERTY_OPTION} }}),
                    }},
                    shared: {{ "steel-neon-v3": (properties: {{ "PLAY SIDE": 901 }}) }},
                    default_skin_installed: true,
                ),
            )"#
        )
    }

    fn blob_of_settings(settings: &str) -> SettingsBlob {
        SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: format!("(settings: {settings})"), ..Default::default() }
    }

    #[test]
    fn a_blob_from_a_build_that_shipped_the_skin_bundle_comes_down_without_it() {
        let blob = blob_of_settings(&bundled_skin_settings(BUNDLED_SKIN_SCHEMA_VERSION));
        let payload = parse_blob(&blob).expect("a schema 2 blob is migrated, not read as the current schema");
        let merged = merge_downloaded(&local_settings(), payload.settings);

        assert_eq!(merged.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(merged.display.skin, rbms_config::DEFAULT_SKIN, "the download brought the retired preset back");
        assert!((merged.play.hispeed - 3.5).abs() < 1e-9, "the rest of the blob still loads");
        assert_eq!(merged.skin.document(SKIN_TYPE_MUSIC_SELECT), None, "a screen is drawn alone by a document written to be layered");
        assert_eq!(merged.skin.customisation(RETIRED_BUNDLE_DOCUMENT), None, "choices were kept for a document of the retired bundle");
        assert_eq!(merged.skin.customisation(RETIRED_BUNDLE_UNSELECTED_DOCUMENT), None, "choices were kept for a retired document no screen used");
        assert_eq!(merged.skin.document(SKIN_TYPE_RESULT), Some(PLAYER_DOCUMENT), "the player's own document was dropped");
        assert_eq!(merged.skin.user_config(PLAYER_DOCUMENT).properties.get(STORED_PROPERTY), Some(&STORED_PROPERTY_OPTION));

        let stored = build_blob(&payload_of(sanitise_for_upload(&merged)), 1, None).unwrap();
        let again = parse_blob(&stored).expect("the copy this build uploads parses").settings;
        assert_eq!(again.display.skin, merged.display.skin);
        let uploaded = rbms_config::SkinOptions { pack: None, ..merged.skin.clone() };
        assert_eq!(again.skin, uploaded, "the next upload carries the retired bundle back to the account, or this machine's pack folder with it");
    }

    #[test]
    fn the_retired_preset_is_dropped_from_a_blob_of_every_older_schema() {
        for version in [SINGLE_JUDGE_WIDTH_SCHEMA_VERSION, BUNDLED_SKIN_SCHEMA_VERSION] {
            let payload = parse_blob(&blob_of_settings(&bundled_skin_settings(version))).expect("an older blob is migrated");
            assert_eq!(payload.settings.display.skin, rbms_config::DEFAULT_SKIN, "schema {version} kept the retired preset");
            assert_eq!(payload.settings.skin.document(SKIN_TYPE_MUSIC_SELECT), None, "schema {version} kept a retired document");
        }
        let flat = blob_of_settings(&format!(r#"(hispeed: 3.0, skin: "{RETIRED_BUNDLE_PRESET}")"#));
        let payload = parse_blob(&flat).expect("a pre-schema blob is migrated");
        assert_eq!(payload.settings.display.skin, rbms_config::DEFAULT_SKIN, "the flat schema kept the retired preset");
        assert!((payload.settings.play.hispeed - 3.0).abs() < 1e-9, "the rest of the flat blob still loads");
    }

    #[test]
    fn a_blob_at_the_current_schema_keeps_the_documents_it_selects() {
        let blob = blob_of_settings(&bundled_skin_settings(CURRENT_SCHEMA_VERSION));
        let payload = parse_blob(&blob).expect("a current blob needs no migration");
        assert_eq!(payload.settings.skin.document(SKIN_TYPE_MUSIC_SELECT), Some(RETIRED_BUNDLE_DOCUMENT), "the retirement ran on a blob this build wrote");
        assert_eq!(payload.settings.skin.document(SKIN_TYPE_RESULT), Some(PLAYER_DOCUMENT));
    }

    #[test]
    fn a_blob_from_a_newer_build_is_refused_rather_than_read_as_this_schema() {
        let newer = format!("(settings: (schema_version: {}, play: (hispeed: 6.0)))", CURRENT_SCHEMA_VERSION + 1);
        let blob = SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: newer, ..Default::default() };
        let error = match parse_blob(&blob) {
            Ok(_) => panic!("reading it as this schema would strip every field the newer build added"),
            Err(e) => e,
        };
        assert!(error.contains(SETTINGS_BLOB_TOO_NEW), "{error}");
        assert!(error.contains(&(CURRENT_SCHEMA_VERSION + 1).to_string()), "the refusal names the schema it saw: {error}");
    }

    #[test]
    fn a_blob_missing_the_keyconfig_half_still_loads() {
        let blob =
            SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: "(settings: (schema_version: 1, play: (hispeed: 6.0)))".into(), ..Default::default() };
        let payload = parse_blob(&blob).expect("serde defaults fill the missing half");
        assert!((payload.settings.play.hispeed - 6.0).abs() < 1e-9);
        assert_eq!(payload.keyconfig.lanes.len(), KeyConfig::default().lanes.len());
    }

    fn conflict_error(server_updated_at: i64) -> IrError {
        let server = SettingsBlob {
            name: SETTINGS_BLOB_NAME.into(),
            content: ron::ser::to_string(&payload_of(Config::default())).unwrap(),
            updated_at: server_updated_at,
            format: SETTINGS_BLOB_FORMAT.into(),
            base_updated_at: None,
        };
        IrError::SettingsConflict(Box::new(SettingsConflict { conflict: true, server: Some(server) }))
    }

    #[test]
    fn a_fresh_lock_uploads_unconditionally() {
        assert_eq!(SyncLock::default().base(), None);
    }

    #[test]
    fn a_read_copy_sets_the_lock_and_a_later_read_replaces_it() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        assert_eq!(lock.base(), Some(1_000));
        lock.apply(SyncOutcome::Read(2_000));
        assert_eq!(lock.base(), Some(2_000), "a later read replaces it");
    }

    #[test]
    fn an_upload_that_learned_the_server_stamp_moves_the_lock_without_a_read_back() {
        let outcome = upload_outcome(SettingsPutResult { updated_at: 2_200, from_server: true });
        assert_eq!(outcome, Some(SyncOutcome::Uploaded(2_200)));
    }

    #[test]
    fn a_body_less_upload_leaves_the_lock_to_the_read_back() {
        let outcome = upload_outcome(SettingsPutResult { updated_at: 2_200, from_server: false });
        assert_eq!(outcome, None, "the echoed stamp is a guess; adopting it could clobber the server copy");
    }

    #[test]
    fn an_upload_adopts_the_stamp_the_server_reported() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        lock.apply(SyncOutcome::Uploaded(1_200));
        assert_eq!(lock.base(), Some(1_200), "the PUT body carries the stored stamp, so no read-back is needed");

        let next = build_blob(&payload_of(Config::default()), 1_300, lock.base()).unwrap();
        assert_eq!(next.base_updated_at, Some(1_200), "the second save of a session no longer conflicts");
    }

    #[test]
    fn a_204_upload_teaches_the_lock_nothing_until_the_read_back_lands() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        if let Some(outcome) = upload_outcome(SettingsPutResult { updated_at: 1_400, from_server: false }) {
            lock.apply(outcome);
        }
        assert_eq!(lock.base(), Some(1_000), "an older server ignored the client stamp, so the old base is all we know");
        lock.apply(SyncOutcome::Read(1_500));
        assert_eq!(lock.base(), Some(1_500), "the read-back is what refreshes it");
    }

    #[test]
    fn a_conflict_never_advances_the_lock_to_the_server_copy() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        let error = conflict_error(2_500);
        assert_eq!(conflict_server_copy(&error).map(|copy| copy.updated_at), Some(2_500));

        lock.apply(SyncOutcome::Conflict);
        assert_eq!(lock.base(), Some(1_000), "adopting 2500 would let the next upload clobber the newer server copy");

        let retry = build_blob(&payload_of(Config::default()), 3_000, lock.base()).unwrap();
        assert_eq!(retry.base_updated_at, Some(1_000), "the retry conflicts again instead of overwriting");
    }

    #[test]
    fn signing_out_clears_the_lock_so_the_next_account_starts_clean() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        lock.apply(SyncOutcome::SignedOut);
        assert_eq!(lock.base(), None);
    }
}
