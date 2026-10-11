use encoding_rs::{EUC_KR, Encoding, SHIFT_JIS, UTF_16BE, UTF_16LE};

use super::{TextEncoding, classify, decode_text};
use crate::parse;

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF16BE_BOM: [u8; 2] = [0xFE, 0xFF];

/// Lines a Japanese chart would carry, one per kind of text that has to stay Shift_JIS.
const JAPANESE_LINES: [&str; 12] = [
    "#TITLE 約束",
    "#TITLE ひらがなのうた",
    "#TITLE カタカナ・ソング",
    "#TITLE 星の歌 [ANOTHER]",
    "#ARTIST 作曲者 / obj:譜面職人",
    "#GENRE ドラムンベース",
    "#TITLE ＡＢＣ ☆ １２３",
    "#TITLE 翔",
    "#ARTIST 髙橋",
    "#SUBTITLE ～終わりの始まり～",
    "#COMMENT \"ｷﾀ━━━(ﾟ∀ﾟ)━━━!!\"",
    "#WAV0A ドラム_01.wav",
];

/// Lines a Korean chart would carry.
const KOREAN_LINES: [&str; 8] = [
    "#TITLE 별의 노래",
    "#TITLE 가나다",
    "#ARTIST 작곡가 / obj: 채보 제작자",
    "#GENRE 드럼 앤 베이스",
    "#SUBTITLE [어나더]",
    "#TITLE 봄날 ★ 여름밤",
    "#COMMENT 즐겁게 플레이해 주세요 ㅋㅋ",
    "#WAV0A 드럼_01.wav",
];

fn detect_encoding(bytes: &[u8]) -> TextEncoding {
    classify(bytes).0
}

fn encoded(encoding: &'static Encoding, text: &str) -> Vec<u8> {
    let (bytes, _, unmappable) = encoding.encode(text);
    assert!(!unmappable, "{text:?} has a character {} cannot write", encoding.name());
    bytes.into_owned()
}

#[test]
fn a_korean_chart_in_euc_kr_reads_as_korean() {
    let chart =
        encoded(EUC_KR, "#TITLE 별의 노래\r\n#SUBTITLE [어나더]\r\n#ARTIST 작곡가 / obj: 채보 제작자\r\n#GENRE 발라드\r\n#WAV01 피아노.wav\r\n#00111:01\r\n");
    let source = parse(&chart);
    assert_eq!(source.headers.title, "별의 노래");
    assert_eq!(source.headers.subtitle, "[어나더]");
    assert_eq!(source.headers.artist, "작곡가 / obj: 채보 제작자");
    assert_eq!(source.headers.genre, "발라드");
    assert_eq!(source.wav.get(&1).map(String::as_str), Some("피아노.wav"));
}

#[test]
fn korean_keysound_names_alone_decide_the_encoding() {
    let chart = encoded(
        EUC_KR,
        "#TITLE Scale\r\n#WAV01 3도.wav\r\n#WAV02 3레.wav\r\n#WAV03 3미.wav\r\n#WAV04 3파.wav\r\n#WAV05 3솔.wav\r\n#WAV06 3라.wav\r\n#WAV07 3시.wav\r\n",
    );
    let source = parse(&chart);
    assert_eq!(source.wav.get(&1).map(String::as_str), Some("3도.wav"));
    assert_eq!(source.wav.get(&6).map(String::as_str), Some("3라.wav"));
    assert_eq!(source.wav.get(&7).map(String::as_str), Some("3시.wav"));
}

#[test]
fn japanese_written_through_the_korean_code_page_reads_as_japanese() {
    let chart = encoded(EUC_KR, "#TITLE 夜明けの歌\r\n#ARTIST 星の少女たち\r\n");
    let source = parse(&chart);
    assert_eq!(source.headers.title, "夜明けの歌");
    assert_eq!(source.headers.artist, "星の少女たち");
}

#[test]
fn a_utf8_mark_in_front_of_shift_jis_text_does_not_hide_it() {
    let mut chart = UTF8_BOM.to_vec();
    chart.extend(encoded(SHIFT_JIS, "#ARTIST wa. / 約束の人\r\n#TITLE 永遠の砂時計 <秒->\r\n"));
    let source = parse(&chart);
    assert_eq!(source.headers.title, "永遠の砂時計 <秒->");
    assert_eq!(source.headers.artist, "wa. / 約束の人");
}

#[test]
fn a_utf8_mark_in_front_of_euc_kr_text_does_not_hide_it() {
    let mut chart = UTF8_BOM.to_vec();
    chart.extend(encoded(EUC_KR, "#TITLE 별의 노래\r\n#ARTIST 작곡가\r\n"));
    assert_eq!(parse(&chart).headers.title, "별의 노래");
}

/// A marked UTF-8 chart with one damaged character is still that chart: the mark is given up only
/// when the text after it is mostly not UTF-8, since the keysound names are read with the title.
#[test]
fn a_marked_utf8_chart_with_one_damaged_character_is_still_utf8() {
    let text = "#TITLE 永遠の砂時計はどこまでも続く物語のはじまり\r\n#WAV01 キック.wav\r\n#COMMENT 終";
    let mut cut_short = UTF8_BOM.to_vec();
    cut_short.extend_from_slice(text.as_bytes());
    cut_short.truncate(cut_short.len() - 1);
    assert!(std::str::from_utf8(&cut_short[UTF8_BOM.len()..]).is_err(), "the sample ends in part of a character");
    assert_eq!(detect_encoding(&cut_short), TextEncoding::Utf8);
    let source = parse(&cut_short);
    assert_eq!(source.headers.title, "永遠の砂時計はどこまでも続く物語のはじまり");
    assert_eq!(source.wav.get(&1).map(String::as_str), Some("キック.wav"));

    let mut stray_byte = UTF8_BOM.to_vec();
    stray_byte.extend_from_slice(b"#TITLE Caf\xE9 del Mar ");
    stray_byte.extend_from_slice("永遠\r\n#WAV01 キック.wav\r\n".as_bytes());
    assert_eq!(detect_encoding(&stray_byte), TextEncoding::Utf8);
    let source = parse(&stray_byte);
    assert_eq!(source.headers.title, "Caf\u{fffd} del Mar 永遠", "the one byte that is no character is the only thing lost");
    assert_eq!(source.wav.get(&1).map(String::as_str), Some("キック.wav"));
}

#[test]
fn plain_ascii_and_utf8_are_utf8_with_or_without_the_mark() {
    assert_eq!(detect_encoding(b""), TextEncoding::Utf8);
    assert_eq!(detect_encoding(b"#TITLE Plain\r\n#00111:0101\r\n"), TextEncoding::Utf8);
    let text = "#TITLE 별의 노래 / 星の歌\r\n";
    assert_eq!(detect_encoding(text.as_bytes()), TextEncoding::Utf8);
    assert_eq!(decode_text(text.as_bytes()), text);
    let mut marked = UTF8_BOM.to_vec();
    marked.extend_from_slice(text.as_bytes());
    assert_eq!(detect_encoding(&marked), TextEncoding::Utf8);
    assert_eq!(decode_text(&marked), text, "the mark is not part of the text");
}

#[test]
fn a_utf16_mark_is_believed() {
    let text = "#TITLE 별의 노래 / 星の歌\r\n";
    let mut little = UTF16LE_BOM.to_vec();
    little.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    let mut big = UTF16BE_BOM.to_vec();
    big.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
    assert_eq!(detect_encoding(&little), TextEncoding::Utf16Le);
    assert_eq!(detect_encoding(&big), TextEncoding::Utf16Be);
    assert_eq!(decode_text(&little), text);
    assert_eq!(decode_text(&big), text);
    assert_eq!(UTF_16LE.decode(&little).0, text);
    assert_eq!(UTF_16BE.decode(&big).0, text);
}

#[test]
fn every_kind_of_japanese_line_stays_shift_jis_on_its_own() {
    for line in JAPANESE_LINES {
        let bytes = encoded(SHIFT_JIS, &format!("{line}\r\n"));
        assert_eq!(detect_encoding(&bytes), TextEncoding::ShiftJis, "{line}");
        assert_eq!(decode_text(&bytes), format!("{line}\r\n"), "{line}");
    }
}

#[test]
fn a_japanese_chart_decodes_exactly_as_shift_jis_alone_decodes_it() {
    let chart = encoded(SHIFT_JIS, &JAPANESE_LINES.join("\r\n"));
    assert_eq!(detect_encoding(&chart), TextEncoding::ShiftJis);
    assert_eq!(decode_text(&chart), SHIFT_JIS.decode_without_bom_handling(&chart).0);
}

#[test]
fn every_kind_of_korean_line_is_euc_kr_on_its_own() {
    for line in KOREAN_LINES {
        let bytes = encoded(EUC_KR, &format!("{line}\r\n"));
        assert_eq!(detect_encoding(&bytes), TextEncoding::EucKr, "{line}");
        assert_eq!(decode_text(&bytes), format!("{line}\r\n"), "{line}");
    }
}

#[test]
fn japanese_text_that_is_also_well_formed_korean_stays_japanese() {
    let chart = encoded(SHIFT_JIS, "#TITLE りりくろ\r\n#ARTIST ねここうもり\r\n");
    let (as_korean, korean_errors) = EUC_KR.decode_without_bom_handling(&chart);
    assert!(!korean_errors, "the sample has to decode both ways to be a test of the choice: {as_korean}");
    assert_eq!(detect_encoding(&chart), TextEncoding::ShiftJis);
    assert_eq!(parse(&chart).headers.title, "りりくろ");
}

#[test]
fn an_even_run_of_halfwidth_katakana_stays_shift_jis() {
    let chart = encoded(SHIFT_JIS, "#ARTIST someone obj:ｻﾝｼﾞﾊﾝ!!\r\n#WAV4F ﾀｸ.wav\r\n#WAV4G ﾁｸ.wav\r\n");
    let (as_korean, korean_errors) = EUC_KR.decode_without_bom_handling(&chart);
    assert!(!korean_errors, "the sample has to decode both ways to be a test of the choice: {as_korean}");
    assert_eq!(detect_encoding(&chart), TextEncoding::ShiftJis);
    let source = parse(&chart);
    assert_eq!(source.headers.artist, "someone obj:ｻﾝｼﾞﾊﾝ!!");
    assert_eq!(source.wav.get(&crate::parse_pair(b'4', b'F', 36)).map(String::as_str), Some("ﾀｸ.wav"));
}

#[test]
fn halfwidth_katakana_beside_wide_text_does_not_outweigh_it() {
    let chart = encoded(SHIFT_JIS, "#TITLE 歌\r\n#COMMENT ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖ\r\n");
    assert_eq!(detect_encoding(&chart), TextEncoding::ShiftJis);
}

#[test]
fn a_damaged_japanese_chart_is_still_japanese() {
    let mut chart = encoded(SHIFT_JIS, "#TITLE りりくろ\r\n#ARTIST ねここうもり");
    chart.truncate(chart.len() - 1);
    chart.extend_from_slice(b"\r\n");
    assert!(SHIFT_JIS.decode_without_bom_handling(&chart).1, "the sample ends in half a character");
    assert_eq!(detect_encoding(&chart), TextEncoding::ShiftJis);
    assert_eq!(parse(&chart).headers.title, "りりくろ");
}

#[test]
fn one_euc_kr_line_in_a_japanese_chart_does_not_turn_the_chart() {
    let mut chart = encoded(SHIFT_JIS, "#GENRE ドラムンベース\r\n#ARTIST 作曲者 / obj:譜面職人\r\n#TITLE song ");
    chart.extend(encoded(EUC_KR, "별"));
    chart.extend_from_slice(b"\r\n");
    assert_eq!(detect_encoding(&chart), TextEncoding::ShiftJis);
    assert_eq!(parse(&chart).headers.genre, "ドラムンベース");
}

#[test]
fn bytes_with_nothing_korean_in_them_are_shift_jis_however_damaged() {
    for bytes in [&[0xFFu8][..], &[0x80], &[0xA0, 0xA0], &[0x81], &[0xE0], &[0xB0], &[0x82, 0xA0, 0xFF], &[0xEF, 0xBB, 0xBF, 0xFF]] {
        assert_eq!(detect_encoding(bytes), TextEncoding::ShiftJis, "{bytes:02X?}");
        assert!(!decode_text(bytes).is_empty(), "{bytes:02X?}");
    }
}

#[test]
fn every_two_byte_sequence_gets_an_answer() {
    for lead in 0x80..=0xFFu8 {
        for trail in 0x00..=0xFFu8 {
            let bytes = [b'#', lead, trail, b'\n', lead];
            let encoding = detect_encoding(&bytes);
            assert!(matches!(encoding, TextEncoding::Utf8 | TextEncoding::ShiftJis | TextEncoding::EucKr), "{bytes:02X?}");
            assert!(decode_text(&bytes).starts_with('#'), "{bytes:02X?}");
        }
    }
}

#[test]
fn the_answer_depends_on_nothing_but_the_bytes() {
    let chart = encoded(EUC_KR, &KOREAN_LINES.join("\r\n"));
    assert_eq!(detect_encoding(&chart), TextEncoding::EucKr);
    assert_eq!(detect_encoding(&chart), detect_encoding(&chart.clone()));
    assert_eq!(decode_text(&chart), KOREAN_LINES.join("\r\n"));
}
