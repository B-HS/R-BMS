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
