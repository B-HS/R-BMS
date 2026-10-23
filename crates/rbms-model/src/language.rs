//! The languages the interface is written in.
//!
//! A language here is one the player's own words are written in: its screens, settings and
//! messages. What a chart or a table is called stays as its author wrote it whatever this says.

/// How many languages the interface is written in. Every catalog entry carries this many strings.
pub const LANGUAGE_COUNT: usize = 3;

/// A language the interface is written in.
///
/// The order of the variants is the order a catalog entry lists its strings in
/// ([`crate::translations!`]), and [`Language::id`] is the position in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    English,
    Korean,
    Japanese,
}

impl Language {
    /// Every language, in catalog order.
    pub const ALL: [Language; LANGUAGE_COUNT] = [Language::English, Language::Korean, Language::Japanese];

    /// Position of the language in [`Language::ALL`] and in a catalog entry's strings: 0, 1, 2.
    pub const fn id(self) -> usize {
        self as usize
    }

    /// The ISO 639-1 code the language is named by in a locale.
    pub const fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Korean => "ko",
            Language::Japanese => "ja",
        }
    }

    /// The language a locale or a bare language code names, or `None` for one the interface is not
    /// written in.
    ///
    /// Only the language subtag is read, so `ko`, `ko-KR`, `ko_KR.UTF-8` and `KO` all name Korean,
    /// while `en-KR` names English: a host set to English in Korea reads English. A region never
    /// picks the language.
    pub fn from_code(locale: &str) -> Option<Language> {
        let subtag = locale.trim().split(['-', '_', '.', '@']).next().unwrap_or_default();
        Language::ALL.into_iter().find(|language| language.code().eq_ignore_ascii_case(subtag))
    }

    /// The language a host locale asks for, falling back to English for one the interface is not
    /// written in, an empty one and one that names no language at all (`C`, `POSIX`).
    pub fn from_locale(locale: &str) -> Language {
        Language::from_code(locale).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ids_follow_the_catalog_order_and_english_is_the_default() {
        assert_eq!(Language::ALL.map(Language::id), [0, 1, 2]);
        assert_eq!(Language::default(), Language::English);
        assert_eq!(Language::ALL.map(Language::code), ["en", "ko", "ja"]);
    }

    #[test]
    fn a_locale_is_read_by_its_language_alone() {
        let table = [
            ("ko-KR", Language::Korean),
            ("ko", Language::Korean),
            ("ko_KR.UTF-8", Language::Korean),
            ("KO", Language::Korean),
            ("ja-JP", Language::Japanese),
            ("ja", Language::Japanese),
            ("ja_JP.eucJP", Language::Japanese),
            ("en-US", Language::English),
            ("en-KR", Language::English),
            ("en", Language::English),
            ("zh-TW", Language::English),
            ("fr", Language::English),
            ("C", Language::English),
            ("POSIX", Language::English),
            ("", Language::English),
            ("  ", Language::English),
            ("kor", Language::English),
        ];
        for (locale, expected) in table {
            assert_eq!(Language::from_locale(locale), expected, "{locale:?}");
        }
    }

    #[test]
    fn only_a_language_the_interface_is_written_in_is_a_code() {
        assert_eq!(Language::from_code(" ja "), Some(Language::Japanese));
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("zh-TW"), None);
        assert_eq!(Language::from_code(""), None);
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
    }
}
