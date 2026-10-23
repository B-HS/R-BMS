//! Catalogs of the interface's fixed words, one string per [`Language`] for every entry.
//!
//! A crate writes its own catalogs with [`crate::translations!`]: an enum whose every variant is
//! one sentence or label, given in English, Korean and Japanese in that order. An entry missing a
//! language does not match the macro, so it is a compile error, and adding a language makes every
//! entry one until it is filled in. A term kept as it is in every language (`EX SCORE`, a gauge's
//! name) is written once and is marked as kept rather than as a copy left untranslated.
//!
//! Numbers and names are put into a sentence through numbered placeholders, `{0}`, `{1}`, ...
//! ([`Catalog::fill`]), so each language can put them where its own word order wants them. Whether
//! the three strings of an entry agree in their placeholders, their outer white space and in being
//! translated at all is not something the compiler sees; [`catalog_problems`] checks it, and every
//! catalog's tests call it on the entry list the macro writes (`ALL`).
//!
//! The compiler does not warn about an entry no code names: the enum is written by a macro of
//! another crate, and the dead-code lint stays quiet about what such a macro writes. A sentence
//! nothing says any more has to be taken out by hand.

use std::collections::BTreeSet;
use std::fmt::Display;

use crate::language::{LANGUAGE_COUNT, Language};

/// One catalog: an enum of fixed words written by [`crate::translations!`].
pub trait Catalog: Copy + std::fmt::Debug + 'static {
    /// The entry's strings, one per language in [`Language::ALL`] order.
    fn strings(self) -> [&'static str; LANGUAGE_COUNT];

    /// Whether the entry is one term kept the same in every language rather than translated.
    fn is_kept(self) -> bool;

    /// The entry in `language`.
    fn get(self, language: Language) -> &'static str {
        self.strings()[language.id()]
    }

    /// The entry in `language` with each placeholder `{n}` replaced by `args[n]`. A placeholder
    /// with no argument is left as written.
    fn fill(self, language: Language, args: &[&dyn Display]) -> String {
        fill(self.get(language), args)
    }
}

/// `template` with each placeholder `{n}` replaced by `args[n]`. Braces that do not hold a number,
/// and a placeholder past the end of `args`, are left as written.
pub fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        match placeholder_at(after) {
            Some((index, length)) if index < args.len() => {
                out.push_str(&args[index].to_string());
                rest = &after[length..];
            }
            _ => {
                out.push('{');
                rest = &after['{'.len_utf8()..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The number and length of the placeholder `text` starts with, if it starts with one.
fn placeholder_at(text: &str) -> Option<(usize, usize)> {
    let inner = text.strip_prefix('{')?;
    let digits = inner.find(|c: char| !c.is_ascii_digit())?;
    if digits == 0 || !inner[digits..].starts_with('}') {
        return None;
    }
    let index = inner[..digits].parse().ok()?;
    Some((index, '{'.len_utf8() + digits + '}'.len_utf8()))
}

/// The placeholder numbers a string uses, each once.
pub fn placeholders(text: &str) -> BTreeSet<usize> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open..];
        match placeholder_at(after) {
            Some((index, length)) => {
                found.insert(index);
                rest = &after[length..];
            }
            None => rest = &after['{'.len_utf8()..],
        }
    }
    found
}

/// The white space a string starts and ends with.
fn outer_space(text: &str) -> (&str, &str) {
    let start = &text[..text.len() - text.trim_start().len()];
    let end = &text[text.trim_end().len()..];
    (start, end)
}

/// What is wrong with the entries of one catalog, one line per fault, empty when nothing is.
///
/// Every string must say something; every language must use the placeholders the English uses and
/// start and end with the same white space; and an entry that is not a kept term must be written
/// in Korean and Japanese, which a string with no character outside ASCII is not.
pub fn catalog_problems<T: Catalog>(entries: &[T]) -> Vec<String> {
    let mut problems = Vec::new();
    for entry in entries {
        let strings = entry.strings();
        let english = strings[Language::English.id()];
        for language in Language::ALL {
            let text = strings[language.id()];
            if text.trim().is_empty() {
                problems.push(format!("{entry:?}: the {language:?} string is empty"));
                continue;
            }
            if placeholders(text) != placeholders(english) {
                problems.push(format!("{entry:?}: the {language:?} string uses {:?}, the English {:?}", placeholders(text), placeholders(english)));
            }
            if outer_space(text) != outer_space(english) {
                problems.push(format!("{entry:?}: the {language:?} string starts or ends with other white space than the English"));
            }
            if language != Language::English && !entry.is_kept() && text.is_ascii() {
                problems.push(format!("{entry:?}: the {language:?} string is not translated: {text:?}"));
            }
        }
    }
    problems
}

/// Writes a catalog: an enum of fixed words, every variant given in English, Korean and Japanese.
///
/// ```
/// use rbms_model::Language;
/// use rbms_model::translations::Catalog;
///
/// rbms_model::translations! {
///     /// Words of an example screen.
///     pub enum ExampleText {
///         MusicSelect => ("MUSIC SELECT", "선곡", "選曲"),
///         ChartCount => ("{0} CHARTS", "차트 {0}개", "{0}譜面"),
///         ExScore => ("EX SCORE"),
///     }
/// }
///
/// assert_eq!(ExampleText::MusicSelect.get(Language::Japanese), "選曲");
/// assert_eq!(ExampleText::ChartCount.fill(Language::Korean, &[&12]), "차트 12개");
/// assert_eq!(ExampleText::ExScore.get(Language::Korean), "EX SCORE");
/// ```
///
/// An entry is `Name => ("English", "Korean", "Japanese")`, or `Name => ("TERM")` for a term kept
/// the same in every language. The enum implements [`translations::Catalog`](crate::translations::Catalog)
/// and has an `ALL` constant listing every entry, which its tests hand to
/// [`catalog_problems`](crate::translations::catalog_problems).
#[macro_export]
macro_rules! translations {
    (@strings ($english:literal, $korean:literal, $japanese:literal $(,)?)) => {
        [$english, $korean, $japanese]
    };
    (@strings ($kept:literal $(,)?)) => {
        [$kept, $kept, $kept]
    };
    (@kept ($english:literal, $korean:literal, $japanese:literal $(,)?)) => {
        false
    };
    (@kept ($kept:literal $(,)?)) => {
        true
    };
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$entry_meta:meta])* $entry:ident => $strings:tt),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($(#[$entry_meta])* $entry),+
        }

        impl $name {
            /// Every entry, in the order the catalog lists them.
            $vis const ALL: &'static [$name] = &[$($name::$entry),+];
        }

        impl $crate::translations::Catalog for $name {
            fn strings(self) -> [&'static str; $crate::LANGUAGE_COUNT] {
                match self {
                    $($name::$entry => $crate::translations!(@strings $strings)),+
                }
            }

            fn is_kept(self) -> bool {
                match self {
                    $($name::$entry => $crate::translations!(@kept $strings)),+
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::translations! {
        /// A well-formed catalog.
        enum GoodText {
            MusicSelect => ("MUSIC SELECT", "선곡", "選曲"),
            ChartCount => ("{0} CHARTS", "차트 {0}개", "{0}譜面"),
            Swapped => ("{0} OF {1}", "{1} 중 {0}", "{1}の{0}"),
            Indented => ("  NOTE ", "  참고 ", "  注 "),
            ExScore => ("EX SCORE"),
        }
    }

    crate::translations! {
        enum BadText {
            Empty => ("SETTINGS", "", "設定"),
            Blank => ("SETTINGS", "설정", "   "),
            LostPlaceholder => ("{0} CHARTS", "차트", "{0}譜面"),
            ExtraPlaceholder => ("CHARTS", "차트 {1}", "譜面"),
            Spaced => ("SETTINGS", " 설정", "設定 "),
            Copied => ("SETTINGS", "SETTINGS", "設定"),
            CopiedAlmost => ("OPTION", "옵션", "OPTIONS"),
        }
    }

    #[test]
    fn an_entry_is_read_in_each_language_in_catalog_order() {
        assert_eq!(GoodText::MusicSelect.get(Language::English), "MUSIC SELECT");
        assert_eq!(GoodText::MusicSelect.get(Language::Korean), "선곡");
        assert_eq!(GoodText::MusicSelect.get(Language::Japanese), "選曲");
        assert_eq!(GoodText::ExScore.strings(), ["EX SCORE"; LANGUAGE_COUNT]);
        assert!(GoodText::ExScore.is_kept());
        assert!(!GoodText::MusicSelect.is_kept());
        assert_eq!(GoodText::ALL.len(), 5);
    }

    #[test]
    fn placeholders_are_filled_where_each_language_puts_them() {
        assert_eq!(GoodText::ChartCount.fill(Language::English, &[&12]), "12 CHARTS");
        assert_eq!(GoodText::ChartCount.fill(Language::Japanese, &[&12]), "12譜面");
        assert_eq!(GoodText::Swapped.fill(Language::English, &[&"A", &"B"]), "A OF B");
        assert_eq!(GoodText::Swapped.fill(Language::Korean, &[&"A", &"B"]), "B 중 A");
    }

    #[test]
    fn braces_that_are_not_a_filled_placeholder_are_left_as_written() {
        assert_eq!(fill("{0} AND {1}", &[&1]), "1 AND {1}", "a placeholder with no argument stays");
        assert_eq!(fill("{} {x} {", &[&1]), "{} {x} {");
        assert_eq!(fill("{{0}}", &[&7]), "{7}");
        assert_eq!(fill("{10}", &[&0]), "{10}");
        assert_eq!(fill("", &[&0]), "");
        assert_eq!(fill("한국어 {0}", &[&"テキスト"]), "한국어 テキスト", "multi-byte text around a placeholder");
    }

    #[test]
    fn placeholders_are_read_as_a_set_of_numbers() {
        assert_eq!(placeholders("{1} {0} {1}"), BTreeSet::from([0, 1]));
        assert!(placeholders("{} {a} {").is_empty());
    }

    #[test]
    fn a_well_formed_catalog_has_no_problems() {
        assert_eq!(catalog_problems(GoodText::ALL), Vec::<String>::new());
    }

    #[test]
    fn each_kind_of_fault_is_named_against_its_entry() {
        let problems = catalog_problems(BadText::ALL);
        let about = |entry: BadText| problems.iter().filter(|line| line.starts_with(&format!("{entry:?}:"))).count();
        assert_eq!(about(BadText::Empty), 1);
        assert_eq!(about(BadText::Blank), 1);
        assert_eq!(about(BadText::LostPlaceholder), 1);
        assert_eq!(about(BadText::ExtraPlaceholder), 1);
        assert_eq!(about(BadText::Spaced), 2);
        assert_eq!(about(BadText::Copied), 1);
        assert_eq!(about(BadText::CopiedAlmost), 1, "a Japanese string with no Japanese in it is a copy");
        assert_eq!(problems.len(), 8, "{problems:#?}");
    }
}
