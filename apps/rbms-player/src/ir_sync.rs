//! Account-side settings sync: what goes into the synced blob, what stays on this machine, and how
//! a lost optimistic lock is reported.
//!
//! One blob carries both the play settings and the key config, so a fresh install restores the
//! whole configuration in a single round trip.

use rbms_ir::{IrError, SettingsBlob};
use serde::{Deserialize, Serialize};

use crate::keyconfig::KeyConfig;
use crate::settings::PlaySettings;

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
    pub(crate) settings: PlaySettings,
    pub(crate) keyconfig: KeyConfig,
}

/// Fields that describe *this machine or this session*, not the player's preferences, and are
/// therefore never uploaded and never overwritten by a download: the bearer token and the identity
/// it belongs to, the server it was issued by, and the local file paths.
fn keep_local(local: &PlaySettings, mut incoming: PlaySettings) -> PlaySettings {
    incoming.ir_token = local.ir_token.clone();
    incoming.ir_login_id = local.ir_login_id.clone();
    incoming.ir_email = local.ir_email.clone();
    incoming.player_id = local.player_id.clone();
    incoming.server_url = local.server_url.clone();
    incoming.songs_folder = local.songs_folder.clone();
    incoming.font_path = local.font_path.clone();
    incoming
}

/// The settings as they should leave this machine: play preferences only, with the credential and
/// the machine-local paths blanked so a token can never reach the server's blob store.
pub(crate) fn sanitise_for_upload(local: &PlaySettings) -> PlaySettings {
    keep_local(&PlaySettings::default(), local.clone())
}

/// Fold a downloaded copy into the local settings, keeping this machine's session and paths.
pub(crate) fn merge_downloaded(local: &PlaySettings, remote: PlaySettings) -> PlaySettings {
    keep_local(local, remote)
}

/// Serialise the payload into a blob ready for `put_settings`. `base_updated_at` is the
/// `updated_at` of the copy this edit was based on; `None` overwrites the server unconditionally,
/// which is what a first upload does.
pub(crate) fn build_blob(payload: &SyncPayload, updated_at: i64, base_updated_at: Option<i64>) -> Result<SettingsBlob, String> {
    let content = ron::ser::to_string_pretty(payload, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?;
    Ok(SettingsBlob { name: SETTINGS_BLOB_NAME.to_string(), content, updated_at, format: SETTINGS_BLOB_FORMAT.to_string(), base_updated_at })
}

/// Read a blob fetched with `get_settings` back into a payload.
pub(crate) fn parse_blob(blob: &SettingsBlob) -> Result<SyncPayload, String> {
    ron::from_str(&blob.content).map_err(|e| e.to_string())
}

/// One finished settings-sync call, as far as the optimistic lock is concerned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SyncOutcome {
    /// A copy with this `updated_at` was read from the server (a download, or the read-back that
    /// follows an upload). This is the only thing that may set the lock.
    Read(i64),
    /// `PUT` returned 204. It carries no body and the server ignores the stamp the client sent, so
    /// nothing is learned here: the lock only moves once the read-back lands.
    Uploaded,
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
            SyncOutcome::Read(updated_at) => self.base_updated_at = Some(updated_at),
            SyncOutcome::Uploaded | SyncOutcome::Conflict => {}
            SyncOutcome::SignedOut => self.base_updated_at = None,
        }
    }
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
    use rbms_ir::SettingsConflict;

    fn local_settings() -> PlaySettings {
        PlaySettings {
            hispeed: 4.5,
            gauge: "hard".into(),
            judge_rate: 90,
            ir_token: Some("secret-token".into()),
            ir_login_id: Some("dj".into()),
            ir_email: Some("dj@example.test".into()),
            player_id: "dj".into(),
            server_url: Some("https://ir.example/api".into()),
            songs_folder: Some("/local/songs".into()),
            font_path: Some("/local/font.ttf".into()),
            ..PlaySettings::default()
        }
    }

    fn payload_of(settings: PlaySettings) -> SyncPayload {
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
        assert!((back.settings.hispeed - 4.5).abs() < 1e-9);
        assert_eq!(back.settings.judge_rate, 90);
        assert_eq!(back.keyconfig.lanes.len(), KeyConfig::default().lanes.len(), "the key config rides along");
    }

    #[test]
    fn an_upload_never_carries_the_credential_or_a_machine_path() {
        let sanitised = sanitise_for_upload(&local_settings());
        assert_eq!(sanitised.ir_token, None, "the bearer token never leaves the machine");
        assert_eq!(sanitised.ir_login_id, None);
        assert_eq!(sanitised.ir_email, None);
        assert_eq!(sanitised.server_url, None);
        assert_eq!(sanitised.songs_folder, None);
        assert_eq!(sanitised.font_path, None);
        assert_eq!(sanitised.player_id, PlaySettings::default().player_id);
        assert!((sanitised.hispeed - 4.5).abs() < 1e-9, "the play preferences do go up");
        assert_eq!(sanitised.judge_rate, 90);

        let blob = build_blob(&payload_of(sanitised), 1, None).unwrap();
        assert!(!blob.content.contains("secret-token"), "the serialised blob cannot leak the token");
    }

    #[test]
    fn a_download_applies_preferences_but_keeps_this_machine_session_and_paths() {
        let local = local_settings();
        let remote = PlaySettings {
            hispeed: 1.25,
            gauge: "easy".into(),
            judge_rate: 100,
            ir_token: Some("someone-elses-token".into()),
            player_id: "otherdj".into(),
            server_url: Some("https://elsewhere/api".into()),
            songs_folder: Some("/their/songs".into()),
            font_path: Some("/their/font.ttf".into()),
            ..PlaySettings::default()
        };
        let merged = merge_downloaded(&local, remote);
        assert!((merged.hispeed - 1.25).abs() < 1e-9, "preferences come from the server");
        assert_eq!(merged.gauge, "easy");
        assert_eq!(merged.ir_token.as_deref(), Some("secret-token"), "a downloaded token is discarded");
        assert_eq!(merged.player_id, "dj");
        assert_eq!(merged.server_url.as_deref(), Some("https://ir.example/api"));
        assert_eq!(merged.songs_folder.as_deref(), Some("/local/songs"));
        assert_eq!(merged.font_path.as_deref(), Some("/local/font.ttf"));
    }

    #[test]
    fn an_upload_after_a_download_carries_the_optimistic_lock() {
        let blob = build_blob(&payload_of(PlaySettings::default()), 2_000, Some(1_900)).unwrap();
        assert_eq!(blob.base_updated_at, Some(1_900));
        assert_eq!(blob.updated_at, 2_000);
    }

    #[test]
    fn a_lost_lock_reports_the_fixed_instruction_and_hands_back_the_server_copy() {
        let server = SettingsBlob {
            name: SETTINGS_BLOB_NAME.into(),
            content: ron::ser::to_string(&payload_of(PlaySettings { hispeed: 7.0, ..PlaySettings::default() })).unwrap(),
            updated_at: 2_500,
            format: SETTINGS_BLOB_FORMAT.into(),
            base_updated_at: None,
        };
        let error = IrError::SettingsConflict(Box::new(SettingsConflict { conflict: true, server: Some(server) }));

        assert_eq!(sync_error_message(&error), SYNC_CONFLICT_MESSAGE);
        let copy = conflict_server_copy(&error).expect("the 409 body carries the server copy");
        assert_eq!(copy.updated_at, 2_500);
        let payload = parse_blob(copy).expect("the server copy parses without a second round trip");
        assert!((payload.settings.hispeed - 7.0).abs() < 1e-9);
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
    fn a_blob_missing_the_keyconfig_half_still_loads() {
        let blob = SettingsBlob { name: SETTINGS_BLOB_NAME.into(), content: "(settings: (hispeed: 6.0))".into(), ..Default::default() };
        let payload = parse_blob(&blob).expect("serde defaults fill the missing half");
        assert!((payload.settings.hispeed - 6.0).abs() < 1e-9);
        assert_eq!(payload.keyconfig.lanes.len(), KeyConfig::default().lanes.len());
    }

    fn conflict_error(server_updated_at: i64) -> IrError {
        let server = SettingsBlob {
            name: SETTINGS_BLOB_NAME.into(),
            content: ron::ser::to_string(&payload_of(PlaySettings::default())).unwrap(),
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
    fn only_a_read_copy_sets_the_lock() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        assert_eq!(lock.base(), Some(1_000));
        lock.apply(SyncOutcome::Read(2_000));
        assert_eq!(lock.base(), Some(2_000), "a later read replaces it");
    }

    #[test]
    fn a_204_upload_teaches_the_lock_nothing_until_the_read_back_lands() {
        let mut lock = SyncLock::default();
        lock.apply(SyncOutcome::Read(1_000));
        lock.apply(SyncOutcome::Uploaded);
        assert_eq!(lock.base(), Some(1_000), "the server ignored the client stamp, so the old base is all we know");
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

        let retry = build_blob(&payload_of(PlaySettings::default()), 3_000, lock.base()).unwrap();
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
