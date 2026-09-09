use super::*;

#[test]
fn base36_digits() {
    assert_eq!(parse_pair(b'0', b'0', 36), 0);
    assert_eq!(parse_pair(b'1', b'1', 36), 37);
    assert_eq!(parse_pair(b'0', b'1', 36), 1);
    assert_eq!(parse_pair(b'Z', b'Z', 36), 35 * 36 + 35);
    assert_eq!(parse_pair(b'A', b'A', 36), parse_pair(b'a', b'a', 36));
    assert_eq!(parse_pair(b'S', b'C', 36), 1020);
}

#[test]
fn base62_lowercase() {
    assert_eq!(parse_pair(b'a', b'a', 62), 36 * 62 + 36);
    assert_eq!(parse_pair(b'A', b'A', 62), 10 * 62 + 10);
}

#[test]
fn md5_sha256_known_answer() {
    let s = parse(b"abc");
    assert_eq!(s.md5, "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(s.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

const MINIMAL: &[u8] = b"#PLAYER 1\r\n#TITLE Test Chart\r\n#ARTIST rbms\r\n#BPM 150\r\n#PLAYLEVEL 5\r\n#RANK 2\r\n#TOTAL 200\r\n#WAV01 kick.wav\r\n#WAV02 snare.wav\r\n#00111:0102\r\n#00211:01000200\r\n#00102:0.75\r\n";

#[test]
fn headers_and_wav() {
    let s = parse(MINIMAL);
    assert_eq!(s.headers.title, "Test Chart");
    assert_eq!(s.headers.artist, "rbms");
    assert_eq!(s.headers.init_bpm, 150.0);
    assert_eq!(s.headers.play_level, "5");
    assert_eq!(s.headers.rank, 2);
    assert_eq!(s.headers.total, Some(200.0));
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("kick.wav"));
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("snare.wav"));
}

#[test]
fn maker_genre_subtitle_difficulty() {
    let s = parse(b"#GENRE J-CORE\r\n#SUBTITLE [ANOTHER]\r\n#MAKER notecharter\r\n#DIFFICULTY 4\r\n");
    assert_eq!(s.headers.genre, "J-CORE");
    assert_eq!(s.headers.subtitle, "[ANOTHER]");
    assert_eq!(s.headers.maker, "notecharter");
    assert_eq!(s.headers.difficulty, 4);
}

#[test]
fn stagefile_banner_preview() {
    let s = parse(b"#STAGEFILE title.png\r\n#BANNER banner.png\r\n#PREVIEW preview.ogg\r\n");
    assert_eq!(s.headers.stagefile, "title.png");
    assert_eq!(s.headers.banner, "banner.png");
    assert_eq!(s.headers.preview, "preview.ogg");
}

#[test]
fn data_field_objects_and_positions() {
    let s = parse(MINIMAL);
    let m1 = s.measures.get(&1).unwrap();
    assert_eq!(m1.rate, 0.75);
    let ch11 = m1.channels.iter().find(|c| c.channel == 37).unwrap();
    assert_eq!(ch11.objects.len(), 2);
    assert_eq!(ch11.objects[0].value(36), 1);
    assert_eq!(ch11.objects[0].pos(), 0.0);
    assert_eq!(ch11.objects[1].value(36), 2);
    assert_eq!(ch11.objects[1].pos(), 0.5);

    let m2 = s.measures.get(&2).unwrap();
    let ch = &m2.channels.iter().find(|c| c.channel == 37).unwrap().objects;
    assert_eq!(ch.len(), 2);
    assert_eq!(ch[0].pos(), 0.0);
    assert_eq!(ch[1].pos(), 0.5);
    assert_eq!(ch[1].value(36), 2);
}

#[test]
fn channel_03_bpm_hex_preserved() {
    let s = parse(b"#00103:FF\r\n");
    let m = s.measures.get(&1).unwrap();
    let ch = m.channels.iter().find(|c| c.channel == 3).unwrap();
    assert_eq!(ch.objects[0].value16(), 255);
}

#[test]
fn random_setrandom_selects_branch() {
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#WAV01 one.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV01 two.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("two.wav"));
}

#[test]
fn random_else_branch() {
    let chart = b"#SETRANDOM 3\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSE\r\n#WAV01 b.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("b.wav"));
}

#[test]
fn random_is_deterministic_for_seed() {
    let chart = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV01 b.wav\r\n#ENDIF\r\n#IF 3\r\n#WAV01 c.wav\r\n#ENDIF\r\n#IF 4\r\n#WAV01 d.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let a = parse_with(chart, ParseOptions { random_seed: 42 });
    let b = parse_with(chart, ParseOptions { random_seed: 42 });
    assert_eq!(a.wav.get(&1), b.wav.get(&1));
    assert!(a.wav.get(&1).is_some());
}

#[test]
fn data_field_pairs_positionally_invalid_slot_is_empty() {
    let s = parse(b"#00111:01 02\r\n");
    let ch = &s.measures.get(&1).unwrap().channels.iter().find(|c| c.channel == 37).unwrap().objects;
    assert_eq!(ch.len(), 1);
    assert_eq!(ch[0].value(36), 1);
    assert_eq!(ch[0].num, 0);
    assert_eq!(ch[0].den, 2);
}

#[test]
fn base62_header_ids_use_base62() {
    let s = parse(b"#BASE 62\r\n#WAVa0 x.wav\r\n");
    assert_eq!(s.base, 62);
    assert_eq!(s.wav.get(&(36 * 62)).map(String::as_str), Some("x.wav"));
    assert!(s.wav.get(&360).is_none());
}

#[test]
fn shift_jis_title_decodes() {
    let mut bytes = b"#TITLE ".to_vec();
    bytes.extend_from_slice(&[0x96, 0xF1, 0x91, 0xA9]);
    bytes.extend_from_slice(b"\r\n");
    let s = parse(&bytes);
    assert_eq!(s.headers.title, "約束");
}

#[test]
fn stop_and_scroll_defs() {
    let s = parse(b"#STOP01 192\r\n#SCROLL01 2.0\r\n#BPM01 200\r\n");
    assert_eq!(s.stop_def.get(&1).copied(), Some(192.0));
    assert_eq!(s.scroll_def.get(&1).copied(), Some(2.0));
    assert_eq!(s.bpm_def.get(&1).copied(), Some(200.0));
}

// ----------------------------------------------------------------------------
// base::digit / parse_pair edge cases
// ----------------------------------------------------------------------------

#[test]
fn digit_decimal_digits() {
    for c in b'0'..=b'9' {
        assert_eq!(digit(c, 36), Some((c - b'0') as u32));
        assert_eq!(digit(c, 62), Some((c - b'0') as u32));
    }
}

#[test]
fn digit_uppercase_letters_base36() {
    assert_eq!(digit(b'A', 36), Some(10));
    assert_eq!(digit(b'Z', 36), Some(35));
}

#[test]
fn digit_z_boundary_base36_vs_base62() {
    // In base36, lowercase maps the same as uppercase (z -> 35), and 35 < 36.
    assert_eq!(digit(b'z', 36), Some(35));
    // In base62, lowercase is offset by 36 (z -> 61), and 61 < 62.
    assert_eq!(digit(b'z', 62), Some(61));
    assert_eq!(digit(b'a', 62), Some(36));
}

#[test]
fn digit_uppercase_in_base62_is_below_36() {
    // Uppercase letters never use the base-62 lowercase branch.
    assert_eq!(digit(b'A', 62), Some(10));
    assert_eq!(digit(b'Z', 62), Some(35));
}

#[test]
fn digit_out_of_range_returns_none() {
    // In a small base, a digit whose numeric value is >= base is rejected.
    assert_eq!(digit(b'9', 8), None); // 9 >= 8
    assert_eq!(digit(b'A', 10), None); // 10 >= 10
    assert_eq!(digit(b'Z', 16), None); // 35 >= 16
    assert_eq!(digit(b'G', 16), None); // 16 >= 16
    assert_eq!(digit(b'F', 16), Some(15));
}

#[test]
fn digit_invalid_chars_return_none() {
    for c in [b'-', b'+', b' ', b':', b'.', b'/', b'@', b'[', b'{', b'\0', 0x80, 0xFF] {
        assert_eq!(digit(c, 36), None, "char {:#x} should be None", c);
        assert_eq!(digit(c, 62), None, "char {:#x} should be None", c);
    }
}

#[test]
fn parse_pair_invalid_chars_treated_as_zero() {
    // digit() returns None for '-', unwrap_or(0) makes it 0.
    assert_eq!(parse_pair(b'-', b'-', 36), 0);
    assert_eq!(parse_pair(b'-', b'5', 36), 5);
    assert_eq!(parse_pair(b'5', b'-', 36), 5 * 36);
}

#[test]
fn parse_pair_uppercase_lowercase_equal_base36() {
    for &(u, l) in &[(b'A', b'a'), (b'M', b'm'), (b'Z', b'z')] {
        assert_eq!(parse_pair(u, u, 36), parse_pair(l, l, 36));
    }
}

#[test]
fn parse_pair_uppercase_lowercase_differ_base62() {
    // Distinct in base62.
    assert_ne!(parse_pair(b'A', b'A', 62), parse_pair(b'a', b'a', 62));
    assert_eq!(parse_pair(b'z', b'z', 62), 61 * 62 + 61);
}

#[test]
fn parse_pair_max_base62() {
    // 'z' is the largest digit in base62 -> 61.
    assert_eq!(parse_pair(b'z', b'z', 62), 61 * 62 + 61);
    assert_eq!(parse_pair(b'Z', b'9', 36), 35 * 36 + 9);
}

// ----------------------------------------------------------------------------
// data line edge cases: measure indices, empty/odd data, skipped slots
// ----------------------------------------------------------------------------

#[test]
fn measure_index_000_parsed() {
    let s = parse(b"#00011:0102\r\n");
    assert!(s.measures.contains_key(&0));
    let ch = &s.measures.get(&0).unwrap().channels[0];
    assert_eq!(ch.channel, 37); // '11' base36 = 37
    assert_eq!(ch.objects.len(), 2);
}

#[test]
fn measure_index_999_parsed() {
    let s = parse(b"#99911:0102\r\n");
    assert!(s.measures.contains_key(&999));
    assert_eq!(s.measures.get(&999).unwrap().channels[0].objects.len(), 2);
}

#[test]
fn empty_data_field_creates_measure_no_objects() {
    // len == 0 path: measure entry is created via or_default but no channel pushed.
    let s = parse(b"#00111:\r\n");
    assert!(s.measures.contains_key(&1));
    assert!(s.measures.get(&1).unwrap().channels.is_empty());
}

#[test]
fn single_char_data_field_yields_no_objects() {
    // One char -> len = 1/2 = 0 -> early return, no objects.
    let s = parse(b"#00111:5\r\n");
    assert!(s.measures.get(&1).unwrap().channels.is_empty());
}

#[test]
fn odd_length_data_field_drops_trailing_char() {
    // 5 chars -> len = 2 pairs; trailing char ignored.
    let s = parse(b"#00111:01023\r\n");
    let ch = &s.measures.get(&1).unwrap().channels[0].objects;
    assert_eq!(ch.len(), 2);
    assert_eq!(ch[0].value(36), 1);
    assert_eq!(ch[1].value(36), 2);
    assert_eq!(ch[1].den, 2); // den from pair count, not char count
}

#[test]
fn skipped_00_slots_preserve_denominator() {
    // "00" slots are skipped but den reflects total slot count.
    let s = parse(b"#00111:01000200\r\n"); // 4 slots: 01, 00, 02, 00
    let ch = &s.measures.get(&1).unwrap().channels[0].objects;
    assert_eq!(ch.len(), 2);
    assert_eq!(ch[0].num, 0);
    assert_eq!(ch[0].den, 4);
    assert_eq!(ch[1].num, 2);
    assert_eq!(ch[1].den, 4);
    assert_eq!(ch[1].pos(), 0.5);
    assert_eq!(ch[1].value(36), 2);
}

#[test]
fn all_zero_slots_yield_no_channel() {
    let s = parse(b"#00111:00000000\r\n");
    assert!(s.measures.get(&1).unwrap().channels.is_empty());
}

#[test]
fn invalid_digit_slot_skipped_but_den_preserved() {
    // '!!' is not a valid base36 pair -> slot skipped, but den = 2.
    let s = parse(b"#00111:!!05\r\n");
    let ch = &s.measures.get(&1).unwrap().channels[0].objects;
    assert_eq!(ch.len(), 1);
    assert_eq!(ch[0].num, 1);
    assert_eq!(ch[0].den, 2);
    assert_eq!(ch[0].value(36), 5);
}

#[test]
fn non_ascii_char_in_slot_skipped() {
    // A multibyte char makes a slot non-ASCII -> skipped.
    let mut bytes = "#00111:".as_bytes().to_vec();
    bytes.extend_from_slice("あ05".as_bytes()); // 'あ' is one char, then '0','5'
    bytes.extend_from_slice(b"\r\n");
    let s = parse(&bytes);
    // chars: ['あ','0','5'] -> len = 1 pair: ('あ','0') non-ascii -> slot skipped.
    // No channel pushed since the resulting object list is empty.
    assert!(s.measures.get(&1).unwrap().channels.is_empty());
}

#[test]
fn channel_00_is_valid_channel_zero() {
    // channel pair '00' base36 -> 0; not special-cased, objects stored.
    let s = parse(b"#00100:0102\r\n");
    let ch = s.measures.get(&1).unwrap().channels.iter().find(|c| c.channel == 0);
    assert!(ch.is_some());
    assert_eq!(ch.unwrap().objects.len(), 2);
}

#[test]
fn channel_zz_is_max_base36_channel() {
    let s = parse(b"#001ZZ:0102\r\n");
    let ch = s.measures.get(&1).unwrap().channels.iter().find(|c| c.channel == 35 * 36 + 35);
    assert!(ch.is_some());
}

#[test]
fn multiple_data_lines_same_channel_push_separately() {
    // Two data lines, same measure+channel -> two ChannelData entries (no merge).
    let s = parse(b"#00111:0102\r\n#00111:0304\r\n");
    let count = s.measures.get(&1).unwrap().channels.iter().filter(|c| c.channel == 37).count();
    assert_eq!(count, 2);
}

// ----------------------------------------------------------------------------
// channel 02 (rate) and 03 (hex BPM) specifics
// ----------------------------------------------------------------------------

#[test]
fn channel_02_rate_parsed_as_f64_no_objects() {
    let s = parse(b"#00102:0.5\r\n");
    let m = s.measures.get(&1).unwrap();
    assert_eq!(m.rate, 0.5);
    // channel 02 produces no Obj entries.
    assert!(m.channels.iter().all(|c| c.channel != 2));
}

#[test]
fn channel_02_integer_rate() {
    let s = parse(b"#00102:2\r\n");
    assert_eq!(s.measures.get(&1).unwrap().rate, 2.0);
}

#[test]
fn channel_02_invalid_rate_keeps_default() {
    // Non-numeric -> parse fails -> rate stays default 1.0.
    let s = parse(b"#00102:abc\r\n");
    assert_eq!(s.measures.get(&1).unwrap().rate, 1.0);
}

#[test]
fn channel_02_default_rate_is_one() {
    let s = parse(b"#00111:0102\r\n");
    assert_eq!(s.measures.get(&1).unwrap().rate, 1.0);
}

#[test]
fn channel_03_hex_pairs_value16() {
    // Channel 03 carries hex BPM bytes; value16 reads them as base16.
    let s = parse(b"#00103:0A10FF\r\n");
    let ch = &s.measures.get(&1).unwrap().channels.iter().find(|c| c.channel == 3).unwrap().objects;
    assert_eq!(ch.len(), 3);
    assert_eq!(ch[0].value16(), 10); // 0A
    assert_eq!(ch[1].value16(), 16); // 10
    assert_eq!(ch[2].value16(), 255); // FF
}

#[test]
fn value_vs_value16_differ_for_same_token() {
    // 'FF' as base36 channel-agnostic value vs base16.
    let s = parse(b"#00103:FF\r\n");
    let o = &s.measures.get(&1).unwrap().channels[0].objects[0];
    assert_eq!(o.value16(), 255);
    assert_eq!(o.value(36), 15 * 36 + 15);
}

// ----------------------------------------------------------------------------
// is_data_line boundary: header lines that look almost like data
// ----------------------------------------------------------------------------

#[test]
fn line_without_colon_is_header_not_data() {
    // "00111 foo" has digits but no ':' at index 5 -> header line path.
    // It is not a recognized header, so it is silently ignored (no measure made).
    let s = parse(b"#00111 0102\r\n");
    assert!(s.measures.is_empty());
}

#[test]
fn short_line_not_data() {
    // body "001:1" len 5 < 6 -> not data.
    let s = parse(b"#001:1\r\n");
    assert!(s.measures.is_empty());
}

#[test]
fn non_digit_measure_prefix_is_not_data() {
    // "0X111:..." -> b[1] not a digit -> header path, ignored.
    let s = parse(b"#0X111:0102\r\n");
    assert!(s.measures.is_empty());
}

// ----------------------------------------------------------------------------
// header parsing edge cases
// ----------------------------------------------------------------------------

#[test]
fn header_case_insensitive() {
    let s = parse(b"#title Lower\r\n#ArTiSt Mixed\r\n");
    assert_eq!(s.headers.title, "Lower");
    assert_eq!(s.headers.artist, "Mixed");
}

#[test]
fn header_player_invalid_falls_back_to_default() {
    let s = parse(b"#PLAYER notanumber\r\n");
    assert_eq!(s.headers.player, 1);
}

#[test]
fn header_rank_invalid_falls_back_to_default() {
    let s = parse(b"#RANK xyz\r\n");
    assert_eq!(s.headers.rank, 3);
}

#[test]
fn header_bpm_invalid_falls_back_to_default() {
    let s = parse(b"#BPM notanumber\r\n");
    assert_eq!(s.headers.init_bpm, 130.0);
}

#[test]
fn header_total_invalid_is_none() {
    let s = parse(b"#TOTAL notanumber\r\n");
    assert_eq!(s.headers.total, None);
}

#[test]
fn header_total_valid_is_some() {
    let s = parse(b"#TOTAL 260.5\r\n");
    assert_eq!(s.headers.total, Some(260.5));
}

#[test]
fn header_lnobj_parsed_with_base() {
    let s = parse(b"#LNOBJ ZZ\r\n");
    assert_eq!(s.headers.lnobj, Some(35 * 36 + 35));
}

#[test]
fn header_lnobj_too_short_ignored() {
    // Single char rest -> t.len() < 2 -> lnobj stays None.
    let s = parse(b"#LNOBJ Z\r\n");
    assert_eq!(s.headers.lnobj, None);
}

#[test]
fn header_lntype_lnmode_defaults_and_overrides() {
    let d = parse(b"#TITLE x\r\n");
    assert_eq!(d.headers.lntype, 1);
    assert_eq!(d.headers.lnmode, 0);
    let s = parse(b"#LNTYPE 2\r\n#LNMODE 3\r\n");
    assert_eq!(s.headers.lntype, 2);
    assert_eq!(s.headers.lnmode, 3);
}

#[test]
fn header_no_value_yields_empty_string() {
    // "#TITLE" with no whitespace -> rest is "".
    let s = parse(b"#TITLE\r\n");
    assert_eq!(s.headers.title, "");
}

#[test]
fn header_value_trimmed() {
    let s = parse(b"#TITLE    Spaced Out   \r\n");
    assert_eq!(s.headers.title, "Spaced Out");
}

#[test]
fn unknown_header_ignored() {
    let s = parse(b"#WHATEVER something\r\n");
    let d = Headers::default();
    assert_eq!(s.headers.title, d.title);
    assert_eq!(s.headers.player, d.player);
    assert_eq!(s.headers.init_bpm, d.init_bpm);
    assert!(s.wav.is_empty());
}

#[test]
fn wav_overwrite_last_wins() {
    let s = parse(b"#WAV01 first.wav\r\n#WAV01 second.wav\r\n");
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("second.wav"));
}

#[test]
fn bpm_def_invalid_value_not_inserted() {
    let s = parse(b"#BPM01 notanumber\r\n");
    assert!(s.bpm_def.get(&1).is_none());
}

#[test]
fn lines_not_starting_with_hash_ignored() {
    let s = parse(b"this is a comment\r\nTITLE not a header\r\n#TITLE real\r\n");
    assert_eq!(s.headers.title, "real");
}

#[test]
fn base_default_is_36() {
    let s = parse(b"#TITLE x\r\n");
    assert_eq!(s.base, 36);
}

#[test]
fn base_62_via_inline_base_header() {
    // The pre-scan also sets base=62; header parse confirms.
    let s = parse(b"#BASE 62\r\n");
    assert_eq!(s.base, 62);
}

// ----------------------------------------------------------------------------
// decode_text: BOM, valid UTF-8, Shift-JIS fallback, determinism
// ----------------------------------------------------------------------------

#[test]
fn utf8_bom_stripped() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"#TITLE BomTitle\r\n");
    let s = parse(&bytes);
    assert_eq!(s.headers.title, "BomTitle");
}

#[test]
fn plain_utf8_multibyte_title() {
    let bytes = "#TITLE 日本語\r\n".as_bytes();
    let s = parse(bytes);
    assert_eq!(s.headers.title, "日本語");
}

#[test]
fn invalid_utf8_falls_back_to_shift_jis_no_panic() {
    // 0x96 0xF1 0x91 0xA9 is "約束" in Shift-JIS and invalid UTF-8.
    let mut bytes = b"#TITLE ".to_vec();
    bytes.extend_from_slice(&[0x96, 0xF1, 0x91, 0xA9]);
    bytes.extend_from_slice(b"\r\n");
    let s = parse(&bytes);
    assert_eq!(s.headers.title, "約束");
}

#[test]
fn lone_invalid_byte_is_deterministic() {
    // 0xFF alone is invalid UTF-8 and triggers Shift-JIS decode; must not panic and
    // must be deterministic across calls.
    let bytes = vec![0xFFu8];
    let a = parse(&bytes);
    let b = parse(&bytes);
    assert_eq!(a.md5, b.md5);
    assert_eq!(a.headers.title, b.headers.title);
    assert!(a.measures.is_empty());
}

#[test]
fn bom_changes_hash_but_not_decoded_content() {
    // The BOM is part of the bytes hashed, but stripped before decoding.
    let plain = b"#TITLE Hi\r\n".to_vec();
    let mut with_bom = vec![0xEF, 0xBB, 0xBF];
    with_bom.extend_from_slice(&plain);
    let a = parse(&plain);
    let b = parse(&with_bom);
    assert_ne!(a.md5, b.md5);
    assert_eq!(a.headers.title, b.headers.title);
}

#[test]
fn lf_only_line_endings_handled() {
    let s = parse(b"#TITLE LfOnly\n#ARTIST NoCarriage\n");
    assert_eq!(s.headers.title, "LfOnly");
    assert_eq!(s.headers.artist, "NoCarriage");
}

#[test]
fn empty_input_yields_defaults() {
    let s = parse(b"");
    let d = Headers::default();
    assert_eq!(s.headers.player, d.player);
    assert_eq!(s.headers.rank, d.rank);
    assert_eq!(s.headers.init_bpm, d.init_bpm);
    assert_eq!(s.headers.total, d.total);
    assert!(s.measures.is_empty());
    assert!(s.wav.is_empty());
    assert_eq!(s.base, 36);
    // MD5/SHA-256 of empty input are well-known.
    assert_eq!(s.md5, "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(s.sha256, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

// ----------------------------------------------------------------------------
// MD5 / SHA-256 known answers and determinism
// ----------------------------------------------------------------------------

#[test]
fn md5_sha256_determinism() {
    let a = parse(MINIMAL);
    let b = parse(MINIMAL);
    assert_eq!(a.md5, b.md5);
    assert_eq!(a.sha256, b.sha256);
    assert_eq!(a.md5.len(), 32);
    assert_eq!(a.sha256.len(), 64);
}

#[test]
fn hash_independent_of_parse_seed() {
    // Hashes are computed on raw bytes, before any random resolution.
    let chart = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let a = parse_with(chart, ParseOptions { random_seed: 1 });
    let b = parse_with(chart, ParseOptions { random_seed: 999 });
    assert_eq!(a.md5, b.md5);
    assert_eq!(a.sha256, b.sha256);
}

#[test]
fn hash_hex_lowercase_and_hexdigits_only() {
    let s = parse(b"some bytes");
    assert!(s.md5.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert!(s.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

// ----------------------------------------------------------------------------
// control flow: nested RANDOM, ELSEIF chains, ELSE, ENDRANDOM, SETRANDOM,
// unbalanced IF/ENDIF, determinism / divergence
// ----------------------------------------------------------------------------

#[test]
fn if_zero_active_without_enclosing_random() {
    // current_random() is 0 with no Random frame, so "#IF 0" is active.
    let s = parse(b"#IF 0\r\n#WAV01 z.wav\r\n#ENDIF\r\n");
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("z.wav"));
}

#[test]
fn if_nonzero_inactive_without_enclosing_random() {
    let s = parse(b"#IF 1\r\n#WAV01 z.wav\r\n#ENDIF\r\n");
    assert!(s.wav.get(&1).is_none());
}

#[test]
fn setrandom_first_branch_selected() {
    let s = parse(b"#SETRANDOM 1\r\n#IF 1\r\n#WAV01 one.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV01 two.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n");
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("one.wav"));
}

#[test]
fn elseif_chain_matches_middle() {
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSEIF 2\r\n#WAV01 b.wav\r\n#ELSEIF 3\r\n#WAV01 c.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("b.wav"));
}

#[test]
fn elseif_chain_matches_last() {
    let chart = b"#SETRANDOM 3\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSEIF 2\r\n#WAV01 b.wav\r\n#ELSEIF 3\r\n#WAV01 c.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("c.wav"));
}

#[test]
fn elseif_no_match_emits_nothing() {
    let chart = b"#SETRANDOM 9\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSEIF 2\r\n#WAV01 b.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert!(s.wav.get(&1).is_none());
}

#[test]
fn else_after_matched_if_not_taken() {
    let chart = b"#SETRANDOM 1\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSE\r\n#WAV01 b.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
}

#[test]
fn else_after_unmatched_elseif_chain_taken() {
    // IF 1 no, ELSEIF 2 no (random=5), ELSE yes.
    let chart = b"#SETRANDOM 5\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSEIF 2\r\n#WAV01 b.wav\r\n#ELSE\r\n#WAV01 c.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("c.wav"));
}

#[test]
fn only_one_branch_emitted_in_if_elseif_else() {
    // Exactly one of the three slots wins; the others must be absent.
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#WAV01 a.wav\r\n#ELSEIF 2\r\n#WAV02 b.wav\r\n#ELSE\r\n#WAV03 c.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert!(s.wav.get(&1).is_none());
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("b.wav"));
    assert!(s.wav.get(&3).is_none());
}

#[test]
fn nested_random_inner_governs_inner_if() {
    // Outer chooses branch 1; inner SETRANDOM 2 governs the inner IF.
    let chart = b"#SETRANDOM 1\r\n#IF 1\r\n#SETRANDOM 2\r\n#IF 1\r\n#WAV01 inner_one.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV01 inner_two.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("inner_two.wav"));
}

#[test]
fn nested_random_inactive_outer_suppresses_inner() {
    // Outer chooses branch 2 -> the IF 1 block (with the inner random) is inactive,
    // so nothing inside is emitted regardless of the inner random.
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#SETRANDOM 1\r\n#IF 1\r\n#WAV01 should_not.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert!(s.wav.get(&1).is_none());
}

#[test]
fn current_random_uses_innermost_random_frame() {
    // Two stacked randoms; the innermost value (3) drives the IF.
    let chart = b"#SETRANDOM 1\r\n#SETRANDOM 3\r\n#IF 3\r\n#WAV01 hit.wav\r\n#ENDIF\r\n#IF 1\r\n#WAV01 miss.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("hit.wav"));
}

#[test]
fn endrandom_unwinds_dangling_if() {
    // An IF left open inside the random block is popped by ENDRANDOM, so a later
    // IF 0 (no enclosing random) is active again.
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDRANDOM\r\n#IF 0\r\n#WAV02 b.wav\r\n#ENDIF\r\n";
    let s = parse(chart);
    assert!(s.wav.get(&1).is_none()); // never emitted (IF 1 with random 2)
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("b.wav"));
}

#[test]
fn endrandom_with_no_random_is_noop() {
    // ENDRANDOM with empty stack pops nothing and does not panic.
    let s = parse(b"#ENDRANDOM\r\n#TITLE survives\r\n");
    assert_eq!(s.headers.title, "survives");
}

#[test]
fn unbalanced_if_without_endif_suppresses_rest() {
    // An open inactive IF leaves the frame on the stack, suppressing all following
    // header lines until end of file.
    let chart = b"#IF 1\r\n#TITLE inside_if\r\n#WAV01 a.wav\r\n";
    let s = parse(chart);
    assert_eq!(s.headers.title, ""); // inactive, not applied
    assert!(s.wav.get(&1).is_none());
}

#[test]
fn unbalanced_active_if_without_endif_still_emits() {
    let chart = b"#IF 0\r\n#TITLE inside_active_if\r\n";
    let s = parse(chart);
    assert_eq!(s.headers.title, "inside_active_if");
}

#[test]
fn endif_without_if_is_noop() {
    let s = parse(b"#ENDIF\r\n#TITLE after\r\n");
    assert_eq!(s.headers.title, "after");
}

#[test]
fn else_without_if_is_noop() {
    let s = parse(b"#ELSE\r\n#TITLE after\r\n");
    assert_eq!(s.headers.title, "after");
}

#[test]
fn elseif_without_if_is_noop() {
    let s = parse(b"#ELSEIF 1\r\n#TITLE after\r\n");
    assert_eq!(s.headers.title, "after");
}

#[test]
fn endif_pops_to_if_position() {
    // After a balanced IF/ENDIF, subsequent lines are active again.
    let chart = b"#IF 0\r\n#WAV01 a.wav\r\n#ENDIF\r\n#TITLE after_endif\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
    assert_eq!(s.headers.title, "after_endif");
}

#[test]
fn random_rondam_alias_behaves_like_random() {
    // "RONDAM" is accepted as an alias for "RANDOM"; with seed picking value 1 for n=2.
    let chart_random = b"#RANDOM 2\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV01 b.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let chart_rondam = b"#RONDAM 2\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV01 b.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let seed = ParseOptions { random_seed: 0 };
    let a = parse_with(chart_random, seed);
    let b = parse_with(chart_rondam, seed);
    assert_eq!(a.wav.get(&1), b.wav.get(&1));
    assert!(a.wav.get(&1).is_some());
}

#[test]
fn random_exactly_one_branch_selected() {
    // For any seed, a #RANDOM n with one IF per value emits exactly one branch.
    let chart = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV02 b.wav\r\n#ENDIF\r\n#IF 3\r\n#WAV03 c.wav\r\n#ENDIF\r\n#IF 4\r\n#WAV04 d.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    for seed in [0u64, 1, 2, 7, 42, 100, 1234, u64::MAX] {
        let s = parse_with(chart, ParseOptions { random_seed: seed });
        let count = (1..=4u32).filter(|id| s.wav.get(id).is_some()).count();
        assert_eq!(count, 1, "seed {} selected {} branches", seed, count);
    }
}

#[test]
fn random_seed_zero_n4_selects_branch_one() {
    // Derived from the LCG: seed 0 with n=4 draws value 1.
    let chart = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV02 b.wav\r\n#ENDIF\r\n#IF 3\r\n#WAV03 c.wav\r\n#ENDIF\r\n#IF 4\r\n#WAV04 d.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse_with(chart, ParseOptions { random_seed: 0 });
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
}

#[test]
fn random_seed_one_n4_selects_branch_four() {
    // Derived from the LCG: seed 1 with n=4 draws value 4.
    let chart = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV02 b.wav\r\n#ENDIF\r\n#IF 3\r\n#WAV03 c.wav\r\n#ENDIF\r\n#IF 4\r\n#WAV04 d.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse_with(chart, ParseOptions { random_seed: 1 });
    assert_eq!(s.wav.get(&4).map(String::as_str), Some("d.wav"));
    assert!(s.wav.get(&1).is_none());
}

#[test]
fn random_equal_seeds_agree() {
    let chart = b"#RANDOM 8\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 5\r\n#WAV05 e.wav\r\n#ENDIF\r\n#IF 8\r\n#WAV08 h.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    for seed in [0u64, 3, 77, 9999] {
        let a = parse_with(chart, ParseOptions { random_seed: seed });
        let b = parse_with(chart, ParseOptions { random_seed: seed });
        assert_eq!(a.wav, b.wav, "seed {} not deterministic", seed);
    }
}

#[test]
fn random_different_seeds_can_diverge() {
    // seed 0 -> branch 1, seed 1 -> branch 4 for n=4: outputs differ.
    let chart = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV02 b.wav\r\n#ENDIF\r\n#IF 3\r\n#WAV03 c.wav\r\n#ENDIF\r\n#IF 4\r\n#WAV04 d.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let a = parse_with(chart, ParseOptions { random_seed: 0 });
    let b = parse_with(chart, ParseOptions { random_seed: 1 });
    assert_ne!(a.wav, b.wav);
}

#[test]
fn random_n_zero_yields_value_one() {
    // rand_range(0) returns 1, so "#IF 1" is active.
    let chart = b"#RANDOM 0\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    for seed in [0u64, 1, 42] {
        let s = parse_with(chart, ParseOptions { random_seed: seed });
        assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"), "seed {}", seed);
    }
}

#[test]
fn random_missing_arg_defaults_to_one() {
    // "#RANDOM" with no number -> n defaults to 1 -> rand_range(1) == 1.
    let chart = b"#RANDOM\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse_with(chart, ParseOptions { random_seed: 12345 });
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
}

#[test]
fn setrandom_missing_arg_defaults_to_one() {
    let chart = b"#SETRANDOM\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
}

#[test]
fn if_non_numeric_value_treated_as_zero() {
    // "#IF foo" parses to 0; with no random, current_random()==0 -> active.
    let s = parse(b"#IF foo\r\n#WAV01 a.wav\r\n#ENDIF\r\n");
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
}

#[test]
fn data_lines_inside_inactive_branch_skipped() {
    // Control flow also gates data lines, not just headers.
    let chart = b"#IF 1\r\n#00111:0102\r\n#ENDIF\r\n";
    let s = parse(chart);
    assert!(s.measures.is_empty());
}

#[test]
fn data_lines_inside_active_branch_kept() {
    let chart = b"#IF 0\r\n#00111:0102\r\n#ENDIF\r\n";
    let s = parse(chart);
    assert_eq!(s.measures.get(&1).unwrap().channels[0].objects.len(), 2);
}

#[test]
fn measure_rate_nan_is_rejected_and_keeps_default_one() {
    let s = parse(b"#00102:nan\r\n#00111:01\r\n");
    assert_eq!(s.measures.get(&1).unwrap().rate, 1.0);
}

#[test]
fn measure_rate_inf_and_non_positive_are_rejected() {
    for body in [&b"#00102:inf\r\n"[..], b"#00102:-inf\r\n", b"#00102:0\r\n", b"#00102:-0.5\r\n"] {
        let s = parse(body);
        assert_eq!(s.measures.get(&1).unwrap().rate, 1.0, "rejected: {}", String::from_utf8_lossy(body));
    }
}

#[test]
fn measure_rate_valid_positive_is_kept() {
    let s = parse(b"#00102:0.75\r\n");
    assert_eq!(s.measures.get(&1).unwrap().rate, 0.75);
}

#[test]
fn defexrank_is_parsed_and_absent_by_default() {
    assert_eq!(parse(b"#DEFEXRANK 130\r\n").headers.defexrank, Some(130.0));
    assert_eq!(parse(b"#TITLE t\r\n").headers.defexrank, None);
    assert_eq!(parse(b"#DEFEXRANK nan\r\n").headers.defexrank, None, "non-finite is ignored");
}

#[test]
fn switch_keeps_only_the_case_matching_the_value() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1\r\n#WAV01 one.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV02 two.wav\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("two.wav"));
}

#[test]
fn matched_case_falls_through_two_following_cases_until_skip() {
    let chart = b"#SETSWITCH 1\r\n#CASE 1\r\n#WAV01 a.wav\r\n#CASE 2\r\n#WAV02 b.wav\r\n#CASE 3\r\n#WAV03 c.wav\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 3);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("b.wav"));
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("c.wav"));
}

#[test]
fn skip_deactivates_the_rest_of_the_switch_until_endsw() {
    let chart = b"#SETSWITCH 1\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#WAV02 dead.wav\r\n#CASE 2\r\n#WAV03 dead.wav\r\n#SKIP\r\n#DEF\r\n#WAV04 dead.wav\r\n#ENDSW\r\n#WAV05 after.wav\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 2);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
    assert_eq!(s.wav.get(&5).map(String::as_str), Some("after.wav"));
}

#[test]
fn def_runs_when_no_case_matched() {
    let chart = b"#SETSWITCH 9\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV02 b.wav\r\n#SKIP\r\n#DEF\r\n#WAV03 fallback.wav\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("fallback.wav"));
}

#[test]
fn def_in_the_middle_activates_and_falls_through_to_a_later_case() {
    let chart = b"#SETSWITCH 3\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#DEF\r\n#WAV02 fallback.wav\r\n#CASE 3\r\n#WAV03 c.wav\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 2);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("fallback.wav"));
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("c.wav"));
}

#[test]
fn def_in_the_middle_followed_by_skip_hides_a_later_matching_case() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#DEF\r\n#WAV02 fallback.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV03 b.wav\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("fallback.wav"));
}

#[test]
fn random_nested_inside_a_case_selects_its_own_branch() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#CASE 2\r\n#SETRANDOM 2\r\n#IF 1\r\n#WAV02 r1.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV03 r2.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("r2.wav"));
}

#[test]
fn switch_nested_inside_an_if_branch_is_resolved() {
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#SETSWITCH 1\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#ENDSW\r\n#ENDIF\r\n#IF 2\r\n#SETSWITCH 1\r\n#CASE 1\r\n#WAV02 b.wav\r\n#SKIP\r\n#ENDSW\r\n#ENDIF\r\n#ENDRANDOM\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("b.wav"));
}

#[test]
fn switch_inside_an_inactive_if_emits_nothing_not_even_def() {
    let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#SETSWITCH 1\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#DEF\r\n#WAV02 fallback.wav\r\n#ENDSW\r\n#ENDIF\r\n#ENDRANDOM\r\n#WAV03 outside.wav\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("outside.wav"));
}

#[test]
fn endsw_closes_only_the_innermost_switch() {
    let chart = b"#SETSWITCH 1\r\n#CASE 1\r\n#SETSWITCH 2\r\n#CASE 1\r\n#WAV01 inner1.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV02 inner2.wav\r\n#SKIP\r\n#ENDSW\r\n#WAV03 outer.wav\r\n#SKIP\r\n#ENDSW\r\n#WAV04 after.wav\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 3);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("inner2.wav"));
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("outer.wav"));
    assert_eq!(s.wav.get(&4).map(String::as_str), Some("after.wav"));
}

#[test]
fn setswitch_pins_the_value_for_every_seed() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1\r\n#WAV01 one.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV02 two.wav\r\n#SKIP\r\n#ENDSW\r\n";
    for seed in [0u64, 1, 2, 7, 12345] {
        let s = parse_with(chart, ParseOptions { random_seed: seed });
        assert_eq!(s.wav.get(&2).map(String::as_str), Some("two.wav"), "seed {seed}");
        assert_eq!(s.wav.len(), 1, "seed {seed}");
    }
}

const SWITCH_FOUR: &[u8] = b"#SWITCH 4\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV02 b.wav\r\n#SKIP\r\n#CASE 3\r\n#WAV03 c.wav\r\n#SKIP\r\n#CASE 4\r\n#WAV04 d.wav\r\n#SKIP\r\n#ENDSW\r\n";

#[test]
fn switch_draw_is_deterministic_per_seed_and_seeds_can_differ() {
    let first = parse_with(SWITCH_FOUR, ParseOptions { random_seed: 0 });
    let again = parse_with(SWITCH_FOUR, ParseOptions { random_seed: 0 });
    assert_eq!(first.wav.get(&1).map(String::as_str), Some("a.wav"));
    assert_eq!(first.wav.len(), 1);
    assert_eq!(again.wav.get(&1).map(String::as_str), Some("a.wav"));

    let other = parse_with(SWITCH_FOUR, ParseOptions { random_seed: 1 });
    assert_eq!(other.wav.get(&4).map(String::as_str), Some("d.wav"));
    assert_eq!(other.wav.len(), 1);
}

#[test]
fn orphan_case_def_skip_endsw_are_ignored() {
    let s = parse(b"#ENDSW\r\n#CASE 1\r\n#DEF\r\n#SKIP\r\n#WAV01 a.wav\r\n");
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("a.wav"));
}

#[test]
fn switch_left_unclosed_keeps_gating_until_end_of_file() {
    let s = parse(b"#WAV01 before.wav\r\n#SETSWITCH 2\r\n#WAV02 dead.wav\r\n#CASE 1\r\n#WAV03 dead.wav\r\n#CASE 2\r\n#WAV04 b.wav\r\n");
    assert_eq!(s.wav.len(), 2);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("before.wav"));
    assert_eq!(s.wav.get(&4).map(String::as_str), Some("b.wav"));
}

#[test]
fn switch_gates_data_lines_and_header_lines_alike() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1\r\n#TITLE one\r\n#00111:0101\r\n#SKIP\r\n#CASE 2\r\n#TITLE two\r\n#00111:0202\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.headers.title, "two");
    let m = s.measures.get(&1).unwrap();
    let ch = m.channels.iter().find(|c| c.channel == 37).unwrap();
    assert_eq!(ch.objects.len(), 2);
    assert_eq!(ch.objects[0].value(36), 2);
    assert_eq!(ch.objects[1].value(36), 2);
}

#[test]
fn switch_resolution_does_not_change_md5_or_sha256() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1\r\n#WAV01 a.wav\r\n#SKIP\r\n#CASE 2\r\n#WAV01 b.wav\r\n#SKIP\r\n#ENDSW\r\n";
    for seed in [0u64, 1, 99] {
        let s = parse_with(chart, ParseOptions { random_seed: seed });
        assert_eq!(s.md5, "afe8e024f296b30631044310918f0a57", "seed {seed}");
        assert_eq!(s.sha256, "697fd9c7ae1ee8ae0301164464186adcefbf56a338aad4624b716f67c7b60f9a", "seed {seed}");
    }
}

#[test]
fn skip_inside_an_inactive_nested_branch_keeps_the_rest_of_the_case() {
    let chart =
        b"#SETSWITCH 1\r\n#CASE 1\r\n#SETRANDOM 2\r\n#IF 1\r\n#WAV01 dead.wav\r\n#SKIP\r\n#ENDIF\r\n#ENDRANDOM\r\n#WAV02 must_stay.wav\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse_with(chart, ParseOptions { random_seed: 0 });
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("must_stay.wav"));
}

#[test]
fn skip_inside_an_active_nested_branch_ends_the_case() {
    let chart = b"#SETSWITCH 1\r\n#CASE 1\r\n#SETRANDOM 2\r\n#IF 2\r\n#WAV01 live.wav\r\n#SKIP\r\n#ENDIF\r\n#ENDRANDOM\r\n#WAV02 dead.wav\r\n#ENDSW\r\n#WAV03 after.wav\r\n";
    let s = parse_with(chart, ParseOptions { random_seed: 0 });
    assert_eq!(s.wav.len(), 2);
    assert_eq!(s.wav.get(&1).map(String::as_str), Some("live.wav"));
    assert_eq!(s.wav.get(&3).map(String::as_str), Some("after.wav"));
}

#[test]
fn case_label_ignores_trailing_tokens() {
    let chart = b"#SETSWITCH 2\r\n#CASE 1 first\r\n#WAV01 one.wav\r\n#SKIP\r\n#CASE 2 second\r\n#WAV02 two.wav\r\n#SKIP\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("two.wav"));
}

#[test]
fn case_with_an_unparsable_label_never_matches_and_leaves_def_to_run() {
    let chart = b"#SETSWITCH 0\r\n#CASE junk\r\n#WAV01 dead.wav\r\n#SKIP\r\n#DEF\r\n#WAV02 fallback.wav\r\n#ENDSW\r\n";
    let s = parse(chart);
    assert_eq!(s.wav.len(), 1);
    assert_eq!(s.wav.get(&2).map(String::as_str), Some("fallback.wav"));
}

const RANDOM_FOUR: &[u8] = b"#RANDOM 4\r\n#IF 1\r\n#WAV01 a.wav\r\n#ENDIF\r\n#IF 2\r\n#WAV02 b.wav\r\n#ENDIF\r\n#IF 3\r\n#WAV03 c.wav\r\n#ENDIF\r\n#IF 4\r\n#WAV04 d.wav\r\n#ENDIF\r\n#ENDRANDOM\r\n";

#[test]
fn switch_draw_follows_the_same_seed_rule_as_random_draw() {
    for seed in [0u64, 1, 2, 3, 7, 42] {
        let by_switch = parse_with(SWITCH_FOUR, ParseOptions { random_seed: seed });
        let by_random = parse_with(RANDOM_FOUR, ParseOptions { random_seed: seed });
        let switch_keys: Vec<u32> = by_switch.wav.keys().copied().collect();
        let random_keys: Vec<u32> = by_random.wav.keys().copied().collect();
        assert_eq!(switch_keys.len(), 1, "seed {seed}");
        assert_eq!(switch_keys, random_keys, "seed {seed}");
    }
    assert_eq!(parse_with(RANDOM_FOUR, ParseOptions { random_seed: 0 }).wav.get(&1).map(String::as_str), Some("a.wav"));
    assert_eq!(parse_with(RANDOM_FOUR, ParseOptions { random_seed: 1 }).wav.get(&4).map(String::as_str), Some("d.wav"));
}
