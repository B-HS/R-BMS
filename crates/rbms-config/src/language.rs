//! Which language the screens are written in: one the player chose, or whatever the system is set
//! to.
//!
//! The choice is stored under a separator-free token, so a reworded label leaves an existing file
//! alone, and an unknown token reads back as AUTO instead of failing the whole document. What AUTO
//! comes to is settled by the running program, which alone can ask the system.

use serde::{Deserialize, Deserializer, Serializer};

/// How many choices the LANGUAGE row offers.
pub const LANGUAGE_CHOICE_COUNT: usize = 4;

/// The language the screens are written in, as the player chose it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LanguageChoice {
    /// Whatever language the system is set to, when the screens are written in it, and English
    /// when they are not.
    #[default]
    Auto,
    English,
    Korean,
    Japanese,
}

impl LanguageChoice {
    /// Every choice, in the order the row steps through them.
    pub const ALL: [LanguageChoice; LANGUAGE_CHOICE_COUNT] = [LanguageChoice::Auto, LanguageChoice::English, LanguageChoice::Korean, LanguageChoice::Japanese];

    /// Name the row shows. A language is named in itself, so a player who cannot read the language
    /// the screens are in can still find their own.
    pub fn label(self) -> &'static str {
        match self {
            LanguageChoice::Auto => "AUTO",
            LanguageChoice::English => "ENGLISH",
            LanguageChoice::Korean => "한국어",
            LanguageChoice::Japanese => "日本語",
        }
    }

    /// Token the choice is stored under.
    pub fn token(self) -> &'static str {
        match self {
            LanguageChoice::Auto => "AUTO",
            LanguageChoice::English => "ENGLISH",
            LanguageChoice::Korean => "KOREAN",
            LanguageChoice::Japanese => "JAPANESE",
        }
    }

    /// The choice a stored token names, falling back to AUTO.
    pub fn from_token(token: &str) -> LanguageChoice {
        LanguageChoice::ALL.into_iter().find(|entry| entry.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }
}

/// Labels the LANGUAGE row shows, one per entry of [`LanguageChoice::ALL`].
pub const LANGUAGE_LABELS: &[&str] = &["AUTO", "ENGLISH", "한국어", "日本語"];

/// Stores [`LanguageChoice`] as its token.
pub mod language_token {
    use super::{Deserialize, Deserializer, LanguageChoice, Serializer};

    /// Writes the token.
    pub fn serialize<S: Serializer>(value: &LanguageChoice, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    /// Reads the token back.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<LanguageChoice, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(LanguageChoice::from_token(&token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CURRENT_SCHEMA_VERSION, Config, migrate};

    #[test]
    fn the_row_labels_follow_the_choices_in_order() {
        assert_eq!(LanguageChoice::ALL.map(LanguageChoice::label).to_vec(), LANGUAGE_LABELS);
        assert_eq!(LanguageChoice::default(), LanguageChoice::Auto);
    }

    #[test]
    fn every_choice_reads_back_from_its_token_and_an_unknown_one_reads_as_auto() {
        for entry in LanguageChoice::ALL {
            assert_eq!(LanguageChoice::from_token(entry.token()), entry);
            assert!(entry.token().bytes().all(|byte| byte.is_ascii_uppercase()), "{entry:?} is stored under a plain token");
        }
        assert_eq!(LanguageChoice::from_token(" korean "), LanguageChoice::Korean, "the token is trimmed and case-blind");
        assert_eq!(LanguageChoice::from_token("FRENCH"), LanguageChoice::Auto);
        assert_eq!(LanguageChoice::from_token("한국어"), LanguageChoice::Auto, "a label is not a token");
        assert_eq!(LanguageChoice::from_token(""), LanguageChoice::Auto);
    }

    #[test]
    fn a_file_from_before_the_row_reads_as_auto() {
        let older = format!("(schema_version: {CURRENT_SCHEMA_VERSION}, display: (window_mode: \"BORDERLESS\"))");
        let (config, from) = migrate(&older).expect("a current file without the field parses");
        assert_eq!(from, None, "the schema version is not raised for the field");
        assert_eq!(config.display.language, LanguageChoice::Auto);
    }

    #[test]
    fn the_choice_is_written_as_its_token_and_read_back() {
        for entry in LanguageChoice::ALL {
            let mut config = Config::default();
            config.display.language = entry;
            let text = ron::to_string(&config).expect("a config serializes");
            assert!(text.contains(&format!("language:\"{}\"", entry.token())), "{entry:?}: {text}");
            let (back, _) = migrate(&text).expect("a written config reads back");
            assert_eq!(back.display.language, entry);
        }
        let (unknown, _) = migrate(&format!("(schema_version: {CURRENT_SCHEMA_VERSION}, display: (language: \"KLINGON\"))")).expect("an unknown token parses");
        assert_eq!(unknown.display.language, LanguageChoice::Auto, "an unknown token does not fail the document");
    }
}
