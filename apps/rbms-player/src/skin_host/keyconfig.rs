//! Property cluster K: the key configuration screen.
//!
//! Where the values come from: what each key is bound to, for the labels a key configuration skin
//! prints.
//!
//! What this cluster answers: strings 40-49 and 240-283.
//!
//! The reference numbers the key names in two runs (`StringPropertyPattern.KEY_1_TO_10` and
//! `KEY_11_TO_54`): the first ten from string 40, and the forty-four after them from string 240, so
//! string 240 is the eleventh key and not the first again. Here a key is a lane of the mode being
//! configured, in lane order, and a string past the last lane reads as absent.

use std::borrow::Cow;

use rbms_skin::property::{STRING_KEYNAME_EXTENDED_FIRST, STRING_KEYNAME_EXTENDED_LAST, STRING_KEYNAME_FIRST, STRING_KEYNAME_LAST};

use super::ClusterState;

/// How many key names the first run holds, which is where the second run carries on from.
const FIRST_RUN_KEYS: i32 = STRING_KEYNAME_LAST - STRING_KEYNAME_FIRST + 1;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyConfigState<'a> {
    /// What each lane of the mode being configured is bound to, in lane order. Empty on every
    /// screen but the key configuration one, which leaves the cluster knowing nothing.
    pub keys: &'a [String],
}

impl KeyConfigState<'_> {
    /// Which key a key-name string asks for, counting from zero, or `None` for a string that is not
    /// a key name.
    fn key_of(id: i32) -> Option<usize> {
        let key = match id {
            STRING_KEYNAME_FIRST..=STRING_KEYNAME_LAST => id - STRING_KEYNAME_FIRST,
            STRING_KEYNAME_EXTENDED_FIRST..=STRING_KEYNAME_EXTENDED_LAST => id - STRING_KEYNAME_EXTENDED_FIRST + FIRST_RUN_KEYS,
            _ => return None,
        };
        usize::try_from(key).ok()
    }
}

impl ClusterState for KeyConfigState<'_> {
    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        let bound = self.keys.get(KeyConfigState::key_of(id)?)?;
        Some(Cow::Borrowed(bound.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many lanes the mode configured in these tests has: more than the first run of strings
    /// names, so the second run has something to answer.
    const LANES: usize = 16;

    fn bound() -> Vec<String> {
        (0..LANES).map(|lane| format!("KEY{lane}")).collect()
    }

    #[test]
    fn the_first_ten_strings_name_the_first_ten_keys() {
        let keys = bound();
        let state = KeyConfigState { keys: &keys };
        assert_eq!(state.text(STRING_KEYNAME_FIRST).as_deref(), Some("KEY0"));
        assert_eq!(state.text(STRING_KEYNAME_LAST).as_deref(), Some("KEY9"));
    }

    #[test]
    fn the_second_run_carries_on_from_the_eleventh_key() {
        let keys = bound();
        let state = KeyConfigState { keys: &keys };
        assert_eq!(state.text(STRING_KEYNAME_EXTENDED_FIRST).as_deref(), Some("KEY10"), "string 240 is the eleventh key, not the first again");
        assert_eq!(state.text(STRING_KEYNAME_EXTENDED_FIRST + 5).as_deref(), Some("KEY15"));
        assert_eq!(state.text(STRING_KEYNAME_EXTENDED_FIRST + 6), None, "a string past the last lane names no key");
    }

    #[test]
    fn a_string_that_is_no_key_name_and_a_screen_with_no_keys_are_not_answered() {
        let keys = bound();
        let state = KeyConfigState { keys: &keys };
        assert_eq!(state.text(STRING_KEYNAME_LAST + 1), None);
        assert_eq!(state.text(STRING_KEYNAME_EXTENDED_LAST + 1), None);
        assert_eq!(KeyConfigState::default().text(STRING_KEYNAME_FIRST), None, "a screen that binds no keys leaves the string to whoever else knows it");
    }
}
