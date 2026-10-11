//! Which text encoding a chart file is written in.
//!
//! A chart carries no label for its encoding. Most are Shift_JIS, the ones made in Korea are often
//! EUC-KR (code page 949), and newer ones are UTF-8 with or without a byte order mark. One answer
//! is given for the whole file, because the keysound and image names are in the same encoding as
//! the title and a name read wrongly is a file that is not found.
//!
//! The answer is reached in this order:
//!
//! 1. A UTF-16 byte order mark is believed.
//! 2. Bytes that are well-formed UTF-8 are UTF-8, with or without the mark in front of them.
//! 3. Bytes behind a UTF-8 mark that are not well-formed are still UTF-8 while more of them read as
//!    UTF-8 than do not ([`mostly_utf8`]): a marked chart with a damaged character or a stray byte
//!    in it loses that character, not every name in the file.
//! 4. Everything else is Shift_JIS or EUC-KR ([`detect_legacy`]). That includes a file that opens
//!    with the UTF-8 mark and goes on in a legacy encoding, which editors do produce.
//!
//! Telling the two legacy encodings apart cannot be done by trying each and keeping the one that
//! decodes: about half of all Shift_JIS charts are also well-formed code page 949, because that
//! code page assigns a rare Hangul syllable to nearly every pair of bytes Shift_JIS uses for kana
//! and kanji. So both readings are tallied by what kind of character each pair of bytes would be,
//! and the more plausible text wins ([`ShiftJisReading::score`], [`EucKrReading::score`]):
//!
//! - Japanese text is made of pairs whose first byte is `0x81..=0x9F`. Read as Korean, those are
//!   syllables outside the 2350 that Korean text is ordinarily written with, or not characters at
//!   all.
//! - Korean text is made of pairs in `0xB0..=0xC8` followed by `0xA1..=0xFE`. Read as Shift_JIS,
//!   those are half-width katakana broken up by rare kanji, user-defined characters and bytes that
//!   are not characters at all.
//!
//! A file both readings find equally plausible stays Shift_JIS, which is what every chart was read
//! as before and what a chart is far more likely to be. That is the case for a file whose only
//! bytes outside ASCII are an even run of half-width katakana: nothing in it says it is not a pair
//! of Hangul syllables instead, and the keysound it names is on disk under the katakana.

use encoding_rs::{EUC_KR, Encoding, SHIFT_JIS, UTF_8, UTF_16BE, UTF_16LE};

#[cfg(test)]
mod tests;

/// The encodings a chart file is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    ShiftJis,
    EucKr,
}

impl TextEncoding {
    fn decoder(self) -> &'static Encoding {
        match self {
            TextEncoding::Utf8 => UTF_8,
            TextEncoding::Utf16Le => UTF_16LE,
            TextEncoding::Utf16Be => UTF_16BE,
            TextEncoding::ShiftJis => SHIFT_JIS,
            TextEncoding::EucKr => EUC_KR,
        }
    }
}

/// `bytes` as text in the encoding [`classify`] finds, without the byte order mark. A byte sequence
/// that is not a character in that encoding becomes U+FFFD.
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    let (encoding, body) = classify(bytes);
    let (text, _) = encoding.decoder().decode_without_bom_handling(body);
    text.into_owned()
}

/// The encoding of `bytes` and the bytes after any byte order mark. Depends on nothing but the
/// bytes.
fn classify(bytes: &[u8]) -> (TextEncoding, &[u8]) {
    let (marked, body) = match Encoding::for_bom(bytes) {
        Some((mark, length)) => (Some(mark), &bytes[length..]),
        None => (None, bytes),
    };
    let encoding = match marked {
        Some(mark) if mark == UTF_16LE => TextEncoding::Utf16Le,
        Some(mark) if mark == UTF_16BE => TextEncoding::Utf16Be,
        Some(_) if mostly_utf8(body) => TextEncoding::Utf8,
        None if std::str::from_utf8(body).is_ok() => TextEncoding::Utf8,
        _ => detect_legacy(body),
    };
    (encoding, body)
}

/// Whether bytes that came behind a UTF-8 mark are to be read as the mark says.
///
/// They are unless the byte sequences that are no UTF-8 character outnumber the characters outside
/// ASCII that are. A UTF-8 chart with a character cut short or one byte from another encoding in it
/// has a few of the first and many of the second. A legacy chart an editor put the mark in front of
/// is the other way round: its pairs of bytes almost never line up as UTF-8, and the few that do
/// are each surrounded by ones that do not.
fn mostly_utf8(bytes: &[u8]) -> bool {
    let (mut characters, mut errors) = (0usize, 0usize);
    for chunk in bytes.utf8_chunks() {
        characters += chunk.valid().chars().filter(|character| !character.is_ascii()).count();
        errors += usize::from(!chunk.invalid().is_empty());
    }
    errors <= characters
}

/// Shift_JIS or EUC-KR, for bytes that are not UTF-8.
///
/// Bytes with nothing in them that reads as ordinary Korean text are Shift_JIS without the two
/// readings being compared, whatever else is wrong with them: a damaged Shift_JIS file is still
/// better read as what it is.
fn detect_legacy(bytes: &[u8]) -> TextEncoding {
    let korean = EucKrReading::of(bytes);
    if !korean.has_ordinary_text() {
        return TextEncoding::ShiftJis;
    }
    if korean.score() > ShiftJisReading::of(bytes).score() { TextEncoding::EucKr } else { TextEncoding::ShiftJis }
}

/// What a pair of bytes read as one wide character is worth to the Shift_JIS reading: kana, a
/// common kanji or a full-width symbol. It is the weight every other one is set against. A file
/// with Japanese text in it has these throughout, and each is worth more than the two half-width
/// katakana the same two bytes could have been, so no amount of half-width text beside them tips a
/// Japanese file over.
const WIDE_WEIGHT: i64 = 4;

/// What one half-width katakana is worth. Two of them are worth one Hangul syllable, so text that
/// could be either is a tie.
const HALFWIDTH_WEIGHT: i64 = 1;

/// What a kanji outside the common set is worth: the second level of JIS X 0208 and the vendor
/// extensions. Japanese text has them, and so does Korean text read as Shift_JIS, where a third of
/// all syllables end in a byte that starts one. The pair is worth the same as the hanja it would be
/// in the other reading, so a file with nothing but one such character stays Shift_JIS.
const RARE_KANJI_WEIGHT: i64 = 1;

/// What a half-width katakana standing where Japanese cannot have it costs: a small kana, the long
/// vowel mark or a voicing mark with no katakana in front of it.
const MISPLACED_KANA_PENALTY: i64 = 4;

/// What a character from the user-defined area costs. No chart is written with those, and a tenth
/// of the syllables of Korean text read as Shift_JIS land there.
const USER_DEFINED_PENALTY: i64 = 6;

/// What a byte sequence that is no character at all costs either reading.
const ERROR_PENALTY: i64 = 16;

/// What a Hangul syllable of the common 2350, or a symbol or kana of KS X 1001, is worth to the
/// EUC-KR reading.
const KOREAN_WEIGHT: i64 = 2;

/// What a hanja is worth. Korean text has few, and half-width katakana read as EUC-KR has many.
const HANJA_WEIGHT: i64 = 1;

/// What a Hangul syllable outside the common 2350 costs. Korean text has almost none; Japanese
/// text read as EUC-KR is made of them.
const EXTENDED_HANGUL_PENALTY: i64 = 4;

/// First and last byte of the half-width katakana block, each a character on its own.
const HALFWIDTH_FIRST: u8 = 0xA1;
const HALFWIDTH_LAST: u8 = 0xDF;

/// Last of the half-width punctuation marks the block opens with. After one of them a word starts
/// again.
const HALFWIDTH_PUNCTUATION_LAST: u8 = 0xA5;

/// The half-width small kana and the long vowel mark, which follow another katakana.
const HALFWIDTH_SMALL_FIRST: u8 = 0xA7;
const HALFWIDTH_SMALL_LAST: u8 = 0xB0;

/// The half-width voicing marks, which follow the katakana they voice.
const HALFWIDTH_VOICING_FIRST: u8 = 0xDE;

/// What the first byte of a Shift_JIS pair says the pair is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftJisLead {
    Wide,
    RareKanji,
    UserDefined,
    Unassigned,
}

/// What `byte` starts in Shift_JIS, or `None` when it starts no pair.
///
/// The rows are those of the code page as browsers decode it: symbols, kana and the first level of
/// kanji up to `0x9F`, the second level from `0xE0`, the vendor extensions at `0xED`, `0xEE` and
/// `0xFA..=0xFC`, the user-defined area between them, and five rows nothing is assigned to.
fn shift_jis_lead(byte: u8) -> Option<ShiftJisLead> {
    match byte {
        0x85 | 0x86 | 0xEB | 0xEC | 0xEF => Some(ShiftJisLead::Unassigned),
        0x81..=0x9F => Some(ShiftJisLead::Wide),
        0xE0..=0xEA | 0xED | 0xEE | 0xFA..=0xFC => Some(ShiftJisLead::RareKanji),
        0xF0..=0xF9 => Some(ShiftJisLead::UserDefined),
        _ => None,
    }
}

/// Whether `byte` can be the second of a Shift_JIS pair.
fn shift_jis_trail(byte: u8) -> bool {
    matches!(byte, 0x40..=0x7E | 0x80..=0xFC)
}

/// What a byte string holds when it is read as Shift_JIS.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ShiftJisReading {
    wide: i64,
    rare_kanji: i64,
    halfwidth: i64,
    misplaced_kana: i64,
    user_defined: i64,
    errors: i64,
}

impl ShiftJisReading {
    fn of(bytes: &[u8]) -> ShiftJisReading {
        let mut reading = ShiftJisReading::default();
        let mut after_kana = false;
        let mut at = 0;
        while let Some(&byte) = bytes.get(at) {
            at += 1;
            if byte.is_ascii() {
                after_kana = false;
                continue;
            }
            if (HALFWIDTH_FIRST..=HALFWIDTH_LAST).contains(&byte) {
                let follows = (HALFWIDTH_SMALL_FIRST..=HALFWIDTH_SMALL_LAST).contains(&byte) || byte >= HALFWIDTH_VOICING_FIRST;
                reading.halfwidth += 1;
                reading.misplaced_kana += i64::from(follows && !after_kana);
                after_kana = byte > HALFWIDTH_PUNCTUATION_LAST;
                continue;
            }
            after_kana = false;
            let trail = bytes.get(at).copied();
            match (shift_jis_lead(byte), trail) {
                (Some(lead), Some(trail)) if shift_jis_trail(trail) => {
                    at += usize::from(lead != ShiftJisLead::Unassigned || !trail.is_ascii());
                    match lead {
                        ShiftJisLead::Wide => reading.wide += 1,
                        ShiftJisLead::RareKanji => reading.rare_kanji += 1,
                        ShiftJisLead::UserDefined => reading.user_defined += 1,
                        ShiftJisLead::Unassigned => reading.errors += 1,
                    }
                }
                (Some(_), Some(trail)) => {
                    at += usize::from(!trail.is_ascii());
                    reading.errors += 1;
                }
                _ => reading.errors += 1,
            }
        }
        reading
    }

    /// How plausible the bytes are as Shift_JIS text.
    fn score(&self) -> i64 {
        WIDE_WEIGHT * self.wide + RARE_KANJI_WEIGHT * self.rare_kanji + HALFWIDTH_WEIGHT * self.halfwidth
            - MISPLACED_KANA_PENALTY * self.misplaced_kana
            - USER_DEFINED_PENALTY * self.user_defined
            - ERROR_PENALTY * self.errors
    }
}

/// First and last byte that can start a pair in code page 949.
const EUC_KR_LEAD_FIRST: u8 = 0x81;
const EUC_KR_LEAD_LAST: u8 = 0xFE;

/// What a pair of bytes is in code page 949.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EucKrPair {
    Hangul,
    Symbol,
    Hanja,
    ExtendedHangul,
}

/// What `lead` and `trail` are together in code page 949, or `None` when they are no character.
///
/// A second byte from `0xA1` up is the plane KS X 1001 defines: its symbols and kana in rows
/// `0xA1..=0xAC`, the 2350 common Hangul syllables in `0xB0..=0xC8` and hanja in `0xCA..=0xFD`.
/// Everything else the code page assigns is one of the 8822 remaining syllables, which it fits into
/// the bytes KS X 1001 left unused.
fn euc_kr_pair(lead: u8, trail: u8) -> Option<EucKrPair> {
    match (lead, trail) {
        (0xB0..=0xC8, 0xA1..=0xFE) => Some(EucKrPair::Hangul),
        (0xA1..=0xAC, 0xA1..=0xFE) => Some(EucKrPair::Symbol),
        (0xCA..=0xFD, 0xA1..=0xFE) => Some(EucKrPair::Hanja),
        (0x81..=0xA0, 0xA1..=0xFE) | (0x81..=0xC6, 0x41..=0x5A | 0x61..=0x7A | 0x81..=0xA0) => Some(EucKrPair::ExtendedHangul),
        _ => None,
    }
}

/// What a byte string holds when it is read as EUC-KR (code page 949).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct EucKrReading {
    hangul: i64,
    symbols: i64,
    hanja: i64,
    extended_hangul: i64,
    errors: i64,
}

impl EucKrReading {
    fn of(bytes: &[u8]) -> EucKrReading {
        let mut reading = EucKrReading::default();
        let mut at = 0;
        while let Some(&byte) = bytes.get(at) {
            at += 1;
            if byte.is_ascii() {
                continue;
            }
            let trail = bytes.get(at).copied().filter(|_| (EUC_KR_LEAD_FIRST..=EUC_KR_LEAD_LAST).contains(&byte));
            let pair = trail.and_then(|trail| euc_kr_pair(byte, trail));
            at += usize::from(pair.is_some() || trail.is_some_and(|trail| !trail.is_ascii()));
            match pair {
                Some(EucKrPair::Hangul) => reading.hangul += 1,
                Some(EucKrPair::Symbol) => reading.symbols += 1,
                Some(EucKrPair::Hanja) => reading.hanja += 1,
                Some(EucKrPair::ExtendedHangul) => reading.extended_hangul += 1,
                None => reading.errors += 1,
            }
        }
        reading
    }

    /// Whether anything in the bytes reads as what Korean text is ordinarily written with.
    fn has_ordinary_text(&self) -> bool {
        self.hangul + self.symbols + self.hanja > 0
    }

    /// How plausible the bytes are as EUC-KR text.
    fn score(&self) -> i64 {
        KOREAN_WEIGHT * (self.hangul + self.symbols) + HANJA_WEIGHT * self.hanja - EXTENDED_HANGUL_PENALTY * self.extended_hangul - ERROR_PENALTY * self.errors
    }
}
