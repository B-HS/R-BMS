//! The bmson chart format: JSON decoding and the mapping onto the shared chart model.
//!
//! bmson places every event on a pulse grid (`y`, with `info.resolution` pulses per quarter note)
//! instead of the measure/channel grid a BMS file uses, so it does not go through [`BmsSource`].
//! [`parse`] decodes the document and [`BmsonChart::to_model`] turns it into a [`Model`] directly,
//! which is the same shape [`crate::parse`] plus `rbms-chart` produce for a BMS chart.
//!
//! bmson carries no `#DIFFICULTY`, `#VOLWAV`, `#MAKER` or `#PLAYER`, so the model takes the neutral
//! value for each: difficulty 0, unity chart gain, an empty maker, and the player count of the mode
//! the `mode_hint` names.
//!
//! [`BmsSource`]: crate::BmsSource

use std::fmt;

use md5::{Digest, Md5};
use rbms_model::{Mode, Model, default_total_for_mode};
use serde_json::{Map, Value};
use sha2::Sha256;

use crate::hex;

mod convert;
#[cfg(test)]
mod tests;

/// File extension of a bmson chart, without the dot. The library scanner filters on it.
pub const EXTENSION: &str = "bmson";

/// Pulses per quarter note assumed when `info.resolution` is absent or not positive.
pub const DEFAULT_RESOLUTION: i64 = 240;

/// Quarter notes in the 4/4 measure every rbms section length is stated in.
const QUARTERS_PER_MEASURE: i64 = 4;

/// `judge_rank` percentage a bmson document carries when it states none.
const DEFAULT_JUDGE_RANK: i64 = 100;

/// `total` percentage a bmson document carries when it states none: the mode's default TOTAL.
const DEFAULT_TOTAL_PERCENT: f64 = 100.0;

/// Denominator of every percentage bmson states.
const PERCENT_DENOMINATOR: f64 = 100.0;

/// At or above this the reference implementation reads `judge_rank` as a judgerank percentage; below
/// it the value is a BMS `#RANK` index instead (`BMSONDecoder.java` `decodeSource`).
const BMSON_JUDGERANK_MIN: i64 = 5;

/// `#RANK` the reference implementation's chart model starts at, which a negative `judge_rank`
/// leaves untouched: NORMAL.
const NORMAL_RANK: i32 = 2;

/// `#RANK` whose judgerank is 100 % under both window rules. A bmson judgerank percentage maps here
/// because the shared chart model carries no judgerank-type axis yet; see the branch wiring note.
const PERCENT_FALLBACK_RANK: i32 = 3;

/// Long-note flavour code range a note's `t` (or `info.ln_type`) states, matching `#LNMODE`:
/// 1 LN, 2 CN, 3 HCN. Anything else leaves the flavour to the player's LN MODE.
const LN_TYPE_MAX: i64 = 3;

/// `wav` of a long-note tail no `up` note gave a keysound to (`BMSONDecoder.java` `decodeSource`).
/// It indexes no entry of [`Model::wavmap`], so nothing sounds at the tail.
const SILENT_WAV: i32 = -2;

/// Byte-order mark a text editor may leave on a UTF-8 bmson file. It is not JSON, so it is skipped
/// before decoding - but it still counts towards the file hashes.
const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The `mode_hint` tokens this engine has a lane table for, and the mode each selects
/// (reference implementation `Mode` enum hints). Every other hint falls back to [`Mode::BEAT_7K`].
pub const MODE_HINTS: [(&str, Mode); 6] = [
    ("beat-5k", Mode::BEAT_5K),
    ("beat-7k", Mode::BEAT_7K),
    ("beat-10k", Mode::BEAT_10K),
    ("beat-14k", Mode::BEAT_14K),
    ("popn-9k", Mode::POPN_9K),
    ("keyboard-24k", Mode::KEYBOARD_24K),
];

/// Why a byte slice is not a bmson chart.
#[derive(Debug)]
pub enum BmsonError {
    /// The bytes are not well-formed JSON.
    Json(serde_json::Error),
    /// The JSON document is not an object.
    NotAnObject,
    /// `info` is absent, null, or not an object.
    MissingInfo,
    /// A field is present with a type the format does not allow, or a pulse position is negative.
    InvalidField { field: &'static str },
    /// `info.init_bpm` is absent or not a positive, finite tempo.
    InvalidInitBpm { bpm: f64 },
}

impl fmt::Display for BmsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BmsonError::Json(e) => write!(f, "bmson is not valid JSON: {e}"),
            BmsonError::NotAnObject => write!(f, "bmson root is not a JSON object"),
            BmsonError::MissingInfo => write!(f, "bmson has no info object"),
            BmsonError::InvalidField { field } => write!(f, "bmson field {field} has an unusable value"),
            BmsonError::InvalidInitBpm { bpm } => write!(f, "bmson init_bpm {bpm} is not a positive tempo"),
        }
    }
}

impl std::error::Error for BmsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BmsonError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for BmsonError {
    fn from(e: serde_json::Error) -> Self {
        BmsonError::Json(e)
    }
}

/// The `info` object: everything a bmson document states about the chart itself.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsonInfo {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub subartists: Vec<String>,
    pub genre: String,
    pub mode_hint: String,
    pub chart_name: String,
    /// A `#RANK` index below [`BMSON_JUDGERANK_MIN`], a judgerank percentage at or above it.
    pub judge_rank: i64,
    /// TOTAL as a percentage of the mode's default TOTAL, not an absolute gauge total.
    pub total: f64,
    pub init_bpm: f64,
    pub level: i64,
    pub back_image: String,
    pub eyecatch_image: String,
    pub banner_image: String,
    pub preview_music: String,
    /// Pulses per quarter note.
    pub resolution: i64,
    /// Chart-wide long-note flavour, in `#LNMODE` codes.
    pub ln_type: i64,
}

impl Default for BmsonInfo {
    fn default() -> Self {
        BmsonInfo {
            title: String::new(),
            subtitle: String::new(),
            artist: String::new(),
            subartists: Vec::new(),
            genre: String::new(),
            mode_hint: String::new(),
            chart_name: String::new(),
            judge_rank: DEFAULT_JUDGE_RANK,
            total: DEFAULT_TOTAL_PERCENT,
            init_bpm: 0.0,
            level: 0,
            back_image: String::new(),
            eyecatch_image: String::new(),
            banner_image: String::new(),
            preview_music: String::new(),
            resolution: DEFAULT_RESOLUTION,
            ln_type: 0,
        }
    }
}

/// One note of a sound channel. `l` above zero makes it a long note ending `l` pulses later, `c`
/// continues the channel's audio from where the previous note stopped instead of restarting it, `t`
/// overrides the chart-wide long-note flavour and `up` marks the note as the keysound of a long-note
/// tail rather than a note of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmsonNote {
    pub x: i64,
    pub y: i64,
    pub l: i64,
    pub c: bool,
    pub t: i64,
    pub up: bool,
}

/// One keysound and the notes played from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundChannel {
    pub name: String,
    pub notes: Vec<BmsonNote>,
}

/// One mine sound and the mines placed with it.
#[derive(Debug, Clone, PartialEq)]
pub struct MineChannel {
    pub name: String,
    pub notes: Vec<MineNote>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MineNote {
    pub x: i64,
    pub y: i64,
    pub damage: f64,
}

/// One keysound and the invisible notes placed with it. Only `x` and `y` of each note are honoured,
/// which is what the reference implementation reads from this channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChannel {
    pub name: String,
    pub notes: Vec<BmsonNote>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BpmEvent {
    pub y: i64,
    pub bpm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopEvent {
    pub y: i64,
    /// Stop length in pulses.
    pub duration: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollEvent {
    pub y: i64,
    pub rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarLine {
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgaHeader {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BgaEvent {
    pub y: i64,
    pub id: i64,
}

/// The `bga` object. `bga_sequence`, and the `condition`/`interval` of a layer event, are not read:
/// the shared chart model carries one base and one layer picture per timeline and no event layers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bga {
    pub header: Vec<BgaHeader>,
    pub base: Vec<BgaEvent>,
    pub layer: Vec<BgaEvent>,
    pub poor: Vec<BgaEvent>,
}

/// A decoded bmson document, with the hashes of the bytes it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsonChart {
    pub version: String,
    pub info: BmsonInfo,
    pub lines: Vec<BarLine>,
    pub bpm_events: Vec<BpmEvent>,
    pub stop_events: Vec<StopEvent>,
    pub scroll_events: Vec<ScrollEvent>,
    pub sound_channels: Vec<SoundChannel>,
    pub key_channels: Vec<KeyChannel>,
    pub mine_channels: Vec<MineChannel>,
    pub bga: Bga,
    pub md5: String,
    pub sha256: String,
}

impl BmsonChart {
    /// The play mode `info.mode_hint` selects. An unknown hint, and one naming a mode this engine
    /// has no lane table for, falls back to [`Mode::BEAT_7K`] like the reference implementation.
    pub fn mode(&self) -> Mode {
        MODE_HINTS.iter().find(|(hint, _)| *hint == self.info.mode_hint).map(|(_, mode)| *mode).unwrap_or(Mode::BEAT_7K)
    }

    /// Pulses per 4/4 measure, the unit `TimeLine::section` is stated in.
    pub fn pulses_per_measure(&self) -> f64 {
        let resolution = if self.info.resolution > 0 { self.info.resolution } else { DEFAULT_RESOLUTION };
        (resolution * QUARTERS_PER_MEASURE) as f64
    }

    /// The `#RANK` [`Self::to_model`] gives the chart. A `judge_rank` below [`BMSON_JUDGERANK_MIN`]
    /// is a rank index and maps across exactly; a negative one leaves the model at NORMAL; a
    /// judgerank percentage has no place in the shared chart model yet and maps to the 100 % rank.
    pub fn rank(&self) -> i32 {
        match self.info.judge_rank {
            j if j < 0 => NORMAL_RANK,
            j if j < BMSON_JUDGERANK_MIN => j as i32,
            _ => PERCENT_FALLBACK_RANK,
        }
    }

    /// The judgerank percentage the reference implementation judges this chart at, or `None` when
    /// `judge_rank` is a `#RANK` index instead of a percentage.
    pub fn judgerank_percent(&self) -> Option<i64> {
        (self.info.judge_rank >= BMSON_JUDGERANK_MIN).then_some(self.info.judge_rank)
    }

    /// Absolute `#TOTAL` for a chart of `mode` with `notes` playable notes. bmson states TOTAL as a
    /// percentage of the mode's default total, and a non-positive percentage means that default
    /// (`BMSPlayerRule.java:75-78`).
    pub fn total_for_notes(&self, mode: &Mode, notes: usize) -> f64 {
        let default = default_total_for_mode(mode, notes);
        if self.info.total > 0.0 { self.info.total / PERCENT_DENOMINATOR * default } else { default }
    }

    /// Build the play model under the mode `info.mode_hint` selects.
    pub fn to_model(&self) -> Model {
        convert::to_model(self, self.mode())
    }

    /// Build the play model under `mode`, whatever `info.mode_hint` says.
    pub fn to_model_in_mode(&self, mode: Mode) -> Model {
        convert::to_model(self, mode)
    }
}

/// Decode a bmson document. The md5 and sha256 of `bytes` exactly as given are kept on the result,
/// so a chart identifies the same way whichever format it is authored in.
pub fn parse(bytes: &[u8]) -> Result<BmsonChart, BmsonError> {
    let json = bytes.strip_prefix(&UTF8_BOM).unwrap_or(bytes);
    let value: Value = serde_json::from_slice(json)?;
    let root = value.as_object().ok_or(BmsonError::NotAnObject)?;
    let info = object_of(root, "info")?.ok_or(BmsonError::MissingInfo)?;

    Ok(BmsonChart {
        version: string_of(root, "version")?,
        info: info_of(info)?,
        lines: items_of(root, "lines", |v| Ok(BarLine { y: pulse_of(v, "y")? }))?,
        bpm_events: items_of(root, "bpm_events", |v| Ok(BpmEvent { y: pulse_of(v, "y")?, bpm: number_of(v, "bpm", 0.0)? }))?,
        stop_events: items_of(root, "stop_events", |v| Ok(StopEvent { y: pulse_of(v, "y")?, duration: integer_of(v, "duration", 0)? }))?,
        scroll_events: items_of(root, "scroll_events", |v| Ok(ScrollEvent { y: pulse_of(v, "y")?, rate: number_of(v, "rate", 1.0)? }))?,
        sound_channels: items_of(root, "sound_channels", |v| Ok(SoundChannel { name: string_of(v, "name")?, notes: notes_of(v)? }))?,
        key_channels: items_of(root, "key_channels", |v| Ok(KeyChannel { name: string_of(v, "name")?, notes: notes_of(v)? }))?,
        mine_channels: items_of(root, "mine_channels", |v| Ok(MineChannel { name: string_of(v, "name")?, notes: mines_of(v)? }))?,
        bga: match object_of(root, "bga")? {
            Some(bga) => bga_of(bga)?,
            None => Bga::default(),
        },
        md5: hex(Md5::digest(bytes).as_slice()),
        sha256: hex(Sha256::digest(bytes).as_slice()),
    })
}

fn info_of(info: &Map<String, Value>) -> Result<BmsonInfo, BmsonError> {
    let init_bpm = number_of(info, "init_bpm", 0.0)?;
    if !init_bpm.is_finite() || init_bpm <= 0.0 {
        return Err(BmsonError::InvalidInitBpm { bpm: init_bpm });
    }
    Ok(BmsonInfo {
        title: string_of(info, "title")?,
        subtitle: string_of(info, "subtitle")?,
        artist: string_of(info, "artist")?,
        subartists: strings_of(info, "subartists")?,
        genre: string_of(info, "genre")?,
        mode_hint: string_of(info, "mode_hint")?,
        chart_name: string_of(info, "chart_name")?,
        judge_rank: integer_of(info, "judge_rank", DEFAULT_JUDGE_RANK)?,
        total: number_of(info, "total", DEFAULT_TOTAL_PERCENT)?,
        init_bpm,
        level: integer_of(info, "level", 0)?,
        back_image: string_of(info, "back_image")?,
        eyecatch_image: string_of(info, "eyecatch_image")?,
        banner_image: string_of(info, "banner_image")?,
        preview_music: string_of(info, "preview_music")?,
        resolution: integer_of(info, "resolution", DEFAULT_RESOLUTION)?,
        ln_type: integer_of(info, "ln_type", 0)?,
    })
}

fn notes_of(channel: &Map<String, Value>) -> Result<Vec<BmsonNote>, BmsonError> {
    items_of(channel, "notes", |v| {
        Ok(BmsonNote {
            x: integer_of(v, "x", 0)?,
            y: pulse_of(v, "y")?,
            l: integer_of(v, "l", 0)?,
            c: flag_of(v, "c")?,
            t: integer_of(v, "t", 0)?,
            up: flag_of(v, "up")?,
        })
    })
}

fn mines_of(channel: &Map<String, Value>) -> Result<Vec<MineNote>, BmsonError> {
    items_of(channel, "notes", |v| Ok(MineNote { x: integer_of(v, "x", 0)?, y: pulse_of(v, "y")?, damage: number_of(v, "damage", 0.0)? }))
}

fn bga_of(bga: &Map<String, Value>) -> Result<Bga, BmsonError> {
    let event = |v: &Map<String, Value>| Ok(BgaEvent { y: pulse_of(v, "y")?, id: integer_of(v, "id", 0)? });
    Ok(Bga {
        header: items_of(bga, "bga_header", |v| Ok(BgaHeader { id: integer_of(v, "id", 0)?, name: string_of(v, "name")? }))?,
        base: items_of(bga, "bga_events", event)?,
        layer: items_of(bga, "layer_events", event)?,
        poor: items_of(bga, "poor_events", event)?,
    })
}

fn present<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    map.get(key).filter(|v| !v.is_null())
}

fn object_of<'a>(map: &'a Map<String, Value>, key: &'static str) -> Result<Option<&'a Map<String, Value>>, BmsonError> {
    match present(map, key) {
        None => Ok(None),
        Some(Value::Object(o)) => Ok(Some(o)),
        Some(_) => Err(BmsonError::InvalidField { field: key }),
    }
}

fn items_of<T, F>(map: &Map<String, Value>, key: &'static str, mut item: F) -> Result<Vec<T>, BmsonError>
where
    F: FnMut(&Map<String, Value>) -> Result<T, BmsonError>,
{
    let Some(value) = present(map, key) else {
        return Ok(Vec::new());
    };
    let Value::Array(items) = value else {
        return Err(BmsonError::InvalidField { field: key });
    };
    items.iter().map(|v| v.as_object().ok_or(BmsonError::InvalidField { field: key }).and_then(&mut item)).collect()
}

fn strings_of(map: &Map<String, Value>, key: &'static str) -> Result<Vec<String>, BmsonError> {
    let Some(value) = present(map, key) else {
        return Ok(Vec::new());
    };
    let Value::Array(items) = value else {
        return Err(BmsonError::InvalidField { field: key });
    };
    items.iter().map(|v| v.as_str().map(str::to_owned).ok_or(BmsonError::InvalidField { field: key })).collect()
}

fn string_of(map: &Map<String, Value>, key: &'static str) -> Result<String, BmsonError> {
    match present(map, key) {
        None => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(BmsonError::InvalidField { field: key }),
    }
}

fn number_of(map: &Map<String, Value>, key: &'static str, default: f64) -> Result<f64, BmsonError> {
    match present(map, key) {
        None => Ok(default),
        Some(v) => v.as_f64().ok_or(BmsonError::InvalidField { field: key }),
    }
}

fn integer_of(map: &Map<String, Value>, key: &'static str, default: i64) -> Result<i64, BmsonError> {
    match present(map, key) {
        None => Ok(default),
        Some(v) => v.as_i64().or_else(|| v.as_f64().filter(|f| f.is_finite()).map(|f| f as i64)).ok_or(BmsonError::InvalidField { field: key }),
    }
}

fn pulse_of(map: &Map<String, Value>, key: &'static str) -> Result<i64, BmsonError> {
    match integer_of(map, key, 0)? {
        y if y < 0 => Err(BmsonError::InvalidField { field: key }),
        y => Ok(y),
    }
}

fn flag_of(map: &Map<String, Value>, key: &'static str) -> Result<bool, BmsonError> {
    match present(map, key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(BmsonError::InvalidField { field: key }),
    }
}
