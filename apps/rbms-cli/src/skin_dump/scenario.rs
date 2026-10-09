//! The game state a skin is dumped against.
//!
//! A Lua skin reads `main_state` while it builds its screen, so what it comes to depends on the
//! chart, the score and the connection it finds. A dump has no game, so a scenario describes one: a
//! [`MapHost`] read from JSON, whose maps are keyed by property id.
//!
//! The built-in scenarios are JSON assets next to this crate (`scenarios/`). `base.json` holds what
//! every screen is dumped with, and the file of the screen the document belongs to is laid over it:
//! maps are merged key by key and the later file wins. A scenario file given on the command line
//! replaces all of that.
//!
//! A scenario is a JSON object whose keys are [`MapHost`]'s tables. `MapHost` itself reads whatever
//! it is handed and leaves out what it does not know, so a misspelt table or a file of the wrong
//! shape would dump a skin against an emptier game than the one asked for without saying so. Every
//! scenario is therefore held to [`SCENARIO_KEYS`] before it becomes a host.
//!
//! What the files hold, and which survey finding each value answers:
//!
//! - `base.json`: the first of the six difficulty options on and the other five off (a decide body
//!   indexes a colour table by the one that is on and fails without one); the chart has a BGA, a
//!   stage file and a back image; the chart's title, artist, genre and level, its BPM range and the
//!   date; and the texts a skin reads: player, rival, chart texts, search word, skin name and
//!   author, customisation labels, ranking names, targets, folder, table, version, IR name, hashes.
//! - `play.json`: autoplay off, the chart loaded, the 7-key option on, timers 40 and 41 running
//!   since the scene began, and the high-speed-fix image index 55 set to MAIN: a play skin's cover
//!   sliders index the table `adjustedCover()` returns for modes 2 to 4 and fail on a nil without
//!   it. The cover amount and the current BPM are filled in for the same functions.
//! - `select.json`: the song bar option on, the start-input timer running, and the song browser's
//!   static classification, which decides which conditions the loader settles once.
//! - `decide.json`: the start-input timer running.
//! - `result.json`: a cleared result, a clear type (the result body connects a function on it, and
//!   without it the connection is nil), and the result screen's static classification. It has no
//!   course stage titles, because a result body treats a non-empty first title as a course.
//! - `course_result.json`: the same, plus the course option and the first four stage titles.

use std::path::Path;

use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_24KEYS,
    SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_RESULT,
};
use rbms_skin::property::MapHost;
use serde_json::Value;

const BASE: &str = include_str!("../../scenarios/base.json");
const PLAY: &str = include_str!("../../scenarios/play.json");
const SELECT: &str = include_str!("../../scenarios/select.json");
const DECIDE: &str = include_str!("../../scenarios/decide.json");
const RESULT: &str = include_str!("../../scenarios/result.json");
const COURSE_RESULT: &str = include_str!("../../scenarios/course_result.json");

/// The keys a scenario may hold: the tables and values of [`MapHost`] that are read from a file.
pub const SCENARIO_KEYS: [&str; 22] = [
    "booleans",
    "static_booleans",
    "static_screen",
    "integers",
    "image_indices",
    "rates",
    "floats",
    "texts",
    "offsets",
    "timers",
    "now_us",
    "pressed_keys",
    "screen",
    "gauge",
    "gauge_type",
    "judges",
    "score",
    "score_best",
    "score_rival",
    "volume_system",
    "volume_key",
    "volume_background",
];

/// The screen a document draws, as far as the scenarios tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Play,
    Select,
    Decide,
    Result,
    CourseResult,
    /// Key configuration, skin selection and anything unknown: the base scenario alone.
    Other,
}

impl Screen {
    pub fn of(skin_type: i32) -> Self {
        match skin_type {
            SKIN_TYPE_PLAY_7KEYS..=SKIN_TYPE_PLAY_9KEYS | SKIN_TYPE_PLAY_24KEYS..=SKIN_TYPE_PLAY_24KEYS_BATTLE => Self::Play,
            SKIN_TYPE_MUSIC_SELECT => Self::Select,
            SKIN_TYPE_DECIDE => Self::Decide,
            SKIN_TYPE_RESULT => Self::Result,
            SKIN_TYPE_COURSE_RESULT => Self::CourseResult,
            _ => Self::Other,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::Select => "select",
            Self::Decide => "decide",
            Self::Result => "result",
            Self::CourseResult => "course_result",
            Self::Other => "base",
        }
    }

    const fn layer(self) -> Option<&'static str> {
        match self {
            Self::Play => Some(PLAY),
            Self::Select => Some(SELECT),
            Self::Decide => Some(DECIDE),
            Self::Result => Some(RESULT),
            Self::CourseResult => Some(COURSE_RESULT),
            Self::Other => None,
        }
    }
}

/// One scenario, ready to be turned into a host for each document dumped against it.
#[derive(Debug, Clone)]
pub struct Scenario {
    /// Where it came from, for the report.
    pub label: String,
    document: Value,
}

fn parse(text: &str, source: &str) -> Result<Value, String> {
    let document: Value = serde_json::from_str(text).map_err(|e| format!("{source}: {e}"))?;
    let Value::Object(tables) = &document else {
        return Err(format!("{source}: a scenario is a JSON object keyed by {}", SCENARIO_KEYS.join(", ")));
    };
    let unknown: Vec<&str> = tables.keys().map(String::as_str).filter(|key| !SCENARIO_KEYS.contains(key)).collect();
    if !unknown.is_empty() {
        return Err(format!("{source}: {} is not a scenario key; the keys are {}", unknown.join(", "), SCENARIO_KEYS.join(", ")));
    }
    Ok(document)
}

/// Lays `layer` over `base`: objects are merged key by key, anything else is replaced.
fn merge(base: &mut Value, layer: Value) {
    match (base, layer) {
        (Value::Object(base), Value::Object(layer)) => {
            for (key, value) in layer {
                match base.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, layer) => *base = layer,
    }
}

impl Scenario {
    /// The built-in scenario for a document of `screen`.
    pub fn builtin(screen: Screen) -> Result<Self, String> {
        let mut document = parse(BASE, "scenarios/base.json")?;
        if let Some(layer) = screen.layer() {
            merge(&mut document, parse(layer, &format!("scenarios/{}.json", screen.label()))?);
        }
        let scenario = Self { label: format!("built-in {}", screen.label()), document };
        scenario.host()?;
        Ok(scenario)
    }

    /// A scenario read from a file in the shape of [`MapHost`].
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("scenario {} not read: {e}", path.display()))?;
        let scenario = Self { label: path.display().to_string(), document: parse(&text, &path.display().to_string())? };
        scenario.host()?;
        Ok(scenario)
    }

    /// A fresh host: its recorded calls and volumes start empty for every document.
    pub fn host(&self) -> Result<MapHost, String> {
        serde_json::from_value(self.document.clone()).map_err(|e| format!("{}: {e}", self.label))
    }
}
