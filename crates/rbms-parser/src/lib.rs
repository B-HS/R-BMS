#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use md5::{Digest, Md5};
use rbms_model::VOLWAV_DEFAULT_PERCENT;
use sha2::Sha256;

mod base;
mod control;

pub use base::{digit, parse_pair};

/// One object placed in a measure channel: value token `(c0,c1)` at fraction `num/den`.
#[derive(Debug, Clone)]
pub struct Obj {
    pub num: u32,
    pub den: u32,
    pub c0: u8,
    pub c1: u8,
}

impl Obj {
    pub fn value(&self, base: u32) -> u32 {
        parse_pair(self.c0, self.c1, base)
    }
    pub fn value16(&self) -> u32 {
        parse_pair(self.c0, self.c1, 16)
    }
    pub fn pos(&self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

#[derive(Debug, Clone)]
pub struct ChannelData {
    pub channel: u32,
    pub objects: Vec<Obj>,
}

#[derive(Debug, Clone)]
pub struct Measure {
    pub rate: f64,
    pub channels: Vec<ChannelData>,
}

impl Default for Measure {
    fn default() -> Self {
        Measure { rate: 1.0, channels: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub struct Headers {
    pub player: i32,
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub subartist: String,
    pub maker: String,
    pub genre: String,
    pub stagefile: String,
    pub banner: String,
    pub preview: String,
    pub play_level: String,
    pub rank: i32,
    pub defexrank: Option<f64>,
    pub total: Option<f64>,
    /// `#VOLWAV` as authored, in percent. Absent or unparsable headers keep
    /// [`VOLWAV_DEFAULT_PERCENT`], which converts to unity gain.
    pub volwav: i32,
    pub init_bpm: f64,
    pub lnobj: Option<u32>,
    pub lntype: i32,
    pub lnmode: i32,
    pub difficulty: i32,
}

impl Default for Headers {
    fn default() -> Self {
        Headers {
            player: 1,
            title: String::new(),
            subtitle: String::new(),
            artist: String::new(),
            subartist: String::new(),
            maker: String::new(),
            genre: String::new(),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
            play_level: String::new(),
            rank: 3,
            defexrank: None,
            total: None,
            volwav: VOLWAV_DEFAULT_PERCENT,
            init_bpm: 130.0,
            lnobj: None,
            lntype: 1,
            lnmode: 0,
            difficulty: 0,
        }
    }
}

/// Raw lexical result of parsing a BMS-family chart, before timing integration.
#[derive(Debug, Clone)]
pub struct BmsSource {
    pub headers: Headers,
    pub wav: BTreeMap<u32, String>,
    pub bmp: BTreeMap<u32, String>,
    pub bpm_def: BTreeMap<u32, f64>,
    pub stop_def: BTreeMap<u32, f64>,
    pub scroll_def: BTreeMap<u32, f64>,
    pub measures: BTreeMap<u32, Measure>,
    pub base: u32,
    pub md5: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParseOptions {
    pub random_seed: u64,
}

pub fn parse(bytes: &[u8]) -> BmsSource {
    parse_with(bytes, ParseOptions::default())
}

pub fn parse_with(bytes: &[u8], opts: ParseOptions) -> BmsSource {
    let md5 = hex(Md5::digest(bytes).as_slice());
    let sha256 = hex(Sha256::digest(bytes).as_slice());
    let text = decode_text(bytes);

    let mut src = BmsSource {
        headers: Headers::default(),
        wav: BTreeMap::new(),
        bmp: BTreeMap::new(),
        bpm_def: BTreeMap::new(),
        stop_def: BTreeMap::new(),
        scroll_def: BTreeMap::new(),
        measures: BTreeMap::new(),
        base: 36,
        md5,
        sha256,
    };

    for raw in text.lines() {
        let l = raw.trim();
        if let Some(p) = l.get(..5)
            && p.eq_ignore_ascii_case("#BASE")
            && l.get(5..).map(str::trim) == Some("62")
        {
            src.base = 62;
        }
    }

    let mut ctrl = control::Control::new(opts.random_seed);

    for raw in text.lines() {
        let line = raw.trim();
        if !line.starts_with('#') {
            continue;
        }
        let body = &line[1..];
        if ctrl.handle(body) {
            continue;
        }
        if !ctrl.active() {
            continue;
        }
        if is_data_line(body) {
            parse_data_line(&mut src, body);
        } else {
            parse_header_line(&mut src, body);
        }
    }

    src
}

fn is_data_line(body: &str) -> bool {
    let b = body.as_bytes();
    b.len() >= 6 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2].is_ascii_digit() && b[5] == b':'
}

fn parse_data_line(src: &mut BmsSource, body: &str) {
    let b = body.as_bytes();
    let measure_idx = ((b[0] - b'0') as u32) * 100 + ((b[1] - b'0') as u32) * 10 + (b[2] - b'0') as u32;
    let channel = parse_pair(b[3], b[4], 36);
    let data = &body[6..];

    let measure = src.measures.entry(measure_idx).or_default();

    if channel == 2 {
        if let Ok(rate) = data.trim().parse::<f64>()
            && rate.is_finite()
            && rate > 0.0
        {
            measure.rate = rate;
        }
        return;
    }

    let chars: Vec<char> = data.chars().collect();
    let len = chars.len() / 2;
    if len == 0 {
        return;
    }
    let mut objects = Vec::new();
    for i in 0..len {
        let a = chars[2 * i];
        let b = chars[2 * i + 1];
        if !a.is_ascii() || !b.is_ascii() {
            continue;
        }
        let (c0, c1) = (a as u8, b as u8);
        if digit(c0, 36).is_none() || digit(c1, 36).is_none() {
            continue;
        }
        if c0 == b'0' && c1 == b'0' {
            continue;
        }
        objects.push(Obj { num: i as u32, den: len as u32, c0, c1 });
    }
    if !objects.is_empty() {
        measure.channels.push(ChannelData { channel, objects });
    }
}

fn parse_header_line(src: &mut BmsSource, body: &str) {
    let (head, rest) = match body.find(char::is_whitespace) {
        Some(i) => (&body[..i], body[i..].trim()),
        None => (body, ""),
    };
    let hu = head.to_ascii_uppercase();
    let base = src.base;
    let h = &mut src.headers;

    match hu.as_str() {
        "TITLE" => h.title = rest.to_owned(),
        "SUBTITLE" => h.subtitle = rest.to_owned(),
        "ARTIST" => h.artist = rest.to_owned(),
        "SUBARTIST" => h.subartist = rest.to_owned(),
        "MAKER" => h.maker = rest.to_owned(),
        "GENRE" => h.genre = rest.to_owned(),
        "STAGEFILE" => h.stagefile = rest.to_owned(),
        "BANNER" => h.banner = rest.to_owned(),
        "PREVIEW" => h.preview = rest.to_owned(),
        "PLAYLEVEL" => h.play_level = rest.to_owned(),
        "PLAYER" => h.player = rest.parse().unwrap_or(1),
        "RANK" => h.rank = rest.parse().unwrap_or(3),
        "DEFEXRANK" => h.defexrank = rest.parse::<f64>().ok().filter(|v| v.is_finite()),
        "TOTAL" => h.total = rest.parse().ok(),
        "VOLWAV" => h.volwav = rest.parse().unwrap_or(VOLWAV_DEFAULT_PERCENT),
        "DIFFICULTY" => h.difficulty = rest.parse().unwrap_or(0),
        "BPM" => h.init_bpm = rest.parse().unwrap_or(130.0),
        "LNTYPE" => h.lntype = rest.parse().unwrap_or(1),
        "LNMODE" => h.lnmode = rest.parse().unwrap_or(0),
        "LNOBJ" => {
            let t = rest.as_bytes();
            if t.len() >= 2 {
                h.lnobj = Some(parse_pair(t[0], t[1], base));
            }
        }
        "BASE" => {
            if rest.trim() == "62" {
                src.base = 62;
            }
        }
        _ => {
            if hu.len() == 5 && hu.starts_with("WAV") {
                let id = parse_pair(head.as_bytes()[3], head.as_bytes()[4], base);
                src.wav.insert(id, rest.to_owned());
            } else if hu.len() == 5 && hu.starts_with("BMP") {
                let id = parse_pair(head.as_bytes()[3], head.as_bytes()[4], base);
                src.bmp.insert(id, rest.to_owned());
            } else if hu.len() == 5 && hu.starts_with("BPM") {
                let id = parse_pair(head.as_bytes()[3], head.as_bytes()[4], base);
                if let Ok(v) = rest.parse::<f64>() {
                    src.bpm_def.insert(id, v);
                }
            } else if hu.len() == 6 && hu.starts_with("STOP") {
                let id = parse_pair(head.as_bytes()[4], head.as_bytes()[5], base);
                if let Ok(v) = rest.parse::<f64>() {
                    src.stop_def.insert(id, v);
                }
            } else if hu.len() == 8 && hu.starts_with("SCROLL") {
                let id = parse_pair(head.as_bytes()[6], head.as_bytes()[7], base);
                if let Ok(v) = rest.parse::<f64>() {
                    src.scroll_def.insert(id, v);
                }
            }
        }
    }
}

fn decode_text(bytes: &[u8]) -> String {
    if let [0xEF, 0xBB, 0xBF, rest @ ..] = bytes {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_owned();
    }
    let (cow, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
    cow.into_owned()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod volwav_tests {
    use super::*;

    fn volwav_of(line: &[u8]) -> i32 {
        parse(line).headers.volwav
    }

    #[test]
    fn absent_header_keeps_the_default_percent() {
        assert_eq!(Headers::default().volwav, VOLWAV_DEFAULT_PERCENT);
        assert_eq!(volwav_of(b"#TITLE no volume header\r\n"), VOLWAV_DEFAULT_PERCENT);
    }

    #[test]
    fn reference_boundary_values_parse_verbatim() {
        assert_eq!(volwav_of(b"#VOLWAV 0\r\n"), 0);
        assert_eq!(volwav_of(b"#VOLWAV 1\r\n"), 1);
        assert_eq!(volwav_of(b"#VOLWAV 99\r\n"), 99);
        assert_eq!(volwav_of(b"#VOLWAV 100\r\n"), 100);
        assert_eq!(volwav_of(b"#VOLWAV 199\r\n"), 199);
        assert_eq!(volwav_of(b"#VOLWAV 200\r\n"), 200);
        assert_eq!(volwav_of(b"#VOLWAV 201\r\n"), 201);
        assert_eq!(volwav_of(b"#VOLWAV -5\r\n"), -5);
    }

    #[test]
    fn non_numeric_value_falls_back_to_the_default_percent() {
        assert_eq!(volwav_of(b"#VOLWAV abc\r\n"), VOLWAV_DEFAULT_PERCENT);
    }

    #[test]
    fn decimal_value_falls_back_like_an_integer_parse() {
        assert_eq!(volwav_of(b"#VOLWAV 100.0\r\n"), VOLWAV_DEFAULT_PERCENT);
        assert_eq!(volwav_of(b"#VOLWAV 80.5\r\n"), VOLWAV_DEFAULT_PERCENT);
    }

    #[test]
    fn missing_value_falls_back_to_the_default_percent() {
        assert_eq!(volwav_of(b"#VOLWAV\r\n"), VOLWAV_DEFAULT_PERCENT);
        assert_eq!(volwav_of(b"#VOLWAV   \r\n"), VOLWAV_DEFAULT_PERCENT);
    }

    #[test]
    fn value_outside_i32_falls_back_to_the_default_percent() {
        assert_eq!(volwav_of(b"#VOLWAV 99999999999\r\n"), VOLWAV_DEFAULT_PERCENT);
        assert_eq!(volwav_of(b"#VOLWAV -99999999999\r\n"), VOLWAV_DEFAULT_PERCENT);
    }

    #[test]
    fn header_name_is_case_insensitive() {
        assert_eq!(volwav_of(b"#volwav 80\r\n"), 80);
        assert_eq!(volwav_of(b"#VolWav 80\r\n"), 80);
    }

    #[test]
    fn leading_plus_and_surrounding_whitespace_are_accepted() {
        assert_eq!(volwav_of(b"#VOLWAV +50\r\n"), 50);
        assert_eq!(volwav_of(b"#VOLWAV   75  \r\n"), 75);
    }

    #[test]
    fn last_definition_wins() {
        assert_eq!(volwav_of(b"#VOLWAV 40\r\n#VOLWAV 60\r\n"), 60);
    }

    #[test]
    fn header_does_not_collide_with_wav_definitions() {
        let s = parse(b"#WAV01 kick.wav\r\n#VOLWAV 50\r\n");
        assert_eq!(s.headers.volwav, 50);
        assert_eq!(s.wav.len(), 1);
        assert_eq!(s.wav.get(&1).map(String::as_str), Some("kick.wav"));
    }

    #[test]
    fn inactive_control_branch_does_not_apply_the_value() {
        let chart = b"#SETRANDOM 2\r\n#IF 1\r\n#VOLWAV 30\r\n#ENDIF\r\n#IF 2\r\n#VOLWAV 70\r\n#ENDIF\r\n#ENDRANDOM\r\n";
        assert_eq!(parse(chart).headers.volwav, 70);
    }
}
