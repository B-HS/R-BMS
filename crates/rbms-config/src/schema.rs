use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeAutoShift, clamp_bottom_shiftable};
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;
use serde::{Deserialize, Serialize};

use crate::audio::AudioOptions;
use crate::judge::ScoreTarget;

/// Schema version this build writes. A file without a `schema_version` field is
/// [`LEGACY_SCHEMA_VERSION`] and is migrated on load.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Schema that carried one JUDGE WIDTH percentage for every judge tier of both key and scratch
/// lanes, before the per-tier rows split it into six.
pub const SINGLE_JUDGE_WIDTH_SCHEMA_VERSION: u32 = 1;

/// Version of the flat pre-migration file (`settings.ron` plus its `folders.ron` / `tables.ron`
/// siblings), which carried no version field at all.
pub const LEGACY_SCHEMA_VERSION: u32 = 0;

/// Slowest note scroll the HI-SPEED row can hold.
pub const HISPEED_MIN: f64 = 0.5;

/// Fastest note scroll the HI-SPEED row can hold.
pub const HISPEED_MAX: f64 = 10.0;

/// One left/right step on the HI-SPEED row.
pub const HISPEED_STEP: f64 = 0.25;

/// Smallest fraction of the lane the LIFT and LANE COVER rows can hide.
pub const LANE_SHADE_MIN: f32 = 0.0;

/// Largest fraction of the lane the LIFT and LANE COVER rows can hide.
pub const LANE_SHADE_MAX: f32 = 0.9;

/// One left/right step on the LIFT and LANE COVER rows.
pub const LANE_SHADE_STEP: f32 = 0.05;

/// Earliest judge offset the JUDGE OFFSET row can hold.
pub const JUDGE_OFFSET_MIN_MS: i32 = -200;

/// Latest judge offset the JUDGE OFFSET row can hold.
pub const JUDGE_OFFSET_MAX_MS: i32 = 200;

/// One left/right step on the JUDGE OFFSET row.
pub const JUDGE_OFFSET_STEP_MS: i32 = 5;

/// Narrowest judge windows the JUDGE WIDTH row can ask for, as a percentage of the chart's own.
pub const JUDGE_RATE_MIN_PERCENT: i32 = 50;

/// Widest judge windows the JUDGE WIDTH row can ask for, as a percentage of the chart's own.
pub const JUDGE_RATE_MAX_PERCENT: i32 = 200;

/// One left/right step on the JUDGE WIDTH row.
pub const JUDGE_RATE_STEP_PERCENT: i32 = 5;

/// Judge width that leaves the chart's own windows untouched, and the value a fresh install holds.
pub const JUDGE_RATE_DEFAULT_PERCENT: i32 = 100;

/// How many judge tiers a JUDGE WIDTH percentage covers: PGREAT, GREAT and GOOD. BAD and the empty
/// POOR band never widen, so they have no row. Pinned against `rbms_play::JUDGE_WIDTH_TIER_COUNT`
/// by the player rather than depending on the play crate from here.
pub const JUDGE_WIDTH_TIER_COUNT: usize = 3;

/// JUDGE WIDTH percentages that leave every widenable tier at the width the chart states.
pub const UNMODIFIED_JUDGE_RATES: [i32; JUDGE_WIDTH_TIER_COUNT] = [JUDGE_RATE_DEFAULT_PERCENT; JUDGE_WIDTH_TIER_COUNT];

/// Shortest long-note release margin the LN MARGIN row can ask for, as a percentage of the mode's.
pub const LN_MARGIN_MIN_PERCENT: i32 = 50;

/// Longest long-note release margin the LN MARGIN row can ask for.
pub const LN_MARGIN_MAX_PERCENT: i32 = 200;

/// One left/right step on the LN MARGIN row.
pub const LN_MARGIN_STEP_PERCENT: i32 = 5;

/// Long-note margin that leaves the mode's own release window untouched.
pub const LN_MARGIN_DEFAULT_PERCENT: i32 = 100;

/// TOTAL value that means "use the chart's own", and the floor of the TOTAL row.
pub const TOTAL_FROM_CHART: f64 = 0.0;

/// One left/right step on the TOTAL row.
pub const TOTAL_STEP: f64 = 10.0;

/// Note scroll a fresh install starts at.
pub const DEFAULT_HISPEED: f64 = 2.0;

/// Skin a fresh install starts on.
pub const DEFAULT_SKIN: &str = "NORMAL";

/// The id an unconfigured client submits under. Mirrors the score server's own guest id; the player
/// pins the two together with a test rather than depending on the IR crate from here.
pub const DEFAULT_PLAYER_ID: &str = "guest";

/// A user-added difficulty table: a display name and a `location` (an http(s) URL to a
/// header/data json, or a local file path).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSource {
    pub name: String,
    pub location: String,
}

/// Everything the player remembers between runs, in one versioned document.
///
/// The groups mirror the settings screen's tabs. Every group carries a serde default, so a file
/// written by an older build (or hand-edited down to a fragment) still loads with the missing
/// halves falling back to this build's defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub play: PlayOptions,
    pub judge: JudgeOptions,
    pub display: DisplayOptions,
    pub audio: AudioOptions,
    pub network: NetworkOptions,
    pub library: LibraryOptions,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            schema_version: CURRENT_SCHEMA_VERSION,
            play: PlayOptions::default(),
            judge: JudgeOptions::default(),
            display: DisplayOptions::default(),
            audio: AudioOptions::default(),
            network: NetworkOptions::default(),
            library: LibraryOptions::default(),
        }
    }
}

impl Config {
    /// Pull every value back into the range its settings row can produce and stamp the current
    /// schema version. Applied to whatever comes off disk or down from an account, so a
    /// hand-edited or foreign file can never drive the engine outside its supported ranges.
    pub fn sanitise(&mut self) {
        self.schema_version = CURRENT_SCHEMA_VERSION;
        self.play.hispeed = self.play.hispeed.clamp(HISPEED_MIN, HISPEED_MAX);
        self.play.lift = self.play.lift.clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
        self.play.cover = self.play.cover.clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
        self.play.total_override = self.play.total_override.max(TOTAL_FROM_CHART);
        self.judge.sanitise();
        self.display.skin = if self.display.skin.trim().is_empty() { DEFAULT_SKIN.to_string() } else { self.display.skin.to_ascii_uppercase() };
        self.audio.sanitise();
    }
}

/// The PLAY and GAUGE tabs: how the chart is dealt out and how the gauge treats it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayOptions {
    pub autoplay: bool,
    pub hispeed: f64,
    pub constant_speed: bool,
    #[serde(with = "note_option_token")]
    pub random: NoteOption,
    #[serde(with = "gauge_kind_token")]
    pub gauge: GaugeKind,
    pub lift: f32,
    pub cover: f32,
    pub scratch_left: bool,
    pub scratch_auto: bool,
    pub total_override: f64,
    pub auto_replay: bool,
}

impl Default for PlayOptions {
    fn default() -> Self {
        PlayOptions {
            autoplay: true,
            hispeed: DEFAULT_HISPEED,
            constant_speed: false,
            random: NoteOption::Off,
            gauge: GaugeKind::Normal,
            lift: LANE_SHADE_MIN,
            cover: LANE_SHADE_MIN,
            scratch_left: false,
            scratch_auto: false,
            total_override: TOTAL_FROM_CHART,
            auto_replay: true,
        }
    }
}

/// The JUDGE tab: when an input counts as on time, which note a press takes, and how the gauge
/// behind it is built and allowed to move.
///
/// The three judge widths and the long-note margin are stored per tier and per lane kind exactly as
/// the reference implementation configures them, because widening any one of them is what makes a
/// run a custom-judge run (`BMSPlayer.java:208-214`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct JudgeOptions {
    pub offset_ms: i32,
    pub auto_offset: bool,
    /// `[PGREAT, GREAT, GOOD]` JUDGE WIDTH percentages for key lanes.
    pub judge_rate_key: [i32; JUDGE_WIDTH_TIER_COUNT],
    /// The same three percentages for scratch lanes.
    pub judge_rate_scratch: [i32; JUDGE_WIDTH_TIER_COUNT],
    /// LN MARGIN as a percentage of the mode's own long-note release window.
    pub longnote_margin_rate: i32,
    #[serde(with = "judge_algorithm_token")]
    pub judge_algorithm: JudgeAlgorithm,
    #[serde(with = "ln_mode_token")]
    pub ln_mode: LnMode,
    /// Gauge table to play on, or `None` to take the one the chart's mode selects.
    #[serde(with = "gauge_set_token")]
    pub gauge_set: Option<GaugeSetId>,
    #[serde(with = "gauge_auto_shift_token")]
    pub gauge_auto_shift: GaugeAutoShift,
    /// The floor a per-frame auto-shift may drop the gauge selection to.
    #[serde(with = "gauge_kind_token")]
    pub bottom_shiftable_gauge: GaugeKind,
    #[serde(with = "target_token")]
    pub target: ScoreTarget,
}

impl Default for JudgeOptions {
    fn default() -> Self {
        JudgeOptions {
            offset_ms: 0,
            auto_offset: false,
            judge_rate_key: UNMODIFIED_JUDGE_RATES,
            judge_rate_scratch: UNMODIFIED_JUDGE_RATES,
            longnote_margin_rate: LN_MARGIN_DEFAULT_PERCENT,
            judge_algorithm: JudgeAlgorithm::default(),
            ln_mode: LnMode::default(),
            gauge_set: None,
            gauge_auto_shift: GaugeAutoShift::default(),
            bottom_shiftable_gauge: GaugeKind::AssistEasy,
            target: ScoreTarget::default(),
        }
    }
}

impl JudgeOptions {
    /// Pull every value back into the range its row can produce.
    pub fn sanitise(&mut self) {
        self.offset_ms = self.offset_ms.clamp(JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_MAX_MS);
        for rate in self.judge_rate_key.iter_mut().chain(self.judge_rate_scratch.iter_mut()) {
            *rate = (*rate).clamp(JUDGE_RATE_MIN_PERCENT, JUDGE_RATE_MAX_PERCENT);
        }
        self.longnote_margin_rate = self.longnote_margin_rate.clamp(LN_MARGIN_MIN_PERCENT, LN_MARGIN_MAX_PERCENT);
        self.bottom_shiftable_gauge = clamp_bottom_shiftable(self.bottom_shiftable_gauge);
    }

    /// Spread the one JUDGE WIDTH percentage an older file held over every tier of both lane kinds,
    /// which is what that single row meant.
    pub fn spread_uniform_judge_rate(&mut self, rate: i32) {
        self.judge_rate_key = [rate; JUDGE_WIDTH_TIER_COUNT];
        self.judge_rate_scratch = [rate; JUDGE_WIDTH_TIER_COUNT];
    }
}

/// The DISPLAY tab: what is drawn and with which assets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayOptions {
    pub bga: bool,
    /// Bundled skin name, uppercased on load.
    pub skin: String,
    pub font_path: Option<String>,
    pub score_graph: bool,
    pub replay_analysis: bool,
    pub debug: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        DisplayOptions { bga: true, skin: DEFAULT_SKIN.to_string(), font_path: None, score_graph: true, replay_analysis: true, debug: false }
    }
}

/// The NETWORK tab: the score server, the account signed in to it, and what is shared with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkOptions {
    /// IR score server base URL. `None` = offline.
    pub server_url: Option<String>,
    /// Player id submitted to the score server.
    pub player_id: String,
    /// Bearer token from the last successful IR login/register. `None` = signed out (guest).
    /// Cleared by LOGOUT. The password that produced it is never stored.
    pub ir_token: Option<String>,
    /// Login id the stored token belongs to, shown in the ACCOUNT row.
    pub ir_login_id: Option<String>,
    /// Address kept only to prefill the REGISTER form; display-only, never used to authenticate.
    pub ir_email: Option<String>,
    /// Mirror the local settings + key config to the account with the SYNC SETTINGS actions.
    pub sync_settings: bool,
    /// Upload the saved replay of a ranked submission right after the score lands.
    pub auto_upload_replay: bool,
    /// Cached rival player ids, refreshed from the server on login and after a rival edit.
    pub rivals: Vec<String>,
}

impl Default for NetworkOptions {
    fn default() -> Self {
        NetworkOptions {
            server_url: None,
            player_id: DEFAULT_PLAYER_ID.to_string(),
            ir_token: None,
            ir_login_id: None,
            ir_email: None,
            sync_settings: false,
            auto_upload_replay: true,
            rivals: Vec::new(),
        }
    }
}

/// Where the songs come from and how the select screen previews them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryOptions {
    /// Song-library folders. Every folder is scanned and the results merged into one song list, so
    /// the library can span multiple directories.
    pub folders: Vec<String>,
    /// Last song folder opened before the multi-folder list existed, kept so an old file still
    /// seeds the list on the first run after the upgrade.
    pub songs_folder: Option<String>,
    /// Play the focused song's `#PREVIEW` clip on the select screen.
    pub preview: bool,
    /// User-added difficulty tables.
    pub tables: Vec<TableSource>,
}

impl Default for LibraryOptions {
    fn default() -> Self {
        LibraryOptions { folders: Vec::new(), songs_folder: None, preview: true, tables: Vec::new() }
    }
}

/// Stores the gauge as its settings token rather than as a Rust variant name, so the file stays
/// readable and the account sync blob keeps the vocabulary the score server already speaks.
mod gauge_kind_token {
    use rbms_judge::GaugeKind;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &GaugeKind, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(crate::gauge::gauge_token(*value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<GaugeKind, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::gauge::gauge_from_name(&token))
    }
}

/// Stores the note option as the label the settings row shows, which is also what the pre-migration
/// file held.
mod note_option_token {
    use rbms_chart::shuffle::NoteOption;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &NoteOption, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.label())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NoteOption, D::Error> {
        let label = String::deserialize(deserializer)?;
        Ok(NoteOption::from_str(&label))
    }
}

/// Stores the judge algorithm under the reference implementation's own constant name.
mod judge_algorithm_token {
    use rbms_judge::algorithm::JudgeAlgorithm;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &JudgeAlgorithm, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(crate::judge::algorithm_token(*value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<JudgeAlgorithm, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::judge::algorithm_from_token(&token))
    }
}

/// Stores the long-note flavour as the token the LN MODE row shows.
mod ln_mode_token {
    use rbms_judge::ln::LnMode;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &LnMode, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(crate::judge::ln_mode_token(*value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<LnMode, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::judge::ln_mode_from_token(&token))
    }
}

/// Stores the gauge table as its data-file key, or `AUTO` for the one the chart's mode selects.
mod gauge_set_token {
    use rbms_judge::gauge_tables::GaugeSetId;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &Option<GaugeSetId>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(crate::judge::gauge_set_token(*value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<GaugeSetId>, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::judge::gauge_set_from_token(&token))
    }
}

/// Stores the gauge auto-shift mode as a separator-free token.
mod gauge_auto_shift_token {
    use rbms_judge::gauge::GaugeAutoShift;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &GaugeAutoShift, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(crate::judge::gauge_auto_shift_token(*value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<GaugeAutoShift, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::judge::gauge_auto_shift_from_token(&token))
    }
}

/// Stores the score target as a separator-free token, so the label may be reworded without moving
/// what is on disk.
mod target_token {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::judge::ScoreTarget;

    pub fn serialize<S: Serializer>(value: &ScoreTarget, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ScoreTarget, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::judge::target_from_token(&token))
    }
}
