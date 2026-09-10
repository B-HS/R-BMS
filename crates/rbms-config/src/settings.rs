//! The settings screen's rows as data: one descriptor per row naming its tab, label, kind and
//! range, plus the two functions the screen needs — what a row shows and what one left/right step
//! does to it.
//!
//! Rows are named rather than numbered. Adding one is a single entry in [`SETTINGS`]; nothing else
//! has to be renumbered, and the screen never has to know which position a row happens to occupy.
//! Rows the running program owns rather than the document — a live account, a password held only in
//! memory, a skin forced on the command line — are marked [`SettingDescriptor::host_value`], and
//! the value returned here is what the document alone can say about them.

use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::{BOTTOM_SHIFTABLE_GAUGES, GaugeAutoShift};
use rbms_judge::ln::LnMode;

use crate::audio::{
    AUDIO_BUFFER_FRAMES_CHOICES, AUDIO_POLYPHONY_MAX_VOICES, AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_STEP_VOICES, AUDIO_SAMPLE_RATE_HZ_CHOICES,
    AUDIO_VOLUME_MAX_GAIN, AUDIO_VOLUME_MAX_PERCENT, AUDIO_VOLUME_MIN_GAIN, AUDIO_VOLUME_STEP_PERCENT, cycle_optional_u32, step_polyphony, step_volume,
    volume_percent,
};
use crate::judge::{
    GAUGE_AUTO_SHIFT_LABELS, GAUGE_SET_CYCLE, GAUGE_SET_LABELS, JUDGE_ALGORITHM_LABELS, LN_MODE_LABELS, ScoreTarget, TARGET_LABELS, gauge_set_token,
};
use crate::options;
use crate::schema::{
    Config, DEFAULT_PLAYER_ID, HISPEED_MAX, HISPEED_MIN, HISPEED_STEP, HISPEED_STEP_MAX, HISPEED_STEP_MIN, HISPEED_STEP_STEP, JUDGE_OFFSET_MAX_MS,
    JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_STEP_MS, JUDGE_RATE_MAX_PERCENT, JUDGE_RATE_MIN_PERCENT, JUDGE_RATE_STEP_PERCENT, JUDGE_TEXT_Y_MAX, JUDGE_TEXT_Y_MIN,
    JUDGE_TEXT_Y_STEP, LANE_SHADE_FINE_STEP_MAX, LANE_SHADE_FINE_STEP_MIN, LANE_SHADE_FINE_STEP_STEP, LANE_SHADE_MAX, LANE_SHADE_MIN, LANE_SHADE_STEP,
    LN_MARGIN_MAX_PERCENT, LN_MARGIN_MIN_PERCENT, LN_MARGIN_STEP_PERCENT, PREVIEW_FADE_MAX_MS, PREVIEW_FADE_MIN_MS, PREVIEW_FADE_STEP_MS, PREVIEW_VOLUME_MAX,
    PREVIEW_VOLUME_MIN, PREVIEW_VOLUME_STEP, SKIN_SCREEN_LABELS, TOTAL_FROM_CHART, TOTAL_STEP, skin_screen_label,
};
use crate::sort::SortMode;

/// Value of a row that is on.
pub const ON_VALUE: &str = "ON";

/// Value of a row that is off.
pub const OFF_VALUE: &str = "OFF";

/// Value of a row that runs something or opens a screen instead of holding a value.
pub const ACTION_VALUE: &str = ">";

/// Value of a row left to the chart, the backend or the system.
pub const AUTO_VALUE: &str = "AUTO";

/// Value of a row that is on whatever the system or the build ships.
pub const DEFAULT_VALUE: &str = "DEFAULT";

/// Value of a row pointed at something the user chose.
pub const CUSTOM_VALUE: &str = "CUSTOM";

/// Value of an optional text row that has not been filled in.
pub const NONE_VALUE: &str = "(none)";

/// Value of the password row while no password is held.
pub const UNSET_VALUE: &str = "(not set)";

/// Suffix of a row measured in percent.
pub const PERCENT_UNIT: &str = "%";

/// Suffix of a row measured in milliseconds.
pub const MILLISECOND_UNIT: &str = " MS";

/// Rows measured in whole units carry no suffix.
pub const NO_UNIT: &str = "";

/// A fraction of one read as whole percent.
pub const PERCENT_SCALE: f32 = 100.0;

/// TOTAL has no ceiling; the row only floors at [`TOTAL_FROM_CHART`], which means "use the chart's".
pub const TOTAL_MAX: f64 = f64::INFINITY;

/// Decimals the HI-SPEED row shows.
pub const HISPEED_DECIMALS: u8 = 2;

/// Decimals the TOTAL row shows.
pub const TOTAL_DECIMALS: u8 = 0;

/// Decimals the COVER FINE STEP row shows. Its whole range is under one percent, so a row measured
/// in percent would read as zero however far it was stepped.
pub const LANE_SHADE_FINE_DECIMALS: u8 = 4;

/// One left/right step on a volume row, as a gain.
pub const AUDIO_VOLUME_STEP_GAIN: f32 = AUDIO_VOLUME_STEP_PERCENT as f32 / AUDIO_VOLUME_MAX_PERCENT as f32;

/// Longest score-server URL the row is meant to hold.
pub const SERVER_URL_MAX_LEN: usize = 512;

/// Longest player id the row is meant to hold.
pub const PLAYER_ID_MAX_LEN: usize = 64;

/// Longest address the row is meant to hold.
pub const EMAIL_MAX_LEN: usize = 254;

/// Longest password the row is meant to hold.
pub const PASSWORD_MAX_LEN: usize = 128;

/// Position of the PGREAT tier in a JUDGE WIDTH array.
const PGREAT_TIER: usize = 0;

/// Position of the GREAT tier in a JUDGE WIDTH array.
const GREAT_TIER: usize = 1;

/// Position of the GOOD tier in a JUDGE WIDTH array.
const GOOD_TIER: usize = 2;

/// The gauges the GAUGE row steps through, in the order it shows them.
pub const GAUGE_CYCLE: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];

/// Names the GAUGE row shows, one per entry of [`GAUGE_CYCLE`].
pub const GAUGE_LABELS: &[&str] = &["ASSIST EASY", "EASY", "NORMAL", "HARD", "EX-HARD", "HAZARD"];

/// Names the BOTTOM SHIFTABLE row shows, one per entry of [`rbms_judge::gauge::BOTTOM_SHIFTABLE_GAUGES`].
const BOTTOM_SHIFTABLE_LABELS: &[&str] = &["ASSIST EASY", "EASY", "NORMAL"];

const SPEED_FIX_LABELS: &[&str] = &["FLOATING", "CONSTANT"];
const HISPEED_FIX_LABELS: &[&str] = &["OFF", "START", "MAX", "MAIN", "MIN"];
const LANE_OPTION_LABELS: &[&str] = &["OFF", "FLIP", "BATTLE", "BATTLE AUTO-SC"];
const PLAY_ESCAPE_LABELS: &[&str] = &["IMMEDIATE", "HOLD", "DOUBLE TAP"];
const SORT_LABELS: &[&str] = &["DEFAULT", "TITLE", "ARTIST", "BPM", "LENGTH", "LEVEL", "CLEAR", "SCORE", "MISS COUNT", "DURATION", "LAST UPDATE"];
const SCRATCH_SIDE_LABELS: &[&str] = &["RIGHT", "LEFT"];
const RANDOM_LABELS: &[&str] = &["OFF", "MIRROR", "RANDOM", "S-RANDOM", "R-RANDOM", "ROTATE", "H-RANDOM", "ALL-SCRATCH"];
const SKIN_LABELS: &[&str] = &["NORMAL", "WIDE"];
const AUDIO_DEVICE_LABELS: &[&str] = &[DEFAULT_VALUE];

/// What the SKIN row already knows it can hold: the built-in screen. The documents on disk are only
/// known once the folder has been walked, so the running program adds those.
const SKIN_DOCUMENT_LABELS: &[&str] = &[DEFAULT_VALUE];
const AUDIO_BUFFER_LABELS: &[&str] = &[AUTO_VALUE, "128", "192", "256", "384", "512", "768", "1024", "2048"];
const AUDIO_SAMPLE_RATE_LABELS: &[&str] = &[AUTO_VALUE, "44100", "48000", "88200", "96000"];

/// Bundled skin the SKIN row steps to from anything that is not itself.
const SKIN_ALTERNATE: &str = "WIDE";

/// Bundled skin the SKIN row steps back to.
const SKIN_PRIMARY: &str = "NORMAL";

/// Every row of the settings screen.
///
/// The ids are declared in list order, so [`SettingId::row_index`] is also the position of the row
/// in the whole list.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SettingId {
    Autoplay,
    HiSpeed,
    HiSpeedStep,
    SpeedFix,
    FixHiSpeed,
    Random,
    LaneOption,
    LegacyNote,
    FiveKeyLayout,
    PlayEscapeMode,
    Gauge,
    Lift,
    LaneCover,
    ScratchSide,
    ScratchAuto,
    JudgeOffset,
    Bga,
    KeyConfig,
    JudgeAlgorithm,
    JudgeWidthKeyPGreat,
    JudgeWidthKeyGreat,
    JudgeWidthKeyGood,
    JudgeWidthScratchPGreat,
    JudgeWidthScratchGreat,
    JudgeWidthScratchGood,
    LongNoteMargin,
    LnMode,
    GaugeSet,
    GaugeAutoShift,
    BottomShiftableGauge,
    Target,
    Total,
    Skin,
    SkinScreen,
    SkinDocument,
    SkinInfo,
    SkinReload,
    SkinReset,
    AutoCal,
    AutoReplay,
    DebugMode,
    Font,
    ScoreGraph,
    ResultGraphs,
    ReplayAnalysis,
    Preview,
    EnableLift,
    EnableCover,
    Hidden,
    EnableHidden,
    LaneCoverFineStep,
    WhiteNumber,
    JudgeTextY,
    Letterbox,
    Sort,
    FavoriteOnly,
    PreviewVolume,
    PreviewFade,
    ServerUrl,
    PlayerId,
    Account,
    Email,
    Password,
    Login,
    Register,
    Logout,
    SyncSettings,
    UploadSettings,
    DownloadSettings,
    AutoUploadReplay,
    Rivals,
    MasterVolume,
    KeyVolume,
    BgmVolume,
    SystemVolume,
    AudioDevice,
    AudioBuffer,
    AudioSampleRate,
    AudioPolyphony,
}

/// Rows the settings screen has.
pub const SETTING_COUNT: usize = 79;

impl SettingId {
    /// Every row, in declaration order.
    pub const ALL: [SettingId; SETTING_COUNT] = [
        SettingId::Autoplay,
        SettingId::HiSpeed,
        SettingId::HiSpeedStep,
        SettingId::SpeedFix,
        SettingId::FixHiSpeed,
        SettingId::Random,
        SettingId::LaneOption,
        SettingId::LegacyNote,
        SettingId::FiveKeyLayout,
        SettingId::PlayEscapeMode,
        SettingId::Gauge,
        SettingId::Lift,
        SettingId::LaneCover,
        SettingId::ScratchSide,
        SettingId::ScratchAuto,
        SettingId::JudgeOffset,
        SettingId::Bga,
        SettingId::KeyConfig,
        SettingId::JudgeAlgorithm,
        SettingId::JudgeWidthKeyPGreat,
        SettingId::JudgeWidthKeyGreat,
        SettingId::JudgeWidthKeyGood,
        SettingId::JudgeWidthScratchPGreat,
        SettingId::JudgeWidthScratchGreat,
        SettingId::JudgeWidthScratchGood,
        SettingId::LongNoteMargin,
        SettingId::LnMode,
        SettingId::GaugeSet,
        SettingId::GaugeAutoShift,
        SettingId::BottomShiftableGauge,
        SettingId::Target,
        SettingId::Total,
        SettingId::Skin,
        SettingId::SkinScreen,
        SettingId::SkinDocument,
        SettingId::SkinInfo,
        SettingId::SkinReload,
        SettingId::SkinReset,
        SettingId::AutoCal,
        SettingId::AutoReplay,
        SettingId::DebugMode,
        SettingId::Font,
        SettingId::ScoreGraph,
        SettingId::ResultGraphs,
        SettingId::ReplayAnalysis,
        SettingId::Preview,
        SettingId::EnableLift,
        SettingId::EnableCover,
        SettingId::Hidden,
        SettingId::EnableHidden,
        SettingId::LaneCoverFineStep,
        SettingId::WhiteNumber,
        SettingId::JudgeTextY,
        SettingId::Letterbox,
        SettingId::Sort,
        SettingId::FavoriteOnly,
        SettingId::PreviewVolume,
        SettingId::PreviewFade,
        SettingId::ServerUrl,
        SettingId::PlayerId,
        SettingId::Account,
        SettingId::Email,
        SettingId::Password,
        SettingId::Login,
        SettingId::Register,
        SettingId::Logout,
        SettingId::SyncSettings,
        SettingId::UploadSettings,
        SettingId::DownloadSettings,
        SettingId::AutoUploadReplay,
        SettingId::Rivals,
        SettingId::MasterVolume,
        SettingId::KeyVolume,
        SettingId::BgmVolume,
        SettingId::SystemVolume,
        SettingId::AudioDevice,
        SettingId::AudioBuffer,
        SettingId::AudioSampleRate,
        SettingId::AudioPolyphony,
    ];

    /// Position of the row in the whole settings list, which is also its index in [`SettingId::ALL`].
    pub fn row_index(self) -> usize {
        self as usize
    }

    /// The row at `index`, or `None` past the end of the list.
    pub fn from_row_index(index: usize) -> Option<SettingId> {
        SettingId::ALL.get(index).copied()
    }
}

/// The tabs the rows are grouped into, in the order the tab strip shows them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingTab {
    Play,
    Gauge,
    Judge,
    Display,
    Skin,
    Input,
    Network,
    Audio,
    Select,
}

impl SettingTab {
    /// Every tab, left to right.
    pub const ALL: [SettingTab; 9] = [
        SettingTab::Play,
        SettingTab::Gauge,
        SettingTab::Judge,
        SettingTab::Display,
        SettingTab::Skin,
        SettingTab::Input,
        SettingTab::Network,
        SettingTab::Audio,
        SettingTab::Select,
    ];

    /// Name shown on the tab strip.
    pub fn label(self) -> &'static str {
        match self {
            SettingTab::Play => "PLAY",
            SettingTab::Gauge => "GAUGE",
            SettingTab::Judge => "JUDGE",
            SettingTab::Display => "DISPLAY",
            SettingTab::Skin => "SKIN",
            SettingTab::Input => "INPUT",
            SettingTab::Network => "NETWORK",
            SettingTab::Audio => "AUDIO",
            SettingTab::Select => "SELECT",
        }
    }
}

/// What kind of value a row holds, and over what range.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SettingKind {
    /// On or off.
    Toggle,
    /// A whole number, shown with `unit` after it.
    IntRange { min: i32, max: i32, step: i32, unit: &'static str },
    /// A fractional number, shown to `decimals` places with `unit` after it.
    FloatRange { min: f64, max: f64, step: f64, decimals: u8, unit: &'static str },
    /// A fraction of one, shown as a whole percent.
    Percent { min: f32, max: f32, step: f32 },
    /// A fixed list of values stepped through in order. A row whose list is only known at run time
    /// (the enumerated output devices) declares the entries the program already knows.
    Cycle { values: &'static [&'static str] },
    /// Free text typed into the row. `max_len` is the longest value the row is meant to hold.
    Text { max_len: usize, secret: bool },
    /// Runs something or opens a screen; holds no value of its own.
    Action,
    /// Opens a file picker.
    FilePick,
    /// Reports something the row cannot change. It holds no value of its own — the running program
    /// fills it in — and stepping it does nothing.
    Info,
}

/// Everything the settings screen needs to know about one row.
#[derive(Clone, Copy, Debug)]
pub struct SettingDescriptor {
    pub id: SettingId,
    pub tab: SettingTab,
    pub label: &'static str,
    pub kind: SettingKind,
    pub help: &'static str,
    /// The running program owns this row's value: it reads a live session, a secret held only in
    /// memory, or an override that came from the command line. [`display_value`] answers with what
    /// the document alone can say, and the program replaces it.
    pub host_value: bool,
    /// Whether the row is shown at all for this configuration.
    pub visible: fn(&Config) -> bool,
}

/// What one call to [`adjust`] did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdjustOutcome {
    /// The document moved.
    Changed,
    /// The row is already at the end of its range, or the step landed on the value it held.
    Unchanged,
    /// The document was not touched: this row opens a screen, a picker or an editor, or reaches
    /// something only the running program can see.
    Action(SettingId),
}

fn always(_config: &Config) -> bool {
    true
}

/// Every row of the settings screen, tab by tab and top to bottom within a tab.
pub const SETTINGS: &[SettingDescriptor] = &[
    row(SettingId::Autoplay, SettingTab::Play, "AUTOPLAY", SettingKind::Toggle, "Let the game play the chart itself"),
    row(
        SettingId::HiSpeed,
        SettingTab::Play,
        "HI-SPEED",
        SettingKind::FloatRange { min: HISPEED_MIN, max: HISPEED_MAX, step: HISPEED_STEP, decimals: HISPEED_DECIMALS, unit: NO_UNIT },
        "How fast the notes scroll; one press moves it by HI-SPEED STEP",
    ),
    row(
        SettingId::HiSpeedStep,
        SettingTab::Play,
        "HI-SPEED STEP",
        SettingKind::FloatRange { min: HISPEED_STEP_MIN, max: HISPEED_STEP_MAX, step: HISPEED_STEP_STEP, decimals: HISPEED_DECIMALS, unit: NO_UNIT },
        "How far one HI-SPEED press moves the scroll",
    ),
    row(
        SettingId::SpeedFix,
        SettingTab::Play,
        "SPEED FIX",
        SettingKind::Cycle { values: SPEED_FIX_LABELS },
        "Whether the scroll speed follows the chart's tempo changes",
    ),
    row(
        SettingId::FixHiSpeed,
        SettingTab::Play,
        "HI-SPEED FIX",
        SettingKind::Cycle { values: HISPEED_FIX_LABELS },
        "Which tempo the note travel time is held constant against",
    ),
    row(SettingId::Random, SettingTab::Play, "RANDOM", SettingKind::Cycle { values: RANDOM_LABELS }, "How the lanes are shuffled"),
    unbuilt_row(
        SettingId::LaneOption,
        SettingTab::Play,
        "LANE OPTION",
        SettingKind::Cycle { values: LANE_OPTION_LABELS },
        "What is done to the sides of the chart as a whole",
    ),
    row(SettingId::LegacyNote, SettingTab::Play, "LEGACY NOTE", SettingKind::Toggle, "Draw long notes as plain notes"),
    row(SettingId::FiveKeyLayout, SettingTab::Play, "5KEYS LAYOUT", SettingKind::Toggle, "Give a five-key chart its own lane widths"),
    row(SettingId::AutoReplay, SettingTab::Play, "AUTO REPLAY", SettingKind::Toggle, "Save a replay of every run"),
    row(SettingId::PlayEscapeMode, SettingTab::Play, "ESCAPE", SettingKind::Cycle { values: PLAY_ESCAPE_LABELS }, "What Escape has to be to abandon a run"),
    row(SettingId::Gauge, SettingTab::Gauge, "GAUGE", SettingKind::Cycle { values: GAUGE_LABELS }, "Which gauge the run is cleared on"),
    row(
        SettingId::Total,
        SettingTab::Gauge,
        "TOTAL",
        SettingKind::FloatRange { min: TOTAL_FROM_CHART, max: TOTAL_MAX, step: TOTAL_STEP, decimals: TOTAL_DECIMALS, unit: NO_UNIT },
        "Override the chart's own gauge total",
    ),
    row(
        SettingId::JudgeOffset,
        SettingTab::Judge,
        "JUDGE OFFSET",
        SettingKind::IntRange { min: JUDGE_OFFSET_MIN_MS, max: JUDGE_OFFSET_MAX_MS, step: JUDGE_OFFSET_STEP_MS, unit: MILLISECOND_UNIT },
        "Shift every judgement earlier or later",
    ),
    row(SettingId::AutoCal, SettingTab::Judge, "AUTO CAL", SettingKind::Toggle, "Learn the judge offset from how you play"),
    row(
        SettingId::JudgeAlgorithm,
        SettingTab::Judge,
        "JUDGE ALGORITHM",
        SettingKind::Cycle { values: JUDGE_ALGORITHM_LABELS },
        "Which note a press takes when several are in range",
    ),
    judge_width_row(SettingId::JudgeWidthKeyPGreat, "JUDGE WIDTH KEY PG"),
    judge_width_row(SettingId::JudgeWidthKeyGreat, "JUDGE WIDTH KEY GR"),
    judge_width_row(SettingId::JudgeWidthKeyGood, "JUDGE WIDTH KEY GD"),
    judge_width_row(SettingId::JudgeWidthScratchPGreat, "JUDGE WIDTH SCR PG"),
    judge_width_row(SettingId::JudgeWidthScratchGreat, "JUDGE WIDTH SCR GR"),
    judge_width_row(SettingId::JudgeWidthScratchGood, "JUDGE WIDTH SCR GD"),
    row(
        SettingId::LongNoteMargin,
        SettingTab::Judge,
        "LN MARGIN",
        SettingKind::IntRange { min: LN_MARGIN_MIN_PERCENT, max: LN_MARGIN_MAX_PERCENT, step: LN_MARGIN_STEP_PERCENT, unit: PERCENT_UNIT },
        "Widen or narrow the long-note release window",
    ),
    row(SettingId::LnMode, SettingTab::Judge, "LN MODE", SettingKind::Cycle { values: LN_MODE_LABELS }, "What long notes the chart left unstated play as"),
    row(
        SettingId::GaugeSet,
        SettingTab::Judge,
        "GAUGE SET",
        SettingKind::Cycle { values: GAUGE_SET_LABELS },
        "Which gauge table the nine gauges are built from",
    ),
    row(
        SettingId::GaugeAutoShift,
        SettingTab::Judge,
        "GAUGE AUTO SHIFT",
        SettingKind::Cycle { values: GAUGE_AUTO_SHIFT_LABELS },
        "How the selected gauge may move during play",
    ),
    row(
        SettingId::BottomShiftableGauge,
        SettingTab::Judge,
        "BOTTOM SHIFTABLE",
        SettingKind::Cycle { values: BOTTOM_SHIFTABLE_LABELS },
        "The floor an auto-shift may drop the gauge to",
    ),
    row(SettingId::Target, SettingTab::Judge, "TARGET", SettingKind::Cycle { values: TARGET_LABELS }, "What the run is paced against"),
    host_row(SettingId::Skin, SettingTab::Display, "SKIN", SettingKind::Cycle { values: SKIN_LABELS }, "Which bundled skin the play screen uses"),
    row(SettingId::Font, SettingTab::Display, "FONT", SettingKind::FilePick, "Font the screens are drawn with"),
    row(SettingId::ScoreGraph, SettingTab::Display, "SCORE GRAPH", SettingKind::Toggle, "Draw the score graph on the result screen"),
    row(SettingId::ResultGraphs, SettingTab::Display, "RESULT GRAPHS", SettingKind::Toggle, "Draw the gauge and timing graphs on the result screen"),
    row(SettingId::ReplayAnalysis, SettingTab::Display, "REPLAY ANALYSIS", SettingKind::Toggle, "Allow stepping and slowing a replay"),
    row(SettingId::Preview, SettingTab::Display, "PREVIEW", SettingKind::Toggle, "Play the focused song on the song list"),
    row(
        SettingId::Lift,
        SettingTab::Display,
        "LIFT",
        SettingKind::Percent { min: LANE_SHADE_MIN, max: LANE_SHADE_MAX, step: LANE_SHADE_STEP },
        "Raise the judge line up the lane",
    ),
    row(SettingId::EnableLift, SettingTab::Display, "LIFT ON", SettingKind::Toggle, "Apply the LIFT height"),
    row(
        SettingId::LaneCover,
        SettingTab::Display,
        "LANE COVER",
        SettingKind::Percent { min: LANE_SHADE_MIN, max: LANE_SHADE_MAX, step: LANE_SHADE_STEP },
        "Hide the top of the lane",
    ),
    row(SettingId::EnableCover, SettingTab::Display, "LANE COVER ON", SettingKind::Toggle, "Apply the LANE COVER height"),
    row(
        SettingId::Hidden,
        SettingTab::Display,
        "HIDDEN+",
        SettingKind::Percent { min: LANE_SHADE_MIN, max: LANE_SHADE_MAX, step: LANE_SHADE_STEP },
        "Hide the bottom of the lane above the judge line",
    ),
    row(SettingId::EnableHidden, SettingTab::Display, "HIDDEN+ ON", SettingKind::Toggle, "Apply the HIDDEN+ height"),
    row(
        SettingId::LaneCoverFineStep,
        SettingTab::Display,
        "COVER FINE STEP",
        SettingKind::FloatRange {
            min: LANE_SHADE_FINE_STEP_MIN as f64,
            max: LANE_SHADE_FINE_STEP_MAX as f64,
            step: LANE_SHADE_FINE_STEP_STEP as f64,
            decimals: LANE_SHADE_FINE_DECIMALS,
            unit: NO_UNIT,
        },
        "How far one fine cover press moves the shade",
    ),
    row(SettingId::WhiteNumber, SettingTab::Display, "WHITE NUMBER", SettingKind::Toggle, "Show the travel time to the bottom of the cover"),
    row(
        SettingId::JudgeTextY,
        SettingTab::Display,
        "JUDGE TEXT Y",
        SettingKind::Percent { min: JUDGE_TEXT_Y_MIN, max: JUDGE_TEXT_Y_MAX, step: JUDGE_TEXT_Y_STEP },
        "Where the judgement text sits, or 0 for the skin's own place",
    ),
    row(SettingId::Letterbox, SettingTab::Display, "LETTERBOX", SettingKind::Toggle, "Keep the screen's aspect ratio inside the window"),
    row(SettingId::Bga, SettingTab::Display, "BGA", SettingKind::Toggle, "Show the chart's background animation"),
    row(SettingId::DebugMode, SettingTab::Display, "DEBUG MODE", SettingKind::Toggle, "Draw the frame and audio counters"),
    row(SettingId::SkinScreen, SettingTab::Skin, "SCREEN", SettingKind::Cycle { values: SKIN_SCREEN_LABELS }, "Which screen's skin the rows below configure"),
    row(SettingId::SkinDocument, SettingTab::Skin, "SKIN", SettingKind::Cycle { values: SKIN_DOCUMENT_LABELS }, "Which document draws this screen"),
    host_row(SettingId::SkinInfo, SettingTab::Skin, "LOADED", SettingKind::Info, "What the chosen document is, or why it is not being drawn"),
    row(SettingId::SkinReload, SettingTab::Skin, "RELOAD", SettingKind::Action, "Read the document again with the choices made here"),
    row(SettingId::SkinReset, SettingTab::Skin, "RESET", SettingKind::Action, "Drop every choice made in this document"),
    row(
        SettingId::ScratchSide,
        SettingTab::Input,
        "SCRATCH SIDE",
        SettingKind::Cycle { values: SCRATCH_SIDE_LABELS },
        "Which side the scratch lane is drawn on",
    ),
    row(SettingId::ScratchAuto, SettingTab::Input, "SCRATCH AUTO", SettingKind::Toggle, "Let the game play the scratch lane"),
    row(SettingId::KeyConfig, SettingTab::Input, "KEY CONFIG", SettingKind::Action, "Open the key editor"),
    row(
        SettingId::ServerUrl,
        SettingTab::Network,
        "SERVER URL",
        SettingKind::Text { max_len: SERVER_URL_MAX_LEN, secret: false },
        "Score server to submit to; empty is offline",
    ),
    row(
        SettingId::PlayerId,
        SettingTab::Network,
        "PLAYER ID",
        SettingKind::Text { max_len: PLAYER_ID_MAX_LEN, secret: false },
        "Id scores are submitted under",
    ),
    host_row(SettingId::Account, SettingTab::Network, "ACCOUNT", SettingKind::Action, "State of the signed-in account"),
    row(SettingId::Email, SettingTab::Network, "EMAIL", SettingKind::Text { max_len: EMAIL_MAX_LEN, secret: false }, "Address the register form uses"),
    host_row(
        SettingId::Password,
        SettingTab::Network,
        "PASSWORD",
        SettingKind::Text { max_len: PASSWORD_MAX_LEN, secret: true },
        "Password for the next sign-in; never stored",
    ),
    row(SettingId::Login, SettingTab::Network, "LOGIN", SettingKind::Action, "Sign in to the score server"),
    row(SettingId::Register, SettingTab::Network, "REGISTER", SettingKind::Action, "Create an account on the score server"),
    row(SettingId::Logout, SettingTab::Network, "LOGOUT", SettingKind::Action, "Forget the signed-in account"),
    row(SettingId::SyncSettings, SettingTab::Network, "SYNC SETTINGS", SettingKind::Toggle, "Mirror settings and keys to the account"),
    row(SettingId::UploadSettings, SettingTab::Network, "UPLOAD SETTINGS NOW", SettingKind::Action, "Send the local settings to the account"),
    row(SettingId::DownloadSettings, SettingTab::Network, "DOWNLOAD SETTINGS NOW", SettingKind::Action, "Take the account's settings"),
    row(SettingId::AutoUploadReplay, SettingTab::Network, "AUTO UPLOAD REPLAY", SettingKind::Toggle, "Send the replay after a ranked score"),
    row(SettingId::Rivals, SettingTab::Network, "RIVALS", SettingKind::Action, "Players shown next to you on the ranking"),
    row(
        SettingId::MasterVolume,
        SettingTab::Audio,
        "MASTER VOL",
        SettingKind::Percent { min: AUDIO_VOLUME_MIN_GAIN, max: AUDIO_VOLUME_MAX_GAIN, step: AUDIO_VOLUME_STEP_GAIN },
        "Gain applied after the buses are summed",
    ),
    row(
        SettingId::KeyVolume,
        SettingTab::Audio,
        "KEY VOL",
        SettingKind::Percent { min: AUDIO_VOLUME_MIN_GAIN, max: AUDIO_VOLUME_MAX_GAIN, step: AUDIO_VOLUME_STEP_GAIN },
        "Gain of the keysound bus",
    ),
    row(
        SettingId::BgmVolume,
        SettingTab::Audio,
        "BGM VOL",
        SettingKind::Percent { min: AUDIO_VOLUME_MIN_GAIN, max: AUDIO_VOLUME_MAX_GAIN, step: AUDIO_VOLUME_STEP_GAIN },
        "Gain of the background bus",
    ),
    row(
        SettingId::SystemVolume,
        SettingTab::Audio,
        "SYSTEM VOL",
        SettingKind::Percent { min: AUDIO_VOLUME_MIN_GAIN, max: AUDIO_VOLUME_MAX_GAIN, step: AUDIO_VOLUME_STEP_GAIN },
        "Gain of the interface bus",
    ),
    row(SettingId::AudioDevice, SettingTab::Audio, "AUDIO DEVICE", SettingKind::Cycle { values: AUDIO_DEVICE_LABELS }, "Output device to open"),
    row(SettingId::AudioBuffer, SettingTab::Audio, "BUFFER SIZE", SettingKind::Cycle { values: AUDIO_BUFFER_LABELS }, "Frames per output callback"),
    row(SettingId::AudioSampleRate, SettingTab::Audio, "SAMPLE RATE", SettingKind::Cycle { values: AUDIO_SAMPLE_RATE_LABELS }, "Rate the output runs at"),
    row(
        SettingId::AudioPolyphony,
        SettingTab::Audio,
        "POLYPHONY",
        SettingKind::IntRange {
            min: AUDIO_POLYPHONY_MIN_VOICES as i32,
            max: AUDIO_POLYPHONY_MAX_VOICES as i32,
            step: AUDIO_POLYPHONY_STEP_VOICES as i32,
            unit: NO_UNIT,
        },
        "Voices the mixer may sound at once",
    ),
    row(SettingId::Sort, SettingTab::Select, "SORT", SettingKind::Cycle { values: SORT_LABELS }, "How the song list is ordered"),
    row(SettingId::FavoriteOnly, SettingTab::Select, "FAVORITE ONLY", SettingKind::Toggle, "List only the charts marked as favourites"),
    row(
        SettingId::PreviewVolume,
        SettingTab::Select,
        "PREVIEW VOLUME",
        SettingKind::Percent { min: PREVIEW_VOLUME_MIN, max: PREVIEW_VOLUME_MAX, step: PREVIEW_VOLUME_STEP },
        "Gain the hover preview is played at",
    ),
    row(
        SettingId::PreviewFade,
        SettingTab::Select,
        "PREVIEW FADE",
        SettingKind::IntRange { min: PREVIEW_FADE_MIN_MS as i32, max: PREVIEW_FADE_MAX_MS as i32, step: PREVIEW_FADE_STEP_MS as i32, unit: MILLISECOND_UNIT },
        "How long the hover preview fades in and out over",
    ),
];

/// One of the six JUDGE WIDTH rows: same range, same help, one judge tier of one lane kind.
const fn judge_width_row(id: SettingId, label: &'static str) -> SettingDescriptor {
    row(
        id,
        SettingTab::Judge,
        label,
        SettingKind::IntRange { min: JUDGE_RATE_MIN_PERCENT, max: JUDGE_RATE_MAX_PERCENT, step: JUDGE_RATE_STEP_PERCENT, unit: PERCENT_UNIT },
        "Widen or narrow this judge tier's own window",
    )
}

const fn row(id: SettingId, tab: SettingTab, label: &'static str, kind: SettingKind, help: &'static str) -> SettingDescriptor {
    SettingDescriptor { id, tab, label, kind, help, host_value: false, visible: always }
}

const fn host_row(id: SettingId, tab: SettingTab, label: &'static str, kind: SettingKind, help: &'static str) -> SettingDescriptor {
    SettingDescriptor { id, tab, label, kind, help, host_value: true, visible: always }
}

/// A row whose value nothing acts on yet.
///
/// It keeps its descriptor — its label, its vocabulary and its place in the table — so the value can
/// still be stored and read back, and so building the feature is a one-line change here. It is not
/// shown, because a row a player can move that changes nothing about the run reads as a chosen
/// option rather than an unbuilt one. What is still missing for each is in the divergence ledger.
const fn unbuilt_row(id: SettingId, tab: SettingTab, label: &'static str, kind: SettingKind, help: &'static str) -> SettingDescriptor {
    SettingDescriptor { id, tab, label, kind, help, host_value: false, visible: never }
}

fn never(_config: &Config) -> bool {
    false
}

/// The descriptor of one row.
pub fn descriptor(id: SettingId) -> &'static SettingDescriptor {
    SETTINGS.iter().find(|row| row.id == id).expect("every setting id has a descriptor")
}

/// The rows a tab shows for this configuration, top to bottom.
pub fn tab_rows(tab: SettingTab, config: &Config) -> Vec<SettingId> {
    SETTINGS.iter().filter(|row| row.tab == tab && (row.visible)(config)).map(|row| row.id).collect()
}

/// The values a cycling row steps through, empty for a row of another kind.
pub fn cycle_values(id: SettingId) -> &'static [&'static str] {
    match descriptor(id).kind {
        SettingKind::Cycle { values } => values,
        _ => &[],
    }
}

fn unit_of(id: SettingId) -> &'static str {
    match descriptor(id).kind {
        SettingKind::IntRange { unit, .. } | SettingKind::FloatRange { unit, .. } => unit,
        SettingKind::Percent { .. } => PERCENT_UNIT,
        _ => NO_UNIT,
    }
}

fn on_off(on: bool) -> String {
    if on { ON_VALUE.to_string() } else { OFF_VALUE.to_string() }
}

fn cycle_label(id: SettingId, at: usize) -> String {
    cycle_values(id).get(at).copied().unwrap_or_default().to_string()
}

fn shade_value(fraction: f32, id: SettingId) -> String {
    format!("{}{}", (fraction * PERCENT_SCALE).round() as i32, unit_of(id))
}

fn gain_value(gain: f32, id: SettingId) -> String {
    format!("{}{}", volume_percent(gain), unit_of(id))
}

/// What the SKIN row shows for a chosen document: the file name alone, which is what tells two
/// documents apart on a row too narrow for a whole path.
pub fn skin_document_label(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |name| name.to_string_lossy().into_owned())
}

fn optional_text(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => text.to_string(),
        None => NONE_VALUE.to_string(),
    }
}

fn auto_or_number(value: Option<u32>) -> String {
    value.map_or_else(|| AUTO_VALUE.to_string(), |number| number.to_string())
}

fn gauge_at(gauge: GaugeKind) -> usize {
    GAUGE_CYCLE.iter().position(|entry| *entry == gauge).unwrap_or_default()
}

/// Which of the six JUDGE WIDTH percentages a row edits: the scratch array rather than the key one,
/// and the tier within it. Every other row answers `None`.
fn judge_width_slot(id: SettingId) -> Option<(bool, usize)> {
    match id {
        SettingId::JudgeWidthKeyPGreat => Some((false, PGREAT_TIER)),
        SettingId::JudgeWidthKeyGreat => Some((false, GREAT_TIER)),
        SettingId::JudgeWidthKeyGood => Some((false, GOOD_TIER)),
        SettingId::JudgeWidthScratchPGreat => Some((true, PGREAT_TIER)),
        SettingId::JudgeWidthScratchGreat => Some((true, GREAT_TIER)),
        SettingId::JudgeWidthScratchGood => Some((true, GOOD_TIER)),
        _ => None,
    }
}

fn judge_width(config: &Config, id: SettingId) -> Option<i32> {
    let (scratch, tier) = judge_width_slot(id)?;
    let rates = if scratch { config.judge.judge_rate_scratch } else { config.judge.judge_rate_key };
    rates.get(tier).copied()
}

fn judge_width_mut(config: &mut Config, id: SettingId) -> Option<&mut i32> {
    let (scratch, tier) = judge_width_slot(id)?;
    let rates = if scratch { &mut config.judge.judge_rate_scratch } else { &mut config.judge.judge_rate_key };
    rates.get_mut(tier)
}

fn cycle_at<T: PartialEq, const N: usize>(values: [T; N], current: T) -> usize {
    values.iter().position(|entry| *entry == current).unwrap_or_default()
}

/// What the settings screen shows for one row, as the document alone sees it.
pub fn display_value(config: &Config, id: SettingId) -> String {
    match id {
        SettingId::Autoplay => on_off(config.play.autoplay),
        SettingId::HiSpeed => format!("{:.*}", HISPEED_DECIMALS as usize, config.play.hispeed),
        SettingId::HiSpeedStep => format!("{:.*}", HISPEED_DECIMALS as usize, config.play.hispeed_step),
        SettingId::SpeedFix => cycle_label(id, usize::from(config.play.constant_speed)),
        SettingId::FixHiSpeed => cycle_label(id, cycle_at(options::FixHiSpeed::ALL, config.play.fix_hispeed)),
        SettingId::Random => config.play.random.label().to_string(),
        SettingId::LaneOption => cycle_label(id, cycle_at(options::LaneOption::ALL, config.play.lane_option)),
        SettingId::LegacyNote => on_off(config.play.legacy_note),
        SettingId::FiveKeyLayout => on_off(config.display.five_key_layout),
        SettingId::PlayEscapeMode => cycle_label(id, cycle_at(options::PlayEscape::ALL, config.play.play_escape)),
        SettingId::Gauge => cycle_label(id, gauge_at(config.play.gauge)),
        SettingId::Lift => shade_value(config.play.lift, id),
        SettingId::LaneCover => shade_value(config.play.cover, id),
        SettingId::ScratchSide => cycle_label(id, usize::from(config.play.scratch_left)),
        SettingId::ScratchAuto => on_off(config.play.scratch_auto),
        SettingId::JudgeOffset => format!("{:+}{}", config.judge.offset_ms, unit_of(id)),
        SettingId::Bga => on_off(config.display.bga),
        SettingId::KeyConfig
        | SettingId::Login
        | SettingId::Register
        | SettingId::Logout
        | SettingId::UploadSettings
        | SettingId::DownloadSettings
        | SettingId::SkinReload
        | SettingId::SkinReset => ACTION_VALUE.to_string(),
        SettingId::SkinScreen => skin_screen_label(config.skin.screen).unwrap_or(NONE_VALUE).to_string(),
        SettingId::SkinDocument => config.skin.document(config.skin.screen).map_or_else(|| DEFAULT_VALUE.to_string(), skin_document_label),
        SettingId::SkinInfo => NONE_VALUE.to_string(),
        SettingId::JudgeAlgorithm => cycle_label(id, cycle_at(JudgeAlgorithm::ALL, config.judge.judge_algorithm)),
        SettingId::JudgeWidthKeyPGreat
        | SettingId::JudgeWidthKeyGreat
        | SettingId::JudgeWidthKeyGood
        | SettingId::JudgeWidthScratchPGreat
        | SettingId::JudgeWidthScratchGreat
        | SettingId::JudgeWidthScratchGood => format!("{}{}", judge_width(config, id).unwrap_or_default(), unit_of(id)),
        SettingId::LongNoteMargin => format!("{}{}", config.judge.longnote_margin_rate, unit_of(id)),
        SettingId::LnMode => cycle_label(id, cycle_at(LnMode::ALL, config.judge.ln_mode)),
        SettingId::GaugeSet => gauge_set_token(config.judge.gauge_set).to_string(),
        SettingId::GaugeAutoShift => cycle_label(id, cycle_at(GaugeAutoShift::ALL, config.judge.gauge_auto_shift)),
        SettingId::BottomShiftableGauge => cycle_label(id, cycle_at(BOTTOM_SHIFTABLE_GAUGES, config.judge.bottom_shiftable_gauge)),
        SettingId::Target => config.judge.target.label().to_string(),
        SettingId::Total => {
            if config.play.total_override > TOTAL_FROM_CHART {
                format!("{}", config.play.total_override.round() as i32)
            } else {
                AUTO_VALUE.to_string()
            }
        }
        SettingId::Skin => config.display.skin.clone(),
        SettingId::AutoCal => on_off(config.judge.auto_offset),
        SettingId::AutoReplay => on_off(config.play.auto_replay),
        SettingId::DebugMode => on_off(config.display.debug),
        SettingId::Font => {
            if config.display.font_path.is_some() {
                CUSTOM_VALUE.to_string()
            } else {
                DEFAULT_VALUE.to_string()
            }
        }
        SettingId::ScoreGraph => on_off(config.display.score_graph),
        SettingId::ResultGraphs => on_off(config.display.result_graphs),
        SettingId::ReplayAnalysis => on_off(config.display.replay_analysis),
        SettingId::Preview => on_off(config.library.preview),
        SettingId::EnableLift => on_off(config.play.enable_lift),
        SettingId::EnableCover => on_off(config.play.enable_cover),
        SettingId::Hidden => shade_value(config.play.hidden, id),
        SettingId::EnableHidden => on_off(config.play.enable_hidden),
        SettingId::LaneCoverFineStep => format!("{:.*}", LANE_SHADE_FINE_DECIMALS as usize, config.play.lanecover_step_fine),
        SettingId::WhiteNumber => on_off(config.display.show_white_number),
        SettingId::JudgeTextY => shade_value(config.display.judge_text_y, id),
        SettingId::Letterbox => on_off(config.display.letterbox),
        SettingId::Sort => config.library.sort.label().to_string(),
        SettingId::FavoriteOnly => on_off(config.library.favorite_only),
        SettingId::PreviewVolume => shade_value(config.library.preview_volume, id),
        SettingId::PreviewFade => format!("{}{}", config.library.preview_fade_ms, unit_of(id)),
        SettingId::ServerUrl => optional_text(config.network.server_url.as_deref()),
        SettingId::PlayerId => config.network.player_id.clone(),
        SettingId::Account => config.network.ir_login_id.clone().unwrap_or_else(|| DEFAULT_PLAYER_ID.to_string()),
        SettingId::Email => optional_text(config.network.ir_email.as_deref()),
        SettingId::Password => UNSET_VALUE.to_string(),
        SettingId::SyncSettings => on_off(config.network.sync_settings),
        SettingId::AutoUploadReplay => on_off(config.network.auto_upload_replay),
        SettingId::Rivals => config.network.rivals.len().to_string(),
        SettingId::MasterVolume => gain_value(config.audio.master, id),
        SettingId::KeyVolume => gain_value(config.audio.key, id),
        SettingId::BgmVolume => gain_value(config.audio.bg, id),
        SettingId::SystemVolume => gain_value(config.audio.system, id),
        SettingId::AudioDevice => config.audio.device.clone().unwrap_or_else(|| DEFAULT_VALUE.to_string()),
        SettingId::AudioBuffer => auto_or_number(config.audio.buffer_frames),
        SettingId::AudioSampleRate => auto_or_number(config.audio.sample_rate),
        SettingId::AudioPolyphony => config.audio.polyphony.to_string(),
    }
}

fn store<T: PartialEq>(slot: &mut T, next: T) -> AdjustOutcome {
    if *slot == next {
        return AdjustOutcome::Unchanged;
    }
    *slot = next;
    AdjustOutcome::Changed
}

fn toggle(slot: &mut bool) -> AdjustOutcome {
    *slot = !*slot;
    AdjustOutcome::Changed
}

fn stepped(at: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    (at as i32 + delta).rem_euclid(len as i32) as usize
}

fn reopen(config: &mut Config, outcome: AdjustOutcome) -> AdjustOutcome {
    if outcome == AdjustOutcome::Changed {
        config.audio.mark_reopen_pending();
    }
    outcome
}

/// Apply one left/right step to a row. A row the running program owns leaves the document alone and
/// reports itself back as an [`AdjustOutcome::Action`].
pub fn adjust(config: &mut Config, id: SettingId, delta: i32) -> AdjustOutcome {
    match id {
        SettingId::Autoplay => toggle(&mut config.play.autoplay),
        SettingId::HiSpeed => match crate::schema::stepped_hispeed(config.play.hispeed, f64::from(delta) * config.play.hispeed_step) {
            Some(next) => store(&mut config.play.hispeed, next),
            None => AdjustOutcome::Unchanged,
        },
        SettingId::HiSpeedStep => {
            let next = (config.play.hispeed_step + f64::from(delta) * HISPEED_STEP_STEP).clamp(HISPEED_STEP_MIN, HISPEED_STEP_MAX);
            store(&mut config.play.hispeed_step, next)
        }
        SettingId::SpeedFix => toggle(&mut config.play.constant_speed),
        SettingId::FixHiSpeed => {
            let at = stepped(cycle_at(options::FixHiSpeed::ALL, config.play.fix_hispeed), options::FIX_HISPEED_COUNT, delta);
            store(&mut config.play.fix_hispeed, options::FixHiSpeed::ALL[at])
        }
        SettingId::LaneOption => {
            let at = stepped(cycle_at(options::LaneOption::ALL, config.play.lane_option), options::LANE_OPTION_COUNT, delta);
            store(&mut config.play.lane_option, options::LaneOption::ALL[at])
        }
        SettingId::LegacyNote => toggle(&mut config.play.legacy_note),
        SettingId::FiveKeyLayout => toggle(&mut config.display.five_key_layout),
        SettingId::PlayEscapeMode => {
            let at = stepped(cycle_at(options::PlayEscape::ALL, config.play.play_escape), options::PLAY_ESCAPE_COUNT, delta);
            store(&mut config.play.play_escape, options::PlayEscape::ALL[at])
        }
        SettingId::Random => {
            let at = NoteOption::ALL.iter().position(|option| *option == config.play.random).unwrap_or_default();
            let next = NoteOption::ALL[stepped(at, NoteOption::ALL.len(), delta)];
            store(&mut config.play.random, next)
        }
        SettingId::Gauge => {
            let next = GAUGE_CYCLE[stepped(gauge_at(config.play.gauge), GAUGE_CYCLE.len(), delta)];
            store(&mut config.play.gauge, next)
        }
        SettingId::Lift => {
            let next = (config.play.lift + delta as f32 * LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
            store(&mut config.play.lift, next)
        }
        SettingId::LaneCover => {
            let next = (config.play.cover + delta as f32 * LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
            store(&mut config.play.cover, next)
        }
        SettingId::ScratchSide => toggle(&mut config.play.scratch_left),
        SettingId::ScratchAuto => toggle(&mut config.play.scratch_auto),
        SettingId::JudgeOffset => {
            let next = (config.judge.offset_ms + delta * JUDGE_OFFSET_STEP_MS).clamp(JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_MAX_MS);
            store(&mut config.judge.offset_ms, next)
        }
        SettingId::Bga => toggle(&mut config.display.bga),
        SettingId::JudgeWidthKeyPGreat
        | SettingId::JudgeWidthKeyGreat
        | SettingId::JudgeWidthKeyGood
        | SettingId::JudgeWidthScratchPGreat
        | SettingId::JudgeWidthScratchGreat
        | SettingId::JudgeWidthScratchGood => match judge_width_mut(config, id) {
            Some(rate) => {
                let next = (*rate + delta * JUDGE_RATE_STEP_PERCENT).clamp(JUDGE_RATE_MIN_PERCENT, JUDGE_RATE_MAX_PERCENT);
                store(rate, next)
            }
            None => AdjustOutcome::Unchanged,
        },
        SettingId::JudgeAlgorithm => {
            let at = stepped(cycle_at(JudgeAlgorithm::ALL, config.judge.judge_algorithm), JudgeAlgorithm::ALL.len(), delta);
            store(&mut config.judge.judge_algorithm, JudgeAlgorithm::ALL[at])
        }
        SettingId::LongNoteMargin => {
            let next = (config.judge.longnote_margin_rate + delta * LN_MARGIN_STEP_PERCENT).clamp(LN_MARGIN_MIN_PERCENT, LN_MARGIN_MAX_PERCENT);
            store(&mut config.judge.longnote_margin_rate, next)
        }
        SettingId::LnMode => {
            let at = stepped(cycle_at(LnMode::ALL, config.judge.ln_mode), LnMode::ALL.len(), delta);
            store(&mut config.judge.ln_mode, LnMode::ALL[at])
        }
        SettingId::GaugeSet => {
            let at = stepped(cycle_at(GAUGE_SET_CYCLE, config.judge.gauge_set), GAUGE_SET_CYCLE.len(), delta);
            store(&mut config.judge.gauge_set, GAUGE_SET_CYCLE[at])
        }
        SettingId::GaugeAutoShift => {
            let at = stepped(cycle_at(GaugeAutoShift::ALL, config.judge.gauge_auto_shift), GaugeAutoShift::ALL.len(), delta);
            store(&mut config.judge.gauge_auto_shift, GaugeAutoShift::ALL[at])
        }
        SettingId::BottomShiftableGauge => {
            let at = stepped(cycle_at(BOTTOM_SHIFTABLE_GAUGES, config.judge.bottom_shiftable_gauge), BOTTOM_SHIFTABLE_GAUGES.len(), delta);
            store(&mut config.judge.bottom_shiftable_gauge, BOTTOM_SHIFTABLE_GAUGES[at])
        }
        SettingId::Target => {
            let at = stepped(cycle_at(ScoreTarget::ALL, config.judge.target), ScoreTarget::ALL.len(), delta);
            store(&mut config.judge.target, ScoreTarget::ALL[at])
        }
        SettingId::Total => {
            let next = (config.play.total_override + f64::from(delta) * TOTAL_STEP).clamp(TOTAL_FROM_CHART, TOTAL_MAX);
            store(&mut config.play.total_override, next)
        }
        SettingId::AutoCal => toggle(&mut config.judge.auto_offset),
        SettingId::AutoReplay => toggle(&mut config.play.auto_replay),
        SettingId::DebugMode => toggle(&mut config.display.debug),
        SettingId::ScoreGraph => toggle(&mut config.display.score_graph),
        SettingId::ResultGraphs => toggle(&mut config.display.result_graphs),
        SettingId::ReplayAnalysis => toggle(&mut config.display.replay_analysis),
        SettingId::Preview => toggle(&mut config.library.preview),
        SettingId::EnableLift => toggle(&mut config.play.enable_lift),
        SettingId::EnableCover => toggle(&mut config.play.enable_cover),
        SettingId::EnableHidden => toggle(&mut config.play.enable_hidden),
        SettingId::WhiteNumber => toggle(&mut config.display.show_white_number),
        SettingId::Letterbox => toggle(&mut config.display.letterbox),
        SettingId::FavoriteOnly => toggle(&mut config.library.favorite_only),
        SettingId::Hidden => {
            let next = (config.play.hidden + delta as f32 * LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
            store(&mut config.play.hidden, next)
        }
        SettingId::LaneCoverFineStep => {
            let next = (config.play.lanecover_step_fine + delta as f32 * LANE_SHADE_FINE_STEP_STEP).clamp(LANE_SHADE_FINE_STEP_MIN, LANE_SHADE_FINE_STEP_MAX);
            store(&mut config.play.lanecover_step_fine, next)
        }
        SettingId::JudgeTextY => {
            let next = (config.display.judge_text_y + delta as f32 * JUDGE_TEXT_Y_STEP).clamp(JUDGE_TEXT_Y_MIN, JUDGE_TEXT_Y_MAX);
            store(&mut config.display.judge_text_y, next)
        }
        SettingId::Sort => {
            let at = stepped(cycle_at(SortMode::SELECTABLE, config.library.sort), SortMode::SELECTABLE.len(), delta);
            store(&mut config.library.sort, SortMode::SELECTABLE[at])
        }
        SettingId::PreviewVolume => {
            let next = (config.library.preview_volume + delta as f32 * PREVIEW_VOLUME_STEP).clamp(PREVIEW_VOLUME_MIN, PREVIEW_VOLUME_MAX);
            store(&mut config.library.preview_volume, next)
        }
        SettingId::PreviewFade => {
            let moved = config.library.preview_fade_ms as i32 + delta * PREVIEW_FADE_STEP_MS as i32;
            let next = moved.clamp(PREVIEW_FADE_MIN_MS as i32, PREVIEW_FADE_MAX_MS as i32) as u32;
            store(&mut config.library.preview_fade_ms, next)
        }
        SettingId::SyncSettings => toggle(&mut config.network.sync_settings),
        SettingId::AutoUploadReplay => toggle(&mut config.network.auto_upload_replay),
        SettingId::MasterVolume => {
            let next = step_volume(config.audio.master, delta);
            store(&mut config.audio.master, next)
        }
        SettingId::KeyVolume => {
            let next = step_volume(config.audio.key, delta);
            store(&mut config.audio.key, next)
        }
        SettingId::BgmVolume => {
            let next = step_volume(config.audio.bg, delta);
            store(&mut config.audio.bg, next)
        }
        SettingId::SystemVolume => {
            let next = step_volume(config.audio.system, delta);
            store(&mut config.audio.system, next)
        }
        SettingId::AudioBuffer => {
            let next = cycle_optional_u32(config.audio.buffer_frames, &AUDIO_BUFFER_FRAMES_CHOICES, delta);
            let outcome = store(&mut config.audio.buffer_frames, next);
            reopen(config, outcome)
        }
        SettingId::AudioSampleRate => {
            let next = cycle_optional_u32(config.audio.sample_rate, &AUDIO_SAMPLE_RATE_HZ_CHOICES, delta);
            let outcome = store(&mut config.audio.sample_rate, next);
            reopen(config, outcome)
        }
        SettingId::AudioPolyphony => {
            let next = step_polyphony(config.audio.polyphony, delta);
            let outcome = store(&mut config.audio.polyphony, next);
            reopen(config, outcome)
        }
        SettingId::SkinScreen => {
            let at = stepped(config.skin.screen.max(0) as usize, SKIN_SCREEN_LABELS.len(), delta);
            let next = i32::try_from(at).unwrap_or(crate::schema::DEFAULT_SKIN_SCREEN);
            store(&mut config.skin.screen, next)
        }
        SettingId::SkinInfo => AdjustOutcome::Unchanged,
        SettingId::KeyConfig
        | SettingId::Skin
        | SettingId::SkinDocument
        | SettingId::SkinReload
        | SettingId::SkinReset
        | SettingId::Font
        | SettingId::ServerUrl
        | SettingId::PlayerId
        | SettingId::Account
        | SettingId::Email
        | SettingId::Password
        | SettingId::Login
        | SettingId::Register
        | SettingId::Logout
        | SettingId::UploadSettings
        | SettingId::DownloadSettings
        | SettingId::Rivals
        | SettingId::AudioDevice => AdjustOutcome::Action(id),
    }
}

/// Step the SKIN row between the bundled skins. The skin the run was started with is a launch
/// override the document does not hold, so the program clears that before calling this.
pub fn step_skin(config: &mut Config) {
    config.display.skin = if config.display.skin.eq_ignore_ascii_case(SKIN_ALTERNATE) { SKIN_PRIMARY.to_string() } else { SKIN_ALTERNATE.to_string() };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab_of(id: SettingId) -> SettingTab {
        descriptor(id).tab
    }

    #[test]
    fn every_row_appears_in_the_table_exactly_once() {
        for id in SettingId::ALL {
            let found = SETTINGS.iter().filter(|row| row.id == id).count();
            assert_eq!(found, 1, "{id:?} appears {found} times in the table");
        }
        assert_eq!(SETTINGS.len(), SettingId::ALL.len(), "the table holds a row that is not a declared id");
        assert_eq!(SETTINGS.len(), SETTING_COUNT);
    }

    #[test]
    fn every_tab_has_rows_and_together_they_hold_every_row() {
        let config = Config::default();
        let mut seen: Vec<SettingId> = Vec::new();
        for tab in SettingTab::ALL {
            let rows = tab_rows(tab, &config);
            assert!(!rows.is_empty(), "{tab:?} has no rows");
            seen.extend(rows);
        }
        let hidden = SETTINGS.iter().filter(|row| !(row.visible)(&config)).count();
        assert_eq!(seen.len() + hidden, SETTING_COUNT, "a row is listed in two tabs or in none");
        for id in SettingId::ALL {
            let shown = seen.contains(&id);
            assert_eq!(shown, (descriptor(id).visible)(&config), "{id:?} is in no tab");
        }
    }

    /// A row nothing acts on yet is not offered: a player who moved it would be choosing an option
    /// that changes nothing about the run. It keeps its descriptor so the value still reads back.
    #[test]
    fn a_row_nothing_acts_on_yet_is_not_offered() {
        let config = Config::default();
        assert!(!(descriptor(SettingId::LaneOption).visible)(&config), "LANE OPTION is offered but nothing applies it to a run");
        assert!(!tab_rows(SettingTab::Play, &config).contains(&SettingId::LaneOption));
        assert_eq!(display_value(&config, SettingId::LaneOption), OFF_VALUE, "the value still reads back for the file it is stored in");
    }

    #[test]
    fn every_tab_has_its_own_name() {
        let mut names: Vec<&str> = SettingTab::ALL.iter().map(|tab| tab.label()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
        assert_eq!(SettingTab::ALL.map(SettingTab::label).to_vec(), vec!["PLAY", "GAUGE", "JUDGE", "DISPLAY", "SKIN", "INPUT", "NETWORK", "AUDIO", "SELECT"]);
    }

    #[test]
    fn every_row_has_its_own_label_within_its_tab() {
        let config = Config::default();
        for tab in SettingTab::ALL {
            let mut labels: Vec<&str> = tab_rows(tab, &config).iter().map(|&id| descriptor(id).label).collect();
            let count = labels.len();
            labels.sort_unstable();
            labels.dedup();
            assert_eq!(labels.len(), count, "{tab:?} has two rows with the same label");
        }
        for id in SettingId::ALL {
            assert!(!descriptor(id).label.is_empty(), "{id:?} has no label");
            assert!(!descriptor(id).help.is_empty(), "{id:?} has no help");
        }
    }

    #[test]
    fn a_row_index_is_its_position_in_the_list() {
        for (at, id) in SettingId::ALL.into_iter().enumerate() {
            assert_eq!(id.row_index(), at);
            assert_eq!(SettingId::from_row_index(at), Some(id));
        }
        assert_eq!(SettingId::from_row_index(SETTING_COUNT), None);
    }

    const STRESS_STEPS: i32 = 1_000;

    fn ranged_value(config: &Config, id: SettingId) -> f64 {
        match id {
            SettingId::HiSpeed => config.play.hispeed,
            SettingId::HiSpeedStep => config.play.hispeed_step,
            SettingId::Total => config.play.total_override,
            SettingId::JudgeOffset => f64::from(config.judge.offset_ms),
            SettingId::LongNoteMargin => f64::from(config.judge.longnote_margin_rate),
            _ if judge_width_slot(id).is_some() => f64::from(judge_width(config, id).unwrap_or_default()),
            SettingId::AudioPolyphony => config.audio.polyphony as f64,
            SettingId::Lift => f64::from(config.play.lift),
            SettingId::LaneCover => f64::from(config.play.cover),
            SettingId::Hidden => f64::from(config.play.hidden),
            SettingId::LaneCoverFineStep => f64::from(config.play.lanecover_step_fine),
            SettingId::JudgeTextY => f64::from(config.display.judge_text_y),
            SettingId::PreviewVolume => f64::from(config.library.preview_volume),
            SettingId::PreviewFade => f64::from(config.library.preview_fade_ms),
            SettingId::MasterVolume => f64::from(config.audio.master),
            SettingId::KeyVolume => f64::from(config.audio.key),
            SettingId::BgmVolume => f64::from(config.audio.bg),
            SettingId::SystemVolume => f64::from(config.audio.system),
            _ => f64::NAN,
        }
    }

    #[test]
    fn a_ranged_row_held_down_never_leaves_its_declared_range() {
        for id in SettingId::ALL {
            let (min, max) = match descriptor(id).kind {
                SettingKind::IntRange { min, max, .. } => (f64::from(min), f64::from(max)),
                SettingKind::FloatRange { min, max, .. } => (min, max),
                SettingKind::Percent { min, max, .. } => (f64::from(min), f64::from(max)),
                _ => continue,
            };
            for direction in [1, -1] {
                let mut config = Config::default();
                for _ in 0..STRESS_STEPS {
                    adjust(&mut config, id, direction);
                    let value = ranged_value(&config, id);
                    assert!(value >= min && value <= max, "{id:?} left [{min}, {max}] at {value}");
                }
            }
        }
    }

    #[test]
    fn a_ranged_row_at_the_end_of_its_range_reports_no_change() {
        for id in SettingId::ALL {
            if !matches!(descriptor(id).kind, SettingKind::IntRange { .. } | SettingKind::FloatRange { .. } | SettingKind::Percent { .. }) {
                continue;
            }
            let mut config = Config::default();
            for _ in 0..STRESS_STEPS {
                adjust(&mut config, id, -1);
            }
            assert_eq!(adjust(&mut config, id, -1), AdjustOutcome::Unchanged, "{id:?} keeps reporting a change at its floor");
        }
    }

    #[test]
    fn every_row_shows_something() {
        let config = Config::default();
        for id in SettingId::ALL {
            assert!(!display_value(&config, id).is_empty(), "{id:?} shows nothing");
        }
    }

    #[test]
    fn the_rows_the_program_owns_are_the_ones_it_reads_outside_the_document() {
        let owned: Vec<SettingId> = SettingId::ALL.into_iter().filter(|&id| descriptor(id).host_value).collect();
        assert_eq!(owned, vec![SettingId::Skin, SettingId::SkinInfo, SettingId::Account, SettingId::Password]);
    }

    #[test]
    fn a_row_that_opens_a_screen_or_an_editor_leaves_the_document_alone() {
        for id in SettingId::ALL {
            let mut config = Config::default();
            let before = config.clone();
            let outcome = adjust(&mut config, id, 1);
            if outcome == AdjustOutcome::Action(id) {
                assert_eq!(config, before, "{id:?} reported an action but still wrote to the document");
            }
        }
    }

    #[test]
    fn the_rows_the_program_steps_itself_are_the_screens_pickers_editors_and_the_device_list() {
        let actions: Vec<SettingId> = SettingId::ALL.into_iter().filter(|&id| adjust(&mut Config::default(), id, 1) == AdjustOutcome::Action(id)).collect();
        assert_eq!(
            actions,
            vec![
                SettingId::KeyConfig,
                SettingId::Skin,
                SettingId::SkinDocument,
                SettingId::SkinReload,
                SettingId::SkinReset,
                SettingId::Font,
                SettingId::ServerUrl,
                SettingId::PlayerId,
                SettingId::Account,
                SettingId::Email,
                SettingId::Password,
                SettingId::Login,
                SettingId::Register,
                SettingId::Logout,
                SettingId::UploadSettings,
                SettingId::DownloadSettings,
                SettingId::Rivals,
                SettingId::AudioDevice,
            ]
        );
    }

    #[test]
    fn a_toggle_row_comes_back_to_where_it_started() {
        let mut config = Config::default();
        for id in SettingId::ALL {
            if descriptor(id).kind != SettingKind::Toggle {
                continue;
            }
            let before = display_value(&config, id);
            assert_eq!(adjust(&mut config, id, 1), AdjustOutcome::Changed, "{id:?}");
            assert_ne!(display_value(&config, id), before, "{id:?} did not move");
            adjust(&mut config, id, -1);
            assert_eq!(display_value(&config, id), before, "{id:?} did not come back");
        }
    }

    #[test]
    fn a_cycling_row_walks_its_whole_list_and_returns() {
        let mut config = Config::default();
        for id in [
            SettingId::SpeedFix,
            SettingId::FixHiSpeed,
            SettingId::LaneOption,
            SettingId::PlayEscapeMode,
            SettingId::Random,
            SettingId::Gauge,
            SettingId::ScratchSide,
            SettingId::Sort,
            SettingId::SkinScreen,
        ] {
            let values = cycle_values(id);
            let start = display_value(&config, id);
            let mut seen: Vec<String> = Vec::new();
            for _ in 0..values.len() {
                seen.push(display_value(&config, id));
                adjust(&mut config, id, 1);
            }
            assert_eq!(display_value(&config, id), start, "{id:?} did not come back round");
            seen.sort_unstable();
            let mut declared: Vec<String> = values.iter().map(|value| (*value).to_string()).collect();
            declared.sort_unstable();
            assert_eq!(seen, declared, "{id:?} does not show the values it declares");
        }
    }

    #[test]
    fn the_random_row_declares_the_shuffles_the_engine_has() {
        let declared: Vec<&str> = NoteOption::ALL.iter().map(|option| option.label()).collect();
        assert_eq!(cycle_values(SettingId::Random), declared.as_slice());
    }

    #[test]
    fn the_gauge_row_declares_one_name_per_gauge_it_steps_through() {
        assert_eq!(GAUGE_LABELS.len(), GAUGE_CYCLE.len());
        for gauge in [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard] {
            assert!(GAUGE_CYCLE.contains(&gauge), "{gauge:?} is not in the cycle, so the row could not show it");
        }
    }

    #[test]
    fn the_output_parameter_rows_declare_the_values_the_engine_is_offered() {
        let buffers: Vec<String> = AUDIO_BUFFER_FRAMES_CHOICES.iter().map(u32::to_string).collect();
        assert_eq!(cycle_values(SettingId::AudioBuffer)[0], AUTO_VALUE);
        assert_eq!(cycle_values(SettingId::AudioBuffer)[1..], buffers);
        let rates: Vec<String> = AUDIO_SAMPLE_RATE_HZ_CHOICES.iter().map(u32::to_string).collect();
        assert_eq!(cycle_values(SettingId::AudioSampleRate)[0], AUTO_VALUE);
        assert_eq!(cycle_values(SettingId::AudioSampleRate)[1..], rates);
    }

    #[test]
    fn an_output_parameter_that_moved_asks_for_a_stream_reopen() {
        for id in [SettingId::AudioBuffer, SettingId::AudioSampleRate, SettingId::AudioPolyphony] {
            let mut config = Config::default();
            assert_eq!(adjust(&mut config, id, 1), AdjustOutcome::Changed, "{id:?}");
            assert!(config.audio.reopen_pending(), "{id:?} moved without asking for a reopen");
        }
    }

    #[test]
    fn a_gain_row_never_asks_for_a_stream_reopen() {
        for id in [SettingId::MasterVolume, SettingId::KeyVolume, SettingId::BgmVolume, SettingId::SystemVolume] {
            let mut config = Config::default();
            adjust(&mut config, id, -1);
            assert!(!config.audio.reopen_pending(), "{id:?} restarted the output stream for a gain");
        }
    }

    #[test]
    fn a_parameter_row_that_could_not_move_leaves_no_pending_reopen() {
        let mut config = Config::default();
        for _ in 0..STRESS_STEPS {
            adjust(&mut config, SettingId::AudioPolyphony, -1);
        }
        config.audio.clear_reopen_pending();
        assert_eq!(adjust(&mut config, SettingId::AudioPolyphony, -1), AdjustOutcome::Unchanged);
        assert!(!config.audio.reopen_pending());
    }

    #[test]
    fn the_skin_row_steps_between_the_bundled_skins() {
        let mut config = Config::default();
        assert_eq!(display_value(&config, SettingId::Skin), SKIN_PRIMARY);
        step_skin(&mut config);
        assert_eq!(display_value(&config, SettingId::Skin), SKIN_ALTERNATE);
        step_skin(&mut config);
        assert_eq!(display_value(&config, SettingId::Skin), SKIN_PRIMARY);
        assert_eq!(cycle_values(SettingId::Skin), &[SKIN_PRIMARY, SKIN_ALTERNATE]);
    }

    #[test]
    fn the_default_document_reads_as_the_shipped_settings() {
        let config = Config::default();
        assert_eq!(display_value(&config, SettingId::Autoplay), ON_VALUE);
        assert_eq!(display_value(&config, SettingId::HiSpeed), "2.00");
        assert_eq!(display_value(&config, SettingId::HiSpeedStep), "0.25");
        assert_eq!(display_value(&config, SettingId::SpeedFix), "FLOATING");
        assert_eq!(display_value(&config, SettingId::FixHiSpeed), "MAIN", "PlayConfig.java:43-49 ships MAINBPM");
        assert_eq!(display_value(&config, SettingId::LaneOption), "OFF");
        assert_eq!(display_value(&config, SettingId::PlayEscapeMode), "IMMEDIATE", "escaping a run stays the one press it has always been");
        assert_eq!(display_value(&config, SettingId::Hidden), "0%");
        assert_eq!(display_value(&config, SettingId::EnableCover), ON_VALUE, "PlayConfig.java:66 ships the cover applied");
        assert_eq!(display_value(&config, SettingId::EnableLift), OFF_VALUE, "PlayConfig.java:74");
        assert_eq!(display_value(&config, SettingId::EnableHidden), OFF_VALUE, "PlayConfig.java:82");
        assert_eq!(display_value(&config, SettingId::Sort), "DEFAULT");
        assert_eq!(display_value(&config, SettingId::PreviewVolume), "85%");
        assert_eq!(display_value(&config, SettingId::PreviewFade), "200 MS");
        assert_eq!(display_value(&config, SettingId::Random), "OFF");
        assert_eq!(display_value(&config, SettingId::Gauge), "NORMAL");
        assert_eq!(display_value(&config, SettingId::Lift), "0%");
        assert_eq!(display_value(&config, SettingId::ScratchSide), "RIGHT");
        assert_eq!(display_value(&config, SettingId::JudgeOffset), "+0 MS");
        assert_eq!(display_value(&config, SettingId::JudgeWidthKeyPGreat), "100%");
        assert_eq!(display_value(&config, SettingId::JudgeWidthScratchGood), "100%");
        assert_eq!(display_value(&config, SettingId::LongNoteMargin), "100%");
        assert_eq!(display_value(&config, SettingId::JudgeAlgorithm), "COMBO", "JudgeAlgorithm.java:42 lists Combo first, so it is the shipped default");
        assert_eq!(display_value(&config, SettingId::LnMode), "LN");
        assert_eq!(display_value(&config, SettingId::GaugeSet), AUTO_VALUE);
        assert_eq!(display_value(&config, SettingId::GaugeAutoShift), "NONE");
        assert_eq!(display_value(&config, SettingId::BottomShiftableGauge), "ASSIST EASY");
        assert_eq!(display_value(&config, SettingId::Target), "RATE AAA", "the shipped target is the rate the spec names");
        assert_eq!(display_value(&config, SettingId::Total), AUTO_VALUE);
        assert_eq!(display_value(&config, SettingId::Font), DEFAULT_VALUE);
        assert_eq!(display_value(&config, SettingId::KeyConfig), ACTION_VALUE);
        assert_eq!(display_value(&config, SettingId::ServerUrl), NONE_VALUE);
        assert_eq!(display_value(&config, SettingId::PlayerId), DEFAULT_PLAYER_ID);
        assert_eq!(display_value(&config, SettingId::Password), UNSET_VALUE);
        assert_eq!(display_value(&config, SettingId::Rivals), "0");
        assert_eq!(display_value(&config, SettingId::MasterVolume), "100%");
        assert_eq!(display_value(&config, SettingId::KeyVolume), "50%");
        assert_eq!(display_value(&config, SettingId::AudioDevice), DEFAULT_VALUE);
        assert_eq!(display_value(&config, SettingId::AudioBuffer), AUTO_VALUE);
    }

    #[test]
    fn a_judge_offset_reads_with_its_sign_and_unit() {
        let mut config = Config::default();
        config.judge.offset_ms = -JUDGE_OFFSET_STEP_MS;
        assert_eq!(display_value(&config, SettingId::JudgeOffset), format!("-{JUDGE_OFFSET_STEP_MS} MS"));
        config.judge.offset_ms = JUDGE_OFFSET_STEP_MS;
        assert_eq!(display_value(&config, SettingId::JudgeOffset), format!("+{JUDGE_OFFSET_STEP_MS} MS"));
    }

    #[test]
    fn the_table_is_grouped_by_tab_so_a_tab_reads_in_one_run() {
        let mut runs: Vec<SettingTab> = Vec::new();
        for row in SETTINGS {
            if runs.last() != Some(&row.tab) {
                assert!(!runs.contains(&row.tab), "{:?} is split into two runs of the table", row.tab);
                runs.push(row.tab);
            }
        }
        assert_eq!(runs, SettingTab::ALL.to_vec());
    }

    #[test]
    fn every_tab_keeps_the_order_the_screen_lists_it_in() {
        let config = Config::default();
        assert_eq!(
            tab_rows(SettingTab::Play, &config),
            vec![
                SettingId::Autoplay,
                SettingId::HiSpeed,
                SettingId::HiSpeedStep,
                SettingId::SpeedFix,
                SettingId::FixHiSpeed,
                SettingId::Random,
                SettingId::LegacyNote,
                SettingId::FiveKeyLayout,
                SettingId::AutoReplay,
                SettingId::PlayEscapeMode,
            ]
        );
        assert_eq!(tab_rows(SettingTab::Gauge, &config), vec![SettingId::Gauge, SettingId::Total]);
        assert_eq!(
            tab_rows(SettingTab::Judge, &config),
            vec![
                SettingId::JudgeOffset,
                SettingId::AutoCal,
                SettingId::JudgeAlgorithm,
                SettingId::JudgeWidthKeyPGreat,
                SettingId::JudgeWidthKeyGreat,
                SettingId::JudgeWidthKeyGood,
                SettingId::JudgeWidthScratchPGreat,
                SettingId::JudgeWidthScratchGreat,
                SettingId::JudgeWidthScratchGood,
                SettingId::LongNoteMargin,
                SettingId::LnMode,
                SettingId::GaugeSet,
                SettingId::GaugeAutoShift,
                SettingId::BottomShiftableGauge,
                SettingId::Target,
            ]
        );
        assert_eq!(
            tab_rows(SettingTab::Display, &config),
            vec![
                SettingId::Skin,
                SettingId::Font,
                SettingId::ScoreGraph,
                SettingId::ResultGraphs,
                SettingId::ReplayAnalysis,
                SettingId::Preview,
                SettingId::Lift,
                SettingId::EnableLift,
                SettingId::LaneCover,
                SettingId::EnableCover,
                SettingId::Hidden,
                SettingId::EnableHidden,
                SettingId::LaneCoverFineStep,
                SettingId::WhiteNumber,
                SettingId::JudgeTextY,
                SettingId::Letterbox,
                SettingId::Bga,
                SettingId::DebugMode,
            ]
        );
        assert_eq!(tab_rows(SettingTab::Select, &config), vec![SettingId::Sort, SettingId::FavoriteOnly, SettingId::PreviewVolume, SettingId::PreviewFade]);
        assert_eq!(tab_rows(SettingTab::Input, &config), vec![SettingId::ScratchSide, SettingId::ScratchAuto, SettingId::KeyConfig]);
        assert_eq!(
            tab_rows(SettingTab::Network, &config),
            vec![
                SettingId::ServerUrl,
                SettingId::PlayerId,
                SettingId::Account,
                SettingId::Email,
                SettingId::Password,
                SettingId::Login,
                SettingId::Register,
                SettingId::Logout,
                SettingId::SyncSettings,
                SettingId::UploadSettings,
                SettingId::DownloadSettings,
                SettingId::AutoUploadReplay,
                SettingId::Rivals,
            ]
        );
        assert_eq!(
            tab_rows(SettingTab::Audio, &config),
            vec![
                SettingId::MasterVolume,
                SettingId::KeyVolume,
                SettingId::BgmVolume,
                SettingId::SystemVolume,
                SettingId::AudioDevice,
                SettingId::AudioBuffer,
                SettingId::AudioSampleRate,
                SettingId::AudioPolyphony,
            ]
        );
        for tab in SettingTab::ALL {
            for id in tab_rows(tab, &config) {
                assert_eq!(tab_of(id), tab);
            }
        }
    }
}
