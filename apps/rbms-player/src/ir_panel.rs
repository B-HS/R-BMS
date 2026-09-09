//! The NETWORK settings tab: which rows it has, what they are called, and how their values read.
//!
//! The row indices continue the global settings numbering used by `setting_line`/`adjust_setting`,
//! so the tab is described in exactly one place.

/// Global settings-row indices owned by the NETWORK tab.
pub(crate) const SETTING_SERVER_URL: usize = 22;
pub(crate) const SETTING_PLAYER_ID: usize = 23;
pub(crate) const SETTING_ACCOUNT: usize = 24;
pub(crate) const SETTING_EMAIL: usize = 25;
pub(crate) const SETTING_PASSWORD: usize = 26;
pub(crate) const SETTING_LOGIN: usize = 27;
pub(crate) const SETTING_REGISTER: usize = 28;
pub(crate) const SETTING_LOGOUT: usize = 29;
pub(crate) const SETTING_SYNC_SETTINGS: usize = 30;
pub(crate) const SETTING_UPLOAD_SETTINGS: usize = 31;
pub(crate) const SETTING_DOWNLOAD_SETTINGS: usize = 32;
pub(crate) const SETTING_AUTO_UPLOAD_REPLAY: usize = 33;
pub(crate) const SETTING_RIVALS: usize = 34;

/// The NETWORK tab, top to bottom.
pub(crate) const NETWORK_SETTING_ROWS: &[usize] = &[
    SETTING_SERVER_URL,
    SETTING_PLAYER_ID,
    SETTING_ACCOUNT,
    SETTING_EMAIL,
    SETTING_PASSWORD,
    SETTING_LOGIN,
    SETTING_REGISTER,
    SETTING_LOGOUT,
    SETTING_SYNC_SETTINGS,
    SETTING_UPLOAD_SETTINGS,
    SETTING_DOWNLOAD_SETTINGS,
    SETTING_AUTO_UPLOAD_REPLAY,
    SETTING_RIVALS,
];

/// Value shown on a row that runs an action rather than holding a value; matches the KEY CONFIG
/// row's affordance.
pub(crate) const ACTION_VALUE: &str = ">";

/// Value of a text row that has not been filled in.
pub(crate) const EMPTY_VALUE: &str = "(none)";

/// Value of the PASSWORD row while no password is held.
pub(crate) const PASSWORD_EMPTY_VALUE: &str = "(not set)";

/// Character the PASSWORD row is masked with. ASCII, so it renders in every bundled font.
pub(crate) const PASSWORD_MASK_CHAR: char = '*';

/// Longest mask drawn for the PASSWORD row, so a long password cannot report its exact length or
/// overflow the value column.
pub(crate) const PASSWORD_MASK_MAX: usize = 12;

/// Row appended under the rival ids in the inline rival list.
pub(crate) const RIVAL_ADD_ROW: &str = "+ ADD RIVAL (PLAYER ID)";

/// The setting a NETWORK text row edits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NetworkTextField {
    ServerUrl,
    PlayerId,
    Email,
    Password,
}

/// The field a committed editor writes, or `None` when the row holds no text.
///
/// The mapping is total and has no fallback: a commit against a row that is not a text row writes
/// nothing. A catch-all arm here would send whatever was typed — a password included — into
/// whichever field the arm named, and persist it there in clear.
pub(crate) fn network_text_field(index: usize) -> Option<NetworkTextField> {
    Some(match index {
        SETTING_SERVER_URL => NetworkTextField::ServerUrl,
        SETTING_PLAYER_ID => NetworkTextField::PlayerId,
        SETTING_EMAIL => NetworkTextField::Email,
        SETTING_PASSWORD => NetworkTextField::Password,
        _ => return None,
    })
}

/// Whether a row opens the in-place text editor (as opposed to toggling or running an action).
pub(crate) fn is_text_row(index: usize) -> bool {
    network_text_field(index).is_some()
}

/// Whether the row's editor must hide what is typed.
pub(crate) fn is_secret_row(index: usize) -> bool {
    network_text_field(index) == Some(NetworkTextField::Password)
}

/// Row label, or `None` when the index is not a NETWORK row.
pub(crate) fn network_row_label(index: usize) -> Option<&'static str> {
    Some(match index {
        SETTING_SERVER_URL => "SERVER URL",
        SETTING_PLAYER_ID => "PLAYER ID",
        SETTING_ACCOUNT => "ACCOUNT",
        SETTING_EMAIL => "EMAIL",
        SETTING_PASSWORD => "PASSWORD",
        SETTING_LOGIN => "LOGIN",
        SETTING_REGISTER => "REGISTER",
        SETTING_LOGOUT => "LOGOUT",
        SETTING_SYNC_SETTINGS => "SYNC SETTINGS",
        SETTING_UPLOAD_SETTINGS => "UPLOAD SETTINGS NOW",
        SETTING_DOWNLOAD_SETTINGS => "DOWNLOAD SETTINGS NOW",
        SETTING_AUTO_UPLOAD_REPLAY => "AUTO UPLOAD REPLAY",
        SETTING_RIVALS => "RIVALS",
        _ => return None,
    })
}

/// Value of an optional text row.
pub(crate) fn optional_value(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(text) => text.to_string(),
        None => EMPTY_VALUE.to_string(),
    }
}

/// Value of the PASSWORD row: a fixed-width-capped mask, never the password itself and never its
/// exact length once it is long.
pub(crate) fn password_value(len: usize) -> String {
    if len == 0 {
        return PASSWORD_EMPTY_VALUE.to_string();
    }
    std::iter::repeat_n(PASSWORD_MASK_CHAR, len.min(PASSWORD_MASK_MAX)).collect()
}

/// What the in-place editor shows while typing: the buffer with a caret, masked on a secret row.
pub(crate) fn editor_display(buffer: &str, secret: bool) -> String {
    if secret {
        let mask: String = std::iter::repeat_n(PASSWORD_MASK_CHAR, buffer.chars().count().min(PASSWORD_MASK_MAX)).collect();
        return format!("{mask}_");
    }
    format!("{buffer}_")
}

/// The inline rival list: one row per rival id, then the add row.
pub(crate) fn rival_rows(rivals: &[String]) -> Vec<String> {
    let mut rows: Vec<String> = rivals.to_vec();
    rows.push(RIVAL_ADD_ROW.to_string());
    rows
}

/// Normalise a typed rival id. `None` when it is blank or already in the list.
pub(crate) fn normalise_rival(input: &str, existing: &[String]) -> Option<String> {
    let id = input.trim();
    if id.is_empty() || existing.iter().any(|r| r == id) {
        return None;
    }
    Some(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_network_row_has_a_distinct_label() {
        let mut labels: Vec<&str> = NETWORK_SETTING_ROWS.iter().map(|&i| network_row_label(i).expect("labelled")).collect();
        let count = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), count, "no two NETWORK rows share a label");
    }

    #[test]
    fn row_indices_are_unique_and_recognised() {
        let mut indices = NETWORK_SETTING_ROWS.to_vec();
        let count = indices.len();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), count);
        assert!(!NETWORK_SETTING_ROWS.contains(&0), "a PLAY row is not a NETWORK row");
        assert_eq!(network_row_label(999), None);
    }

    #[test]
    fn text_rows_are_exactly_the_four_editable_fields() {
        let text: Vec<usize> = NETWORK_SETTING_ROWS.iter().copied().filter(|&i| is_text_row(i)).collect();
        assert_eq!(text, vec![SETTING_SERVER_URL, SETTING_PLAYER_ID, SETTING_EMAIL, SETTING_PASSWORD]);
        assert!(is_secret_row(SETTING_PASSWORD));
        assert!(!is_secret_row(SETTING_EMAIL), "only the password is hidden");
    }

    #[test]
    fn optional_values_fall_back_to_the_empty_marker() {
        assert_eq!(optional_value(Some("https://ir.example/api")), "https://ir.example/api");
        assert_eq!(optional_value(None), EMPTY_VALUE);
        assert_eq!(optional_value(Some("   ")), EMPTY_VALUE);
    }

    #[test]
    fn the_password_row_never_shows_the_password() {
        assert_eq!(password_value(0), PASSWORD_EMPTY_VALUE);
        assert_eq!(password_value(4), "****");
        assert_eq!(password_value(200).chars().count(), PASSWORD_MASK_MAX, "a long password is not measurable from the row");
        assert!(password_value(8).chars().all(|c| c == PASSWORD_MASK_CHAR));
    }

    #[test]
    fn the_editor_masks_a_secret_row_and_shows_a_caret() {
        assert_eq!(editor_display("dj@example.test", false), "dj@example.test_");
        assert_eq!(editor_display("hunter2", true), "*******_");
        assert!(!editor_display("hunter2", true).contains("hunter2"));
        assert_eq!(editor_display("", true), "_");
    }

    #[test]
    fn the_rival_list_always_ends_with_the_add_row() {
        assert_eq!(rival_rows(&[]), vec![RIVAL_ADD_ROW.to_string()]);
        let rows = rival_rows(&["one".to_string(), "two".to_string()]);
        assert_eq!(rows, vec!["one".to_string(), "two".to_string(), RIVAL_ADD_ROW.to_string()]);
    }

    #[test]
    fn a_rival_id_is_trimmed_and_never_duplicated() {
        let existing = vec!["friend".to_string()];
        assert_eq!(normalise_rival("  newone  ", &existing), Some("newone".to_string()));
        assert_eq!(normalise_rival("friend", &existing), None, "adding the same rival twice is a no-op");
        assert_eq!(normalise_rival("   ", &existing), None);
    }

    #[test]
    fn every_network_text_row_maps_to_its_own_field() {
        assert_eq!(network_text_field(SETTING_SERVER_URL), Some(NetworkTextField::ServerUrl));
        assert_eq!(network_text_field(SETTING_PLAYER_ID), Some(NetworkTextField::PlayerId));
        assert_eq!(network_text_field(SETTING_EMAIL), Some(NetworkTextField::Email));
        assert_eq!(network_text_field(SETTING_PASSWORD), Some(NetworkTextField::Password));
    }

    #[test]
    fn a_row_that_holds_no_text_maps_to_no_field_at_all() {
        for row in NETWORK_SETTING_ROWS.iter().copied().filter(|row| !is_text_row(*row)) {
            assert_eq!(network_text_field(row), None, "row {row} would receive a committed editor value");
        }
        assert_eq!(network_text_field(SETTING_LOGIN), None, "committing a typed password here must write nothing");
        assert_eq!(network_text_field(SETTING_LOGOUT), None);
        assert_eq!(network_text_field(SETTING_RIVALS), None);
        assert_eq!(network_text_field(usize::MAX), None);
    }

    #[test]
    fn only_the_password_row_masks_what_is_typed() {
        for row in NETWORK_SETTING_ROWS.iter().copied() {
            assert_eq!(is_secret_row(row), row == SETTING_PASSWORD, "row {row}");
        }
    }
}
