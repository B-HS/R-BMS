use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeAutoShift, clamp_bottom_shiftable};
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;
use rbms_skin::dst::{OffsetSource, SkinOffset};
use rbms_skin::loader::SkinUserConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::audio::AudioOptions;
use crate::judge::ScoreTarget;
use crate::options::{FixHiSpeed, LaneOption, PlayEscape};
use crate::sort::SortMode;
use crate::window::{WindowMode, WindowResolution};

/// Schema version this build writes. A file without a `schema_version` field is
/// [`LEGACY_SCHEMA_VERSION`] and is migrated on load.
pub const CURRENT_SCHEMA_VERSION: u32 = 6;

/// Last schema of a build that had no score server unless the player typed one in. Every build
/// writes the whole configuration on each change, so a file of such a schema states
/// `server_url: None` for every player who never touched the row: it says nothing about a choice.
/// Reading such a file gives it [`DEFAULT_SERVER_URL`] once ([`Config::adopt_default_server`]);
/// from the next schema on an empty row is the player's own choice to play offline, and is kept.
pub const NO_DEFAULT_SERVER_SCHEMA_VERSION: u32 = 5;

/// The score server a player is signed up to without typing an address: the R-BMS site, whose
/// IR routes are under `/api`, which the client puts in front of every path.
pub const DEFAULT_SERVER_URL: &str = "https://bms.hyuns.uk/api";

/// Last schema whose JUDGE OFFSET was added to the instant an input was judged at: a positive
/// offset judged every input that much later, for a player who presses early, and left the notes
/// where the sound is. From the next schema on the offset is the reference's `judgetiming`
/// (`LaneRenderer.java:300-301`): how far ahead of the song the notes are drawn, with the
/// judgement left where the sound is. The two are the same distance between the note a player
/// sees on the line and the input that is on time, counted from opposite ends, so a value read
/// from such a schema is turned round once ([`Config::sanitise`]) and the notes and the judgement
/// stay as far apart as the player had set them.
pub const INPUT_JUDGE_OFFSET_SCHEMA_VERSION: u32 = 4;

/// Last schema written while the player shipped a skin bundle of its own: a DISPLAY preset named
/// after it, the documents a first run selected out of its folder, the choices shared by every
/// screen of a bundle and a flag recording the install. Reading it drops all four.
pub const BUNDLED_SKIN_SCHEMA_VERSION: u32 = 2;

/// Last schema whose files held a LETTERBOX value the player never chose. Those builds saved the
/// whole configuration on every change, so a file states `letterbox: false` whether or not the row
/// was ever touched, and a skin is authored for one shape that a window of another would distort.
/// Reading such a file turns the row on once; a file of a later schema keeps what it states.
pub const UNCHOSEN_LETTERBOX_SCHEMA_VERSION: u32 = 3;

/// Schema that carried one JUDGE WIDTH percentage for every judge tier of both key and scratch
/// lanes, before the per-tier rows split it into six.
pub const SINGLE_JUDGE_WIDTH_SCHEMA_VERSION: u32 = 1;

/// Version of the flat pre-migration file (`settings.ron` plus its `folders.ron` / `tables.ron`
/// siblings), which carried no version field at all.
pub const LEGACY_SCHEMA_VERSION: u32 = 0;

/// Slowest note scroll the HI-SPEED row can hold (`PlayConfig.java:18`).
pub const HISPEED_MIN: f64 = 0.01;

/// Fastest note scroll the HI-SPEED row can hold (`PlayConfig.java:19`).
pub const HISPEED_MAX: f64 = 20.0;

/// One left/right step on the HI-SPEED row as a fresh install has it, and the value the HI-SPEED
/// STEP row starts on (`PlayConfig.java:54`).
pub const HISPEED_STEP: f64 = 0.25;

/// The scroll speed one step of `by` away from `hispeed`, or `None` when there is no such speed.
///
/// The bounds are tested exclusively, as they are in the reference implementation
/// (`LaneRenderer.java:236-245` applies a step only while `0 < new < 20`): a step that would land on
/// or past either end is not taken at all rather than clamped onto it. Both routes to the row — the
/// settings screen and the in-play key — ask here, so the two cannot end up with different ceilings
/// and leave a speed the one can reach and the other cannot move off.
pub fn stepped_hispeed(hispeed: f64, by: f64) -> Option<f64> {
    let next = hispeed + by;
    if next <= 0.0 || next >= HISPEED_MAX {
        return None;
    }
    Some(next.max(HISPEED_MIN))
}

/// Smallest step the HI-SPEED STEP row can hold. Narrower than the reference implementation's own
/// `HISPEEDMARGIN` range, which starts at a step of zero: a zero step cannot move the row at all.
pub const HISPEED_STEP_MIN: f64 = 0.01;

/// Largest step the HI-SPEED STEP row can hold. Narrower than the reference implementation's own
/// ceiling of ten, which would overshoot the whole HI-SPEED range in two presses.
pub const HISPEED_STEP_MAX: f64 = 1.0;

/// One left/right step on the HI-SPEED STEP row.
pub const HISPEED_STEP_STEP: f64 = 0.01;

/// Shortest note travel time the GREEN NUMBER row can hold, in milliseconds (`PlayConfig.java:27`).
pub const DURATION_MIN_MS: i32 = 1;

/// Longest note travel time the GREEN NUMBER row can hold, in milliseconds (`PlayConfig.java:26`).
pub const DURATION_MAX_MS: i32 = 10_000;

/// Note travel time a fresh install holds, in milliseconds (`PlayConfig.java:24`): a green number
/// of 300.
pub const DEFAULT_DURATION_MS: i32 = 500;

/// Smallest fraction of the lane the LIFT, LANE COVER and HIDDEN+ rows can hide.
pub const LANE_SHADE_MIN: f32 = 0.0;

/// Largest fraction of the lane the LIFT, LANE COVER and HIDDEN+ rows can hide.
pub const LANE_SHADE_MAX: f32 = 1.0;

/// One left/right step on the LIFT, LANE COVER and HIDDEN+ rows (`PlayConfig.java:91`).
pub const LANE_SHADE_STEP: f32 = 0.01;

/// Smallest fine step the COVER FINE STEP row can hold.
pub const LANE_SHADE_FINE_STEP_MIN: f32 = 0.0001;

/// Largest fine step the COVER FINE STEP row can hold, which is the coarse step.
pub const LANE_SHADE_FINE_STEP_MAX: f32 = 0.01;

/// One left/right step on the COVER FINE STEP row.
pub const LANE_SHADE_FINE_STEP_STEP: f32 = 0.0001;

/// The fine lane-shade step a fresh install holds (`PlayConfig.java:87`).
pub const DEFAULT_LANE_SHADE_FINE_STEP: f32 = 0.001;

/// Lowest fraction of the screen height the judge text may be placed at. Zero means the skin's own
/// placement is kept, which is what every skin did before the row existed.
pub const JUDGE_TEXT_Y_MIN: f32 = 0.0;

/// Highest fraction of the screen height the judge text may be placed at.
pub const JUDGE_TEXT_Y_MAX: f32 = 1.0;

/// One left/right step on the JUDGE TEXT Y row.
pub const JUDGE_TEXT_Y_STEP: f32 = 0.01;

/// Judge text placement that leaves the skin's own alone.
pub const JUDGE_TEXT_Y_FROM_SKIN: f32 = 0.0;

/// Quietest the hover preview can be played at.
pub const PREVIEW_VOLUME_MIN: f32 = 0.0;

/// Loudest the hover preview can be played at.
pub const PREVIEW_VOLUME_MAX: f32 = 1.0;

/// One left/right step on the PREVIEW VOLUME row.
pub const PREVIEW_VOLUME_STEP: f32 = 0.05;

/// Gain a fresh install plays the hover preview at, which is the fixed gain the browser played it
/// at before the row existed, so opening the settings is what changes what an installation hears.
pub const DEFAULT_PREVIEW_VOLUME: f32 = 0.85;

/// Shortest hover-preview fade the row can hold, which is no fade at all.
pub const PREVIEW_FADE_MIN_MS: u32 = 0;

/// Longest hover-preview fade the row can hold.
pub const PREVIEW_FADE_MAX_MS: u32 = 1000;

/// One left/right step on the PREVIEW FADE row.
pub const PREVIEW_FADE_STEP_MS: u32 = 50;

/// Hover-preview fade a fresh install holds.
pub const DEFAULT_PREVIEW_FADE_MS: u32 = 200;

/// Shortest window the KEY DEBOUNCE row can hold, which is none: every change of a key is played.
pub const KEY_DEBOUNCE_MIN_MS: u32 = 0;

/// Longest window the KEY DEBOUNCE row can hold, the reference's own bound for it
/// (`PlayModeConfig.java:118`).
pub const KEY_DEBOUNCE_MAX_MS: u32 = 100;

/// One left/right step on the KEY DEBOUNCE row.
pub const KEY_DEBOUNCE_STEP_MS: u32 = 1;

/// The window a fresh install holds a lane key down for after it is pressed: the reference's
/// keyboard (`PlayModeConfig.java:270`), LR2's `inputinterval` and the controller here
/// (`gamepad::DEFAULT_DEBOUNCE_MS`) all ship 16 ms.
pub const DEFAULT_KEY_DEBOUNCE_MS: u32 = 16;

/// Shortest window the PAD DEBOUNCE row can hold, which is none: every change of a button is played.
/// The key config file holds the value (`pad.debounce_ms`), and the controller layer holds a value
/// written there by hand to this row's two ends.
pub const PAD_DEBOUNCE_MIN_MS: u32 = 0;

/// Longest window the PAD DEBOUNCE row can hold, the reference's own bound for a controller
/// (`PlayModeConfig.java:148`).
pub const PAD_DEBOUNCE_MAX_MS: u32 = 100;

/// One left/right step on the PAD DEBOUNCE row, the same as on the KEY DEBOUNCE row.
pub const PAD_DEBOUNCE_STEP_MS: u32 = 1;

/// The window a fresh install holds a controller button down for after it is pressed: LR2's
/// `inputinterval` (`LR2_configsave.cpp:970`) and the reference's controller
/// (`BMControllerInputProcessor.java:61`) both ship 16 ms.
pub const DEFAULT_PAD_DEBOUNCE_MS: u32 = 16;

/// Lowest judge offset the JUDGE OFFSET row can hold: the notes drawn this far behind the song.
pub const JUDGE_OFFSET_MIN_MS: i32 = -200;

/// Highest judge offset the JUDGE OFFSET row can hold: the notes drawn this far ahead of the song.
pub const JUDGE_OFFSET_MAX_MS: i32 = 200;

/// One left/right step on the JUDGE OFFSET row: a millisecond, which is the step the browser's own
/// panel and the keys of a run move the offset by.
pub const JUDGE_OFFSET_STEP_MS: i32 = 1;

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

/// Lowest and highest share of the chart's ceiling the TARGET RATE row can ask for, in percent.
pub const TARGET_RATE_MIN_PERCENT: i32 = 1;
pub const TARGET_RATE_MAX_PERCENT: i32 = 100;

/// One left/right step on the TARGET RATE row.
pub const TARGET_RATE_STEP_PERCENT: i32 = 1;

/// The share the RATE CUSTOM target asks for until the player chooses one: a little under RATE AAA.
pub const TARGET_RATE_DEFAULT_PERCENT: i32 = 85;

/// TOTAL value that means "use the chart's own", and the floor of the TOTAL row.
pub const TOTAL_FROM_CHART: f64 = 0.0;

/// One left/right step on the TOTAL row.
pub const TOTAL_STEP: f64 = 10.0;

/// Note scroll a fresh install starts at.
pub const DEFAULT_HISPEED: f64 = 2.0;

/// Skin a fresh install starts on.
pub const DEFAULT_SKIN: &str = "NORMAL";

/// Whether an ordinary window keeps the screen's own proportions on a fresh install, and for a
/// settings file written before the LETTERBOX row existed. A file of a schema after
/// [`UNCHOSEN_LETTERBOX_SCHEMA_VERSION`] that states the row either way is read as it states it.
pub const DEFAULT_LETTERBOX: bool = true;

/// Every screen a skin document can declare, named for the skin menu, indexed by the `SkinType` id
/// the document writes in its `type` field (`SkinType.java`).
///
/// Screens this build draws no skin for are listed too. A player whose document declares one then
/// sees it named on the row, and is told it is unsupported, rather than finding their skin missing
/// from the list with no explanation.
pub const SKIN_SCREEN_LABELS: &[&str] = &[
    "PLAY 7KEYS",
    "PLAY 5KEYS",
    "PLAY 14KEYS",
    "PLAY 10KEYS",
    "PLAY 9KEYS",
    "MUSIC SELECT",
    "DECIDE",
    "RESULT",
    "KEY CONFIG",
    "SKIN SELECT",
    "SOUND SET",
    "THEME",
    "PLAY 7KEYS BATTLE",
    "PLAY 5KEYS BATTLE",
    "PLAY 9KEYS BATTLE",
    "COURSE RESULT",
    "PLAY 24KEYS",
    "PLAY 24KEYS DOUBLE",
    "PLAY 24KEYS BATTLE",
];

/// The screen the skin menu starts on, which is the seven-key play screen.
pub const DEFAULT_SKIN_SCREEN: i32 = 0;

/// Directory documents are looked for in when the row names none, relative to the settings file.
pub const DEFAULT_SKIN_FOLDER: &str = "skin";

/// The name of one screen type, or `None` for an id no `SkinType` declares.
pub fn skin_screen_label(skin_type: i32) -> Option<&'static str> {
    usize::try_from(skin_type).ok().and_then(|at| SKIN_SCREEN_LABELS.get(at)).copied()
}

/// One document's customisation as the settings file stores it.
///
/// The rows are addressed by the names the document itself gave them, so a document that gains or
/// loses a row on its next version keeps the choices made for the rows it still has. This is the
/// stored half of [`SkinUserConfig`], which is what the skin loader is handed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SkinCustomisation {
    /// Customisation row name to the option id it is switched to.
    pub properties: BTreeMap<String, i32>,
    /// File slot name to the file chosen for it.
    pub filepaths: BTreeMap<String, String>,
    /// Offset id to the nudge applied to it.
    pub offsets: BTreeMap<i32, SkinOffset>,
}

/// The nudges a document's own offset ids resolve to, which is how the renderer reads what the
/// player moved without knowing anything about the settings file.
impl OffsetSource for SkinCustomisation {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.get(&id).copied()
    }
}

/// The skin menu: where documents are looked for, which screen is being configured, the document
/// each screen is drawn with, and what the player chose inside each one.
///
/// A screen with no entry in `selected` is drawn with the pack's document for it, and by the built-in
/// screen when no pack is named or the pack has none, which is what a fresh install has for every
/// screen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SkinOptions {
    /// Directory documents are looked for in. `None` looks in [`DEFAULT_SKIN_FOLDER`] beside the
    /// settings file.
    pub folder: Option<String>,
    /// The skin pack: one folder holding a document for each screen. Every screen with no entry in
    /// `selected` is drawn with the document of the pack that declares it. `None` names no pack.
    pub pack: Option<String>,
    /// The screen the skin menu is configuring, as a `SkinType` id.
    pub screen: i32,
    /// Document each screen is drawn with, keyed by its `SkinType` id.
    pub selected: BTreeMap<i32, String>,
    /// What the player chose inside each document, keyed by the path it was found under.
    pub custom: BTreeMap<String, SkinCustomisation>,
}

impl Default for SkinOptions {
    fn default() -> Self {
        SkinOptions { folder: None, pack: None, screen: DEFAULT_SKIN_SCREEN, selected: BTreeMap::new(), custom: BTreeMap::new() }
    }
}

impl SkinOptions {
    /// The document the player picked for `screen` by hand, or `None` when the screen is left to the
    /// pack and the built-in screen.
    pub fn document(&self, screen: i32) -> Option<&str> {
        self.selected.get(&screen).map(String::as_str).filter(|path| !path.is_empty())
    }

    /// The folder of the skin pack, or `None` when no pack is named.
    pub fn pack_folder(&self) -> Option<&str> {
        self.pack.as_deref().map(str::trim).filter(|pack| !pack.is_empty())
    }

    /// Draw `screen` with `path`, or leave it to the pack and the built-in screen when `path` is
    /// `None`.
    pub fn select(&mut self, screen: i32, path: Option<String>) {
        match path.filter(|path| !path.is_empty()) {
            Some(path) => self.selected.insert(screen, path),
            None => self.selected.remove(&screen),
        };
    }

    /// What the player chose inside one document, read-only and without creating an entry for a
    /// document nothing has been chosen in yet.
    pub fn customisation(&self, path: &str) -> Option<&SkinCustomisation> {
        self.custom.get(path)
    }

    /// What the player chose inside one document, created empty the first time it is edited.
    pub fn customise(&mut self, path: &str) -> &mut SkinCustomisation {
        self.custom.entry(path.to_owned()).or_default()
    }

    /// Drop every choice made inside one document, so it loads as its author shipped it.
    pub fn forget(&mut self, path: &str) {
        self.custom.remove(path);
    }

    /// What the skin loader should be handed for one document: the path it was found under and the
    /// choices the player made inside it.
    pub fn user_config(&self, path: &str) -> SkinUserConfig {
        let stored = self.custom.get(path);
        SkinUserConfig {
            path: path.to_owned(),
            properties: stored.map(|entry| entry.properties.clone()).unwrap_or_default(),
            filepaths: stored.map(|entry| entry.filepaths.clone()).unwrap_or_default(),
            offsets: stored.map(|entry| entry.offsets.clone()).unwrap_or_default(),
        }
    }

    /// Pull a hand-edited document back into what the rows can produce: an unnamed screen falls back
    /// to the one a fresh install configures, a blank folder means the default folder, and a blank
    /// pack means no pack.
    pub fn sanitise(&mut self) {
        if skin_screen_label(self.screen).is_none() {
            self.screen = DEFAULT_SKIN_SCREEN;
        }
        self.folder = self.folder.take().filter(|folder| !folder.trim().is_empty());
        self.pack = self.pack.take().filter(|pack| !pack.trim().is_empty());
        self.selected.retain(|screen, path| skin_screen_label(*screen).is_some() && !path.trim().is_empty());
    }
}

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
    pub skin: SkinOptions,
}

impl Config {
    /// What a player who has never run the player starts from: the shipped values, with the score
    /// server set to [`DEFAULT_SERVER_URL`].
    ///
    /// [`Config::default`] has no server, because it is what every test, the blanking of an upload
    /// and a file this build is too old to read start from, and none of those may reach the network.
    pub fn first_run() -> Config {
        let mut config = Config::default();
        config.network.server_url = Some(DEFAULT_SERVER_URL.to_string());
        config
    }

    /// Give a configuration read from a schema that had no server by default
    /// ([`NO_DEFAULT_SERVER_SCHEMA_VERSION`]) the default one, when it names none. A server the file
    /// names is kept, and a file of a later schema is left as it is.
    pub fn adopt_default_server(&mut self, from_schema: u32) {
        if from_schema <= NO_DEFAULT_SERVER_SCHEMA_VERSION && self.network.server_url.is_none() {
            self.network.server_url = Some(DEFAULT_SERVER_URL.to_string());
        }
    }
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
            skin: SkinOptions::default(),
        }
    }
}

impl Config {
    /// Pull every value back into the range its settings row can produce and stamp the current
    /// schema version. Applied to whatever comes off disk or down from an account, so a
    /// hand-edited or foreign file can never drive the engine outside its supported ranges.
    ///
    /// The stamp is what makes a document one of this schema's, so the one value whose meaning
    /// the schema changed is carried over here, off the version the document still states: a
    /// JUDGE OFFSET from a schema that added it to the input
    /// ([`INPUT_JUDGE_OFFSET_SCHEMA_VERSION`]) is turned round. Every reader ends here, the file
    /// loader and an account download alike, and a document already stamped is left as it is.
    pub fn sanitise(&mut self) {
        if self.schema_version <= INPUT_JUDGE_OFFSET_SCHEMA_VERSION {
            self.judge.offset_ms = JudgeOptions::offset_drawn_ahead_for(self.judge.offset_ms);
        }
        self.schema_version = CURRENT_SCHEMA_VERSION;
        self.play.hispeed = self.play.hispeed.clamp(HISPEED_MIN, HISPEED_MAX);
        self.play.hispeed_step = self.play.hispeed_step.clamp(HISPEED_STEP_MIN, HISPEED_STEP_MAX);
        self.play.duration_ms = self.play.duration_ms.clamp(DURATION_MIN_MS, DURATION_MAX_MS);
        self.play.lift = self.play.lift.clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
        self.play.cover = self.play.cover.clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
        self.play.hidden = self.play.hidden.clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
        self.play.lanecover_step_fine = self.play.lanecover_step_fine.clamp(LANE_SHADE_FINE_STEP_MIN, LANE_SHADE_FINE_STEP_MAX);
        self.play.total_override = self.play.total_override.max(TOTAL_FROM_CHART);
        self.display.judge_text_y = self.display.judge_text_y.clamp(JUDGE_TEXT_Y_MIN, JUDGE_TEXT_Y_MAX);
        self.library.preview_volume = self.library.preview_volume.clamp(PREVIEW_VOLUME_MIN, PREVIEW_VOLUME_MAX);
        self.library.preview_fade_ms = self.library.preview_fade_ms.clamp(PREVIEW_FADE_MIN_MS, PREVIEW_FADE_MAX_MS);
        self.play.key_debounce_ms = self.play.key_debounce_ms.clamp(KEY_DEBOUNCE_MIN_MS, KEY_DEBOUNCE_MAX_MS);
        self.judge.sanitise();
        self.display.skin = if self.display.skin.trim().is_empty() { DEFAULT_SKIN.to_string() } else { self.display.skin.to_ascii_uppercase() };
        self.audio.sanitise();
        self.skin.sanitise();
        self.network.ir_profiles.retain(|profile| !profile.base_url.trim().is_empty());
    }
}

/// The play options: how the chart is dealt out, how its notes scroll and how the gauge treats it.
/// Most of them are rows of the play menu.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayOptions {
    pub autoplay: bool,
    pub hispeed: f64,
    /// One HI-SPEED step, so a player can trade reach for precision.
    pub hispeed_step: f64,
    pub constant_speed: bool,
    /// Which tempo a fixed green number is pinned to. Independent of `constant_speed`: that one
    /// ignores the chart's tempo changes outright, this one keeps the note travel time constant
    /// across them.
    #[serde(with = "crate::options::fix_hispeed_token")]
    pub fix_hispeed: FixHiSpeed,
    /// Hold the note travel time from one chart to the next. While it is on `duration_ms` is what
    /// is stored and `hispeed` is worked out from it for every chart, at the tempo `fix_hispeed`
    /// names and under the lane cover that is up, the way the reference does while its own speed
    /// fix is on (`LaneRenderer.java:154, 206-215`). Off, `hispeed` is what is stored and
    /// `duration_ms` is not read.
    pub keep_green_number: bool,
    /// The note travel time held while `keep_green_number` is on, in milliseconds
    /// (`PlayConfig.duration`). The green number a skin shows is three fifths of it
    /// ([`crate::speed::green_of_duration`]).
    pub duration_ms: i32,
    #[serde(with = "note_option_token")]
    pub random: NoteOption,
    #[serde(with = "gauge_kind_token")]
    pub gauge: GaugeKind,
    /// What is done to the lanes as a whole, which the note shuffle above is applied inside of.
    #[serde(with = "crate::options::lane_option_token")]
    pub lane_option: LaneOption,
    /// Draw long notes as the plain notes an older skin would have shown.
    pub legacy_note: bool,
    pub lift: f32,
    /// Whether the LIFT value is applied. Kept apart from the value so switching it off and back on
    /// does not lose the height that was dialled in (`PlayConfig.java:74`).
    pub enable_lift: bool,
    pub cover: f32,
    /// Whether the LANE COVER value is applied (`PlayConfig.java:66`).
    pub enable_cover: bool,
    /// Fraction of the lane hidden from the judge line upwards, the counterpart of the cover.
    pub hidden: f32,
    /// Whether the HIDDEN+ value is applied (`PlayConfig.java:82`).
    pub enable_hidden: bool,
    /// The finer of the two lane-shade steps, used with the modifier held (`PlayConfig.java:87`).
    pub lanecover_step_fine: f32,
    pub scratch_left: bool,
    pub scratch_auto: bool,
    /// KEY DEBOUNCE: how long, in milliseconds, a lane key of the keyboard is held down after a press
    /// of it was played, as LR2's `inputinterval` holds a key. A press is played the instant it
    /// comes; a release inside the window is played the instant the window ends, if the key still
    /// reads up then. 0 plays every change.
    pub key_debounce_ms: u32,
    pub total_override: f64,
    pub auto_replay: bool,
    /// What an Escape during a run has to be before it abandons the run.
    #[serde(with = "crate::options::play_escape_token")]
    pub play_escape: PlayEscape,
}

impl Default for PlayOptions {
    fn default() -> Self {
        PlayOptions {
            autoplay: true,
            hispeed: DEFAULT_HISPEED,
            hispeed_step: HISPEED_STEP,
            constant_speed: false,
            fix_hispeed: FixHiSpeed::default(),
            keep_green_number: false,
            duration_ms: DEFAULT_DURATION_MS,
            random: NoteOption::Off,
            gauge: GaugeKind::Normal,
            lane_option: LaneOption::default(),
            legacy_note: false,
            lift: LANE_SHADE_MIN,
            enable_lift: false,
            cover: LANE_SHADE_MIN,
            enable_cover: true,
            hidden: LANE_SHADE_MIN,
            enable_hidden: false,
            lanecover_step_fine: DEFAULT_LANE_SHADE_FINE_STEP,
            scratch_left: false,
            scratch_auto: false,
            key_debounce_ms: DEFAULT_KEY_DEBOUNCE_MS,
            total_override: TOTAL_FROM_CHART,
            auto_replay: true,
            play_escape: PlayEscape::default(),
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
    /// JUDGE OFFSET: how many milliseconds ahead of the song the notes are drawn
    /// (`PlayerConfig.judgetiming`). At +10 a note is on the judge line 10 ms before its sound and
    /// before the instant a press of it is on time; neither of those moves. A player who presses
    /// late raises it, one who presses early lowers it.
    pub offset_ms: i32,
    /// AUTO ADJUST: let every input of a run played by hand move `offset_ms`
    /// (`PlayerConfig.notesDisplayTimingAutoAdjust`).
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
    /// What the run is paced against when [`JudgeOptions::target`] names a score that is not there
    /// (no record yet, no account score, no rival score). It is always a target that needs nothing
    /// but the chart ([`ScoreTarget::FALLBACKS`]).
    #[serde(with = "target_fallback_token")]
    pub target_fallback: ScoreTarget,
    /// The share of the chart's ceiling, in percent, that the RATE CUSTOM target asks for.
    pub target_rate_percent: i32,
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
            target_fallback: ScoreTarget::default(),
            target_rate_percent: TARGET_RATE_DEFAULT_PERCENT,
        }
    }
}

impl JudgeOptions {
    /// The JUDGE OFFSET that leaves the notes and the judgement as far apart as an offset of
    /// `input_judged_later_ms` did on a schema that added it to the instant an input was judged at
    /// ([`INPUT_JUDGE_OFFSET_SCHEMA_VERSION`]): the same distance the other way.
    pub(crate) const fn offset_drawn_ahead_for(input_judged_later_ms: i32) -> i32 {
        input_judged_later_ms.saturating_neg()
    }

    /// Pull every value back into the range its row can produce.
    pub fn sanitise(&mut self) {
        self.offset_ms = self.offset_ms.clamp(JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_MAX_MS);
        for rate in self.judge_rate_key.iter_mut().chain(self.judge_rate_scratch.iter_mut()) {
            *rate = (*rate).clamp(JUDGE_RATE_MIN_PERCENT, JUDGE_RATE_MAX_PERCENT);
        }
        self.longnote_margin_rate = self.longnote_margin_rate.clamp(LN_MARGIN_MIN_PERCENT, LN_MARGIN_MAX_PERCENT);
        self.bottom_shiftable_gauge = clamp_bottom_shiftable(self.bottom_shiftable_gauge);
        if !self.target_fallback.is_always_known() {
            self.target_fallback = ScoreTarget::default();
        }
        self.target_rate_percent = self.target_rate_percent.clamp(TARGET_RATE_MIN_PERCENT, TARGET_RATE_MAX_PERCENT);
    }

    /// Spread the one JUDGE WIDTH percentage an older file held over every tier of both lane kinds,
    /// which is what that single row meant.
    pub fn spread_uniform_judge_rate(&mut self, rate: i32) {
        self.judge_rate_key = [rate; JUDGE_WIDTH_TIER_COUNT];
        self.judge_rate_scratch = [rate; JUDGE_WIDTH_TIER_COUNT];
    }
}

/// The DISPLAY tab: what is drawn and with which assets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayOptions {
    pub bga: bool,
    /// Name of the built-in note-field layout, uppercased on load.
    pub skin: String,
    pub font_path: Option<String>,
    pub score_graph: bool,
    /// Draw the gauge, timing and judge-distribution graphs on the result screen.
    pub result_graphs: bool,
    pub replay_analysis: bool,
    /// Show the note travel time to the bottom of the lane cover next to the green number.
    pub show_white_number: bool,
    /// Where the judge text sits as a fraction of the screen height, or
    /// [`JUDGE_TEXT_Y_FROM_SKIN`] to keep the skin's own placement.
    pub judge_text_y: f32,
    /// Fit the logical screen inside the window at its own aspect ratio rather than stretching it.
    /// On unless a settings file says otherwise: a skin is authored for one shape, and a window of
    /// another would draw it distorted.
    pub letterbox: bool,
    /// Inner size the window opens at, and is put back to when the mode returns to a window.
    #[serde(with = "crate::window::window_resolution_token")]
    pub window_resolution: WindowResolution,
    /// Whether the window is an ordinary one or covers its whole monitor.
    #[serde(with = "crate::window::window_mode_token")]
    pub window_mode: WindowMode,
    /// Lay a five-key chart out on its own lane widths rather than on the seven-key ones.
    pub five_key_layout: bool,
    pub debug: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        DisplayOptions {
            bga: true,
            skin: DEFAULT_SKIN.to_string(),
            font_path: None,
            score_graph: true,
            result_graphs: true,
            replay_analysis: true,
            show_white_number: false,
            judge_text_y: JUDGE_TEXT_Y_FROM_SKIN,
            letterbox: DEFAULT_LETTERBOX,
            window_resolution: WindowResolution::default(),
            window_mode: WindowMode::default(),
            five_key_layout: false,
            debug: false,
        }
    }
}

impl DisplayOptions {
    /// Whether the logical screen is fitted inside the window at its own proportions. A window that
    /// covers a monitor takes the monitor's shape, which is not always 16:9, so it is always fitted;
    /// an ordinary window follows the LETTERBOX row.
    pub fn fits_screen_shape(&self) -> bool {
        self.letterbox || self.window_mode == WindowMode::Borderless
    }
}

/// The NETWORK tab: the score server, the account signed in to it, and what is shared with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkOptions {
    /// IR score server base URL. `None` = offline, which from schema 6 on is a choice the player
    /// made: a new install is given [`DEFAULT_SERVER_URL`] ([`Config::first_run`]).
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
    /// Extra score servers a score is submitted to alongside `server_url`. Empty keeps the
    /// single-server behaviour the fields above describe.
    pub ir_profiles: Vec<IrProfile>,
}

/// One score server a play can be submitted to, so a player can hold accounts on several at once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IrProfile {
    /// Label shown in the settings row and on the ranking panel's profile tab.
    pub name: String,
    /// Base URL of this server.
    pub base_url: String,
    /// Bearer token for this server. `None` = signed out on it.
    pub token: Option<String>,
    /// Submit to this server. A disabled profile is kept but skipped.
    pub enabled: bool,
}

impl Default for IrProfile {
    fn default() -> Self {
        IrProfile { name: String::new(), base_url: String::new(), token: None, enabled: true }
    }
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
            ir_profiles: Vec::new(),
        }
    }
}

/// Where the songs come from and how the select screen previews them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Gain the hover preview is played at, under the background bus.
    pub preview_volume: f32,
    /// How long the hover preview fades in and back out over.
    pub preview_fade_ms: u32,
    /// How the song list is ordered.
    #[serde(with = "crate::sort")]
    pub sort: SortMode,
    /// Show only the charts that have been marked as favourites.
    pub favorite_only: bool,
    /// User-added difficulty tables.
    pub tables: Vec<TableSource>,
    /// The ids of the tables every player is given ([`crate::DEFAULT_TABLES`]) that this player
    /// has been given already, whether or not they kept them: a table here is never given again.
    pub seeded_default_tables: Vec<String>,
}

impl Default for LibraryOptions {
    fn default() -> Self {
        LibraryOptions {
            folders: Vec::new(),
            songs_folder: None,
            preview: true,
            preview_volume: DEFAULT_PREVIEW_VOLUME,
            preview_fade_ms: DEFAULT_PREVIEW_FADE_MS,
            sort: SortMode::default(),
            favorite_only: false,
            tables: Vec::new(),
            seeded_default_tables: Vec::new(),
        }
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

/// Stores the fallback target the way the target is stored, and reads a target that is not always
/// known as the shipped one.
mod target_fallback_token {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::judge::ScoreTarget;

    pub fn serialize<S: Serializer>(value: &ScoreTarget, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ScoreTarget, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(crate::judge::target_fallback_from_token(&token))
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
