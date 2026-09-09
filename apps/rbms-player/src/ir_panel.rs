//! The NETWORK settings tab: which of its rows hold text, which hide what is typed, and how the
//! values the running program owns are rendered.
//!
//! The rows themselves — their order, labels and kinds — are described once in
//! `rbms_config::SETTINGS`, and every routing decision here is read back out of that table by
//! [`rbms_config::SettingId`]. Nothing in this file numbers a row.

use rbms_config::{SettingId, SettingKind, SettingTab, descriptor};

/// Value shown on a row that runs an action rather than holding a value; matches the KEY CONFIG
/// row's affordance.
pub(crate) use rbms_config::ACTION_VALUE;

/// Value of a text row that has not been filled in.
pub(crate) use rbms_config::NONE_VALUE as EMPTY_VALUE;

/// Value of the PASSWORD row while no password is held.
pub(crate) use rbms_config::UNSET_VALUE as PASSWORD_EMPTY_VALUE;

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
pub(crate) fn network_text_field(id: SettingId) -> Option<NetworkTextField> {
    Some(match id {
        SettingId::ServerUrl => NetworkTextField::ServerUrl,
        SettingId::PlayerId => NetworkTextField::PlayerId,
        SettingId::Email => NetworkTextField::Email,
        SettingId::Password => NetworkTextField::Password,
        _ => return None,
    })
}

/// Whether a row opens the in-place text editor (as opposed to toggling or running an action).
/// Read out of the descriptor table, so a row declared as text there always gets an editor.
pub(crate) fn is_text_row(id: SettingId) -> bool {
    matches!(descriptor(id).kind, SettingKind::Text { .. })
}

/// Whether the row's editor must hide what is typed.
pub(crate) fn is_secret_row(id: SettingId) -> bool {
    matches!(descriptor(id).kind, SettingKind::Text { secret: true, .. })
}

/// Row label, or `None` when the row is not on the NETWORK tab.
pub(crate) fn network_row_label(id: SettingId) -> Option<&'static str> {
    let row = descriptor(id);
    (row.tab == SettingTab::Network).then_some(row.label)
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
    use rbms_config::{Config, tab_rows};

    fn network_rows() -> Vec<SettingId> {
        tab_rows(SettingTab::Network, &Config::default())
    }

    #[test]
    fn every_network_row_has_a_distinct_label() {
        let mut labels: Vec<&str> = network_rows().into_iter().map(|id| network_row_label(id).expect("labelled")).collect();
        let count = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), count, "no two NETWORK rows share a label");
    }

    #[test]
    fn a_row_from_another_tab_is_not_a_network_row() {
        assert_eq!(network_row_label(SettingId::Autoplay), None, "a PLAY row is not a NETWORK row");
        assert_eq!(network_row_label(SettingId::AudioBuffer), None);
        assert!(network_rows().contains(&SettingId::ServerUrl));
    }

    #[test]
    fn text_rows_are_exactly_the_four_editable_fields() {
        let text: Vec<SettingId> = network_rows().into_iter().filter(|&id| is_text_row(id)).collect();
        assert_eq!(text, vec![SettingId::ServerUrl, SettingId::PlayerId, SettingId::Email, SettingId::Password]);
        assert!(is_secret_row(SettingId::Password));
        assert!(!is_secret_row(SettingId::Email), "only the password is hidden");
    }

    #[test]
    fn optional_values_fall_back_to_the_empty_marker() {
        assert_eq!(optional_value(Some("https://ir.example/api")), "https://ir.example/api");
        assert_eq!(optional_value(None), EMPTY_VALUE);
        assert_eq!(optional_value(Some("   ")), EMPTY_VALUE);
    }

    #[test]
    fn the_panel_and_the_settings_table_share_one_set_of_row_values() {
        assert_eq!(ACTION_VALUE, rbms_config::ACTION_VALUE);
        assert_eq!(EMPTY_VALUE, rbms_config::NONE_VALUE);
        assert_eq!(PASSWORD_EMPTY_VALUE, rbms_config::UNSET_VALUE);
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
        assert_eq!(network_text_field(SettingId::ServerUrl), Some(NetworkTextField::ServerUrl));
        assert_eq!(network_text_field(SettingId::PlayerId), Some(NetworkTextField::PlayerId));
        assert_eq!(network_text_field(SettingId::Email), Some(NetworkTextField::Email));
        assert_eq!(network_text_field(SettingId::Password), Some(NetworkTextField::Password));
    }

    #[test]
    fn a_row_that_holds_no_text_maps_to_no_field_at_all() {
        for id in SettingId::ALL.into_iter().filter(|&id| !is_text_row(id)) {
            assert_eq!(network_text_field(id), None, "{id:?} would receive a committed editor value");
        }
        assert_eq!(network_text_field(SettingId::Login), None, "committing a typed password here must write nothing");
        assert_eq!(network_text_field(SettingId::Logout), None);
        assert_eq!(network_text_field(SettingId::Rivals), None);
    }

    #[test]
    fn only_the_password_row_masks_what_is_typed() {
        for id in SettingId::ALL {
            assert_eq!(is_secret_row(id), id == SettingId::Password, "{id:?}");
        }
    }

    #[test]
    fn the_text_rows_and_their_fields_are_the_same_set() {
        for id in SettingId::ALL {
            assert_eq!(is_text_row(id), network_text_field(id).is_some(), "{id:?} is described as text but has no field, or the other way round");
        }
    }
}
