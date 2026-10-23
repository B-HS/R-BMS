//! The settings screen's rows as data: one descriptor per row naming its tab, label, kind and
//! range, plus the two functions the screen needs — what a row shows and what one left/right step
//! does to it.
//!
//! Rows are named rather than numbered. Adding one is a single entry in [`SETTINGS`]; nothing else
//! has to be renumbered, and the screen never has to know which position a row happens to occupy.
//! Rows the running program owns rather than the document — a live account, a password held only in
//! memory, a skin forced on the command line, the controller's debounce the key config file holds —
//! are marked [`SettingDescriptor::host_value`], and
//! the value returned here is what the document alone can say about them.

use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::{BOTTOM_SHIFTABLE_GAUGES, GaugeAutoShift};
use rbms_judge::gauge_tables::CourseGaugeMode;
use rbms_judge::ln::LnMode;
use rbms_skin::loader::{
    SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_RESULT,
};

use crate::audio::{
    AUDIO_BUFFER_FRAMES_CHOICES, AUDIO_POLYPHONY_MAX_VOICES, AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_STEP_VOICES, AUDIO_SAMPLE_RATE_HZ_CHOICES,
    AUDIO_VOLUME_MAX_GAIN, AUDIO_VOLUME_MAX_PERCENT, AUDIO_VOLUME_MIN_GAIN, AUDIO_VOLUME_STEP_PERCENT, cycle_optional_u32, step_polyphony, step_volume,
    volume_percent,
};
use crate::judge::{
    COURSE_GAUGE_LABELS, GAUGE_AUTO_SHIFT_LABELS, GAUGE_SET_CYCLE, GAUGE_SET_LABELS, JUDGE_ALGORITHM_LABELS, LN_MODE_LABELS, ScoreTarget,
    TARGET_FALLBACK_LABELS, TARGET_LABELS, gauge_set_token,
};
use crate::language::{LANGUAGE_LABELS, LanguageChoice};
use crate::number::NumberSpec;
use crate::options;
use crate::schema::{
    Config, DEFAULT_PAD_DEBOUNCE_MS, DEFAULT_PLAYER_ID, HISPEED_MAX, HISPEED_MIN, HISPEED_STEP, HISPEED_STEP_MAX, HISPEED_STEP_MIN, HISPEED_STEP_STEP,
    JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_STEP_MS, JUDGE_RATE_MAX_PERCENT, JUDGE_RATE_MIN_PERCENT, JUDGE_RATE_STEP_PERCENT, JUDGE_TEXT_Y_MAX,
    JUDGE_TEXT_Y_MIN, JUDGE_TEXT_Y_STEP, KEY_DEBOUNCE_MAX_MS, KEY_DEBOUNCE_MIN_MS, KEY_DEBOUNCE_STEP_MS, LANE_SHADE_FINE_STEP_MAX, LANE_SHADE_FINE_STEP_MIN,
    LANE_SHADE_FINE_STEP_STEP, LANE_SHADE_MAX, LANE_SHADE_MIN, LANE_SHADE_STEP, LN_MARGIN_MAX_PERCENT, LN_MARGIN_MIN_PERCENT, LN_MARGIN_STEP_PERCENT,
    PAD_DEBOUNCE_MAX_MS, PAD_DEBOUNCE_MIN_MS, PAD_DEBOUNCE_STEP_MS, PREVIEW_FADE_MAX_MS, PREVIEW_FADE_MIN_MS, PREVIEW_FADE_STEP_MS, PREVIEW_VOLUME_MAX,
    PREVIEW_VOLUME_MIN, PREVIEW_VOLUME_STEP, SKIN_SCREEN_LABELS, TARGET_RATE_MAX_PERCENT, TARGET_RATE_MIN_PERCENT, TARGET_RATE_STEP_PERCENT, TOTAL_FROM_CHART,
    TOTAL_STEP, skin_screen_label,
};
use crate::sort::SortMode;
use crate::speed::{GREEN_NUMBER_MAX, GREEN_NUMBER_MIN, duration_of_green, duration_stepped_by_green, green_of_duration};
use crate::window::{WINDOW_MODE_LABELS, WINDOW_RESOLUTION_LABELS, WindowMode, WindowResolution};

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

/// One left/right step on the GREEN NUMBER row.
pub const GREEN_NUMBER_ROW_STEP: i32 = 1;

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

/// Said beside a row that only the built-in screens read, while a skin draws one of the screens it
/// is for.
pub const BUILT_IN_ONLY_NOTE: &str = "BUILT-IN SCREENS ONLY";

/// Said beside a row a skin settles with an option of its own, while a skin draws one of the
/// screens the row is for: the row still holds for the built-in screens, and on the skin's the
/// option the skin menu lists for that skin is what counts.
pub const SKIN_OPTION_NOTE: &str = "SKIN OPTION COMES FIRST";

/// Every play screen the built-in note field draws, as the `SkinType` ids their documents declare.
const PLAY_SCREENS: &[i32] =
    &[SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_24KEYS];

/// The play screens of the charts the 5KEYS LAYOUT row narrows.
const FIVE_KEY_SCREENS: &[i32] = &[SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_10KEYS];

/// The play screens of the charts that have a scratch lane to put on one side.
const SCRATCH_SCREENS: &[i32] = &[SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_10KEYS];

/// The screen a finished run is shown on.
const RESULT_SCREENS: &[i32] = &[SKIN_TYPE_RESULT];

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
    KeyDebounce,
    PadDebounce,
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
    CourseGauge,
    GaugeAutoShift,
    BottomShiftableGauge,
    Target,
    TargetFallback,
    TargetRate,
    Total,
    Skin,
    SkinPack,
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
    WindowResolution,
    WindowMode,
    Language,
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
    SyncRecords,
    AutoUploadReplay,
    AutoUploadTrace,
    Rivals,
    IrProfiles,
    MasterVolume,
    KeyVolume,
    BgmVolume,
    SystemVolume,
    AudioDevice,
    AudioBuffer,
    AudioSampleRate,
    AudioPolyphony,
    SoundFolder,
    GuideSe,
    KeepGreenNumber,
    GreenNumber,
    ShowMissingCharts,
    RankingKeys,
    DownloadFolder,
    DiscordStatus,
}

/// Rows the settings screen has.
pub const SETTING_COUNT: usize = 99;

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
        SettingId::KeyDebounce,
        SettingId::PadDebounce,
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
        SettingId::CourseGauge,
        SettingId::GaugeAutoShift,
        SettingId::BottomShiftableGauge,
        SettingId::Target,
        SettingId::TargetFallback,
        SettingId::TargetRate,
        SettingId::Total,
        SettingId::Skin,
        SettingId::SkinPack,
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
        SettingId::WindowResolution,
        SettingId::WindowMode,
        SettingId::Language,
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
        SettingId::SyncRecords,
        SettingId::AutoUploadReplay,
        SettingId::AutoUploadTrace,
        SettingId::Rivals,
        SettingId::IrProfiles,
        SettingId::MasterVolume,
        SettingId::KeyVolume,
        SettingId::BgmVolume,
        SettingId::SystemVolume,
        SettingId::AudioDevice,
        SettingId::AudioBuffer,
        SettingId::AudioSampleRate,
        SettingId::AudioPolyphony,
        SettingId::SoundFolder,
        SettingId::GuideSe,
        SettingId::KeepGreenNumber,
        SettingId::GreenNumber,
        SettingId::ShowMissingCharts,
        SettingId::RankingKeys,
        SettingId::DownloadFolder,
        SettingId::DiscordStatus,
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

/// The groups the rows are listed in, and the screen that lists each.
///
/// The settings screen shows the groups of [`SettingTab::SHOWN`] as the tabs of its strip. The play
/// menu lists the four of [`SettingTab::PLAY_MENU`] one under another, beside a preview of what
/// they do: they are the rows a player changes from one sitting to the next. [`SettingTab::Skin`]
/// is the group the skin menu lists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingTab {
    /// How fast the notes scroll, on the play menu.
    PlaySpeed,
    /// What the lane is covered with, on the play menu.
    PlayLane,
    /// When a press counts, on the play menu.
    PlayJudge,
    /// How a chart is dealt out and played, on the play menu.
    PlayChart,
    Judge,
    Display,
    Skin,
    Input,
    Network,
    Audio,
    Select,
}

impl SettingTab {
    /// Every group of rows, in the order the table lists them.
    pub const ALL: [SettingTab; 11] = [
        SettingTab::PlaySpeed,
        SettingTab::PlayLane,
        SettingTab::PlayJudge,
        SettingTab::PlayChart,
        SettingTab::Judge,
        SettingTab::Display,
        SettingTab::Skin,
        SettingTab::Input,
        SettingTab::Network,
        SettingTab::Audio,
        SettingTab::Select,
    ];

    /// The tabs the settings screen shows on its tab strip, left to right: every group but the
    /// play menu's and the skin menu's.
    pub const SHOWN: [SettingTab; 6] = [SettingTab::Judge, SettingTab::Display, SettingTab::Input, SettingTab::Network, SettingTab::Audio, SettingTab::Select];

    /// The groups the play menu lists, top to bottom.
    pub const PLAY_MENU: [SettingTab; 4] = [SettingTab::PlaySpeed, SettingTab::PlayLane, SettingTab::PlayJudge, SettingTab::PlayChart];

    /// Name shown on the tab strip, or over the group on the play menu. A group of the play menu
    /// may share its name with a tab of the settings screen: the two are never listed together.
    pub fn label(self) -> &'static str {
        match self {
            SettingTab::PlaySpeed => "SPEED",
            SettingTab::PlayLane => "LANE",
            SettingTab::PlayJudge | SettingTab::Judge => "JUDGE",
            SettingTab::PlayChart => "CHART",
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
    /// A whole number, shown with `unit` after it. `step` is what one call to [`adjust`] moves it
    /// by, which for some rows is more than the smallest change the row holds ([`number_spec`]).
    IntRange { min: i32, max: i32, step: i32, unit: &'static str },
    /// A fractional number, shown to `decimals` places with `unit` after it.
    FloatRange { min: f64, max: f64, step: f64, decimals: u8, unit: &'static str },
    /// A fraction of one, shown in percent.
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
    /// memory, an override that came from the command line, or another file the program keeps. [`display_value`] answers with what
    /// the document alone can say, and the program replaces it.
    pub host_value: bool,
    /// Whether the row is shown at all for this configuration.
    pub visible: fn(&Config) -> bool,
    /// What becomes of the row on a screen a skin draws, or `None` for a row every screen reads
    /// whichever way it is drawn.
    pub skinned: Option<SkinnedRow>,
}

/// A row whose value only the built-in screens read: a skin that draws one of those screens lays it
/// out by its own document and never asks for the row.
///
/// The row is kept and stays adjustable, because the built-in screens are still what a chart with
/// no skin document for its mode is played on, and what a skin that cannot be read falls back to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkinnedRow {
    /// The screens whose built-in drawing reads the row, as `SkinType` ids.
    pub screens: &'static [i32],
    /// What the settings screen says beside the row while a skin draws one of those screens.
    pub note: &'static str,
}

impl SettingDescriptor {
    /// The same row, read by the built-in drawing of `screens` and by no skin that draws them.
    const fn unread_by_skins(self, screens: &'static [i32], note: &'static str) -> SettingDescriptor {
        SettingDescriptor { skinned: Some(SkinnedRow { screens, note }), ..self }
    }
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

/// Every row, group by group in the order of [`SettingTab::ALL`] and top to bottom within a group.
pub const SETTINGS: &[SettingDescriptor] = &[
    row(
        SettingId::KeepGreenNumber,
        SettingTab::PlaySpeed,
        "KEEP GREEN NUMBER",
        SettingKind::Toggle,
        "Start every chart at the speed that shows GREEN NUMBER, whatever its tempo and lane cover; HI-SPEED FIX names the tempo (MAIN while it is OFF)",
    ),
    row(
        SettingId::GreenNumber,
        SettingTab::PlaySpeed,
        "GREEN NUMBER",
        SettingKind::IntRange { min: GREEN_NUMBER_MIN, max: GREEN_NUMBER_MAX, step: GREEN_NUMBER_ROW_STEP, unit: NO_UNIT },
        "The green number KEEP GREEN NUMBER holds: three fifths of the milliseconds a note is on show",
    ),
    row(
        SettingId::HiSpeed,
        SettingTab::PlaySpeed,
        "HI-SPEED",
        SettingKind::FloatRange { min: HISPEED_MIN, max: HISPEED_MAX, step: HISPEED_STEP, decimals: HISPEED_DECIMALS, unit: NO_UNIT },
        "How fast the notes scroll; a HI-SPEED key of a run moves it by HI-SPEED STEP. Worked out for every chart while KEEP GREEN NUMBER is on",
    ),
    row(
        SettingId::HiSpeedStep,
        SettingTab::PlaySpeed,
        "HI-SPEED STEP",
        SettingKind::FloatRange { min: HISPEED_STEP_MIN, max: HISPEED_STEP_MAX, step: HISPEED_STEP_STEP, decimals: HISPEED_DECIMALS, unit: NO_UNIT },
        "How far one press of a HI-SPEED key moves the scroll during a run and on the option panel",
    ),
    row(
        SettingId::FixHiSpeed,
        SettingTab::PlaySpeed,
        "HI-SPEED FIX",
        SettingKind::Cycle { values: HISPEED_FIX_LABELS },
        "Which tempo the note travel time is held constant against",
    ),
    row(
        SettingId::SpeedFix,
        SettingTab::PlaySpeed,
        "SPEED FIX",
        SettingKind::Cycle { values: SPEED_FIX_LABELS },
        "Whether the scroll speed follows the chart's tempo changes",
    ),
    row(SettingId::EnableCover, SettingTab::PlayLane, "LANE COVER ON", SettingKind::Toggle, "Apply the LANE COVER height"),
    row(
        SettingId::LaneCover,
        SettingTab::PlayLane,
        "LANE COVER",
        SettingKind::Percent { min: LANE_SHADE_MIN, max: LANE_SHADE_MAX, step: LANE_SHADE_STEP },
        "Hide the top of the lane",
    ),
    row(SettingId::EnableLift, SettingTab::PlayLane, "LIFT ON", SettingKind::Toggle, "Apply the LIFT height"),
    row(
        SettingId::Lift,
        SettingTab::PlayLane,
        "LIFT",
        SettingKind::Percent { min: LANE_SHADE_MIN, max: LANE_SHADE_MAX, step: LANE_SHADE_STEP },
        "Raise the judge line up the lane",
    ),
    row(SettingId::EnableHidden, SettingTab::PlayLane, "HIDDEN+ ON", SettingKind::Toggle, "Apply the HIDDEN+ height"),
    row(
        SettingId::Hidden,
        SettingTab::PlayLane,
        "HIDDEN+",
        SettingKind::Percent { min: LANE_SHADE_MIN, max: LANE_SHADE_MAX, step: LANE_SHADE_STEP },
        "Hide the bottom of the lane above the judge line",
    ),
    row(
        SettingId::LaneCoverFineStep,
        SettingTab::PlayLane,
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
    row(
        SettingId::JudgeOffset,
        SettingTab::PlayJudge,
        "JUDGE OFFSET",
        SettingKind::IntRange { min: JUDGE_OFFSET_MIN_MS, max: JUDGE_OFFSET_MAX_MS, step: JUDGE_OFFSET_STEP_MS, unit: MILLISECOND_UNIT },
        "Draw the notes this many ms earlier (+) or later (-) than the song; raise it if you hit late, lower it if you hit early",
    ),
    row(
        SettingId::AutoCal,
        SettingTab::PlayJudge,
        "AUTO ADJUST",
        SettingKind::Toggle,
        "Move JUDGE OFFSET a little after every hit that lands early or late, so the notes come to where you hit them",
    ),
    row(SettingId::Random, SettingTab::PlayChart, "RANDOM", SettingKind::Cycle { values: RANDOM_LABELS }, "How the lanes are shuffled"),
    unbuilt_row(
        SettingId::LaneOption,
        SettingTab::PlayChart,
        "LANE OPTION",
        SettingKind::Cycle { values: LANE_OPTION_LABELS },
        "What is done to the sides of the chart as a whole",
    ),
    row(SettingId::Gauge, SettingTab::PlayChart, "GAUGE", SettingKind::Cycle { values: GAUGE_LABELS }, "Which gauge the run is cleared on"),
    row(SettingId::LegacyNote, SettingTab::PlayChart, "LEGACY NOTE", SettingKind::Toggle, "Draw long notes as plain notes"),
    row(
        SettingId::Target,
        SettingTab::PlayChart,
        "TARGET",
        SettingKind::Cycle { values: TARGET_LABELS },
        "What the run is paced against. LOCAL BEST, IR BEST and RIVAL need that score to exist; without it the run is paced against TARGET FALLBACK",
    ),
    row(
        SettingId::TargetFallback,
        SettingTab::PlayChart,
        "TARGET FALLBACK",
        SettingKind::Cycle { values: TARGET_FALLBACK_LABELS },
        "What the run is paced against when TARGET names a score that is not there: no record on the chart yet, none on the score server, no rival score",
    ),
    row(
        SettingId::TargetRate,
        SettingTab::PlayChart,
        "TARGET RATE",
        SettingKind::IntRange { min: TARGET_RATE_MIN_PERCENT, max: TARGET_RATE_MAX_PERCENT, step: TARGET_RATE_STEP_PERCENT, unit: PERCENT_UNIT },
        "The share of the chart's best possible score that RATE CUSTOM asks for, as a TARGET or as a TARGET FALLBACK",
    ),
    row(SettingId::Autoplay, SettingTab::PlayChart, "AUTOPLAY", SettingKind::Toggle, "Let the game play the chart itself"),
    row(
        SettingId::ScratchAuto,
        SettingTab::PlayChart,
        "SCRATCH AUTO",
        SettingKind::Toggle,
        "Let the game play the scratch lane, drawn green; its notes are not counted",
    ),
    row(SettingId::AutoReplay, SettingTab::PlayChart, "AUTO REPLAY", SettingKind::Toggle, "Save a replay of every run"),
    row(
        SettingId::PlayEscapeMode,
        SettingTab::PlayChart,
        "ESCAPE",
        SettingKind::Cycle { values: PLAY_ESCAPE_LABELS },
        "What Escape has to be to abandon a run",
    ),
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
        SettingId::CourseGauge,
        SettingTab::Judge,
        "COURSE GAUGE",
        SettingKind::Cycle { values: COURSE_GAUGE_LABELS },
        "Which gauge table a course is played on, ahead of the course's own gauge constraint",
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
    row(
        SettingId::Total,
        SettingTab::Judge,
        "TOTAL",
        SettingKind::FloatRange { min: TOTAL_FROM_CHART, max: TOTAL_MAX, step: TOTAL_STEP, decimals: TOTAL_DECIMALS, unit: NO_UNIT },
        "Override the chart's own gauge total",
    ),
    row(SettingId::Language, SettingTab::Display, "LANGUAGE", SettingKind::Cycle { values: LANGUAGE_LABELS }, "The language the screens are written in"),
    host_row(SettingId::Skin, SettingTab::Display, "SKIN", SettingKind::Cycle { values: SKIN_LABELS }, "Which bundled skin the play screen uses")
        .unread_by_skins(PLAY_SCREENS, BUILT_IN_ONLY_NOTE),
    row(SettingId::Font, SettingTab::Display, "FONT", SettingKind::FilePick, "Font the screens are drawn with"),
    row(
        SettingId::ScoreGraph,
        SettingTab::Display,
        "SCORE GRAPH",
        SettingKind::Toggle,
        "Show score ranks and score deltas on the built-in screens and in record lists",
    ),
    row(SettingId::ResultGraphs, SettingTab::Display, "RESULT GRAPHS", SettingKind::Toggle, "Draw the gauge and timing graphs on the result screen")
        .unread_by_skins(RESULT_SCREENS, BUILT_IN_ONLY_NOTE),
    row(SettingId::ReplayAnalysis, SettingTab::Display, "REPLAY ANALYSIS", SettingKind::Toggle, "Allow stepping and slowing a replay"),
    row(SettingId::Preview, SettingTab::Display, "PREVIEW", SettingKind::Toggle, "Play the focused song on the song list"),
    row(SettingId::WhiteNumber, SettingTab::Display, "WHITE NUMBER", SettingKind::Toggle, "Show the travel time to the bottom of the cover")
        .unread_by_skins(PLAY_SCREENS, BUILT_IN_ONLY_NOTE),
    row(
        SettingId::JudgeTextY,
        SettingTab::Display,
        "JUDGE TEXT Y",
        SettingKind::Percent { min: JUDGE_TEXT_Y_MIN, max: JUDGE_TEXT_Y_MAX, step: JUDGE_TEXT_Y_STEP },
        "Where the judgement text sits, or 0 for the skin's own place",
    )
    .unread_by_skins(PLAY_SCREENS, BUILT_IN_ONLY_NOTE),
    row(SettingId::FiveKeyLayout, SettingTab::Display, "5KEYS LAYOUT", SettingKind::Toggle, "Give a five-key chart its own lane widths")
        .unread_by_skins(FIVE_KEY_SCREENS, BUILT_IN_ONLY_NOTE),
    row(
        SettingId::Letterbox,
        SettingTab::Display,
        "LETTERBOX",
        SettingKind::Toggle,
        "Keep the skin's own aspect ratio inside the window; off stretches it to the window. A borderless window always keeps it",
    ),
    row(
        SettingId::WindowResolution,
        SettingTab::Display,
        "RESOLUTION",
        SettingKind::Cycle { values: WINDOW_RESOLUTION_LABELS },
        "Size of the window; a borderless window takes its monitor's size instead",
    ),
    row(
        SettingId::WindowMode,
        SettingTab::Display,
        "WINDOW MODE",
        SettingKind::Cycle { values: WINDOW_MODE_LABELS },
        "An ordinary window or one covering the monitor",
    ),
    row(SettingId::Bga, SettingTab::Display, "BGA", SettingKind::Toggle, "Show the chart's background animation"),
    row(SettingId::DebugMode, SettingTab::Display, "DEBUG MODE", SettingKind::Toggle, "Draw the frame and audio counters"),
    host_row(SettingId::SkinPack, SettingTab::Skin, "PACK FOLDER", SettingKind::Action, "Folder of skin documents every screen takes its own from"),
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
    )
    .unread_by_skins(SCRATCH_SCREENS, SKIN_OPTION_NOTE),
    row(SettingId::KeyConfig, SettingTab::Input, "KEY CONFIG", SettingKind::Action, "Open the key editor"),
    row(
        SettingId::KeyDebounce,
        SettingTab::Input,
        "KEY DEBOUNCE",
        SettingKind::IntRange { min: KEY_DEBOUNCE_MIN_MS as i32, max: KEY_DEBOUNCE_MAX_MS as i32, step: KEY_DEBOUNCE_STEP_MS as i32, unit: MILLISECOND_UNIT },
        "How long a lane key is held after it goes down, so a chattering switch plays as one press",
    ),
    host_row(
        SettingId::PadDebounce,
        SettingTab::Input,
        "PAD DEBOUNCE",
        SettingKind::IntRange { min: PAD_DEBOUNCE_MIN_MS as i32, max: PAD_DEBOUNCE_MAX_MS as i32, step: PAD_DEBOUNCE_STEP_MS as i32, unit: MILLISECOND_UNIT },
        "How long a controller button is held after it goes down, so a chattering switch plays as one press",
    ),
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
    row(SettingId::SyncRecords, SettingTab::Network, "SYNC RECORDS NOW", SettingKind::Action, "Send the runs the score server lacks"),
    row(SettingId::AutoUploadReplay, SettingTab::Network, "AUTO UPLOAD REPLAY", SettingKind::Toggle, "Send the replay after a ranked score"),
    row(SettingId::AutoUploadTrace, SettingTab::Network, "AUTO UPLOAD TRACE", SettingKind::Toggle, "Send each run's note-by-note record after its score"),
    row(SettingId::Rivals, SettingTab::Network, "RIVALS", SettingKind::Action, "Players shown next to you on the ranking"),
    row(SettingId::IrProfiles, SettingTab::Network, "IR PROFILES", SettingKind::Action, "Extra score servers this play is also sent to"),
    row(SettingId::DiscordStatus, SettingTab::Network, "DISCORD STATUS", SettingKind::Toggle, "Show what you play on your Discord profile"),
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
    row(SettingId::SoundFolder, SettingTab::Audio, "SOUND FOLDER", SettingKind::Action, "Folder the system sound set is read from"),
    row(SettingId::GuideSe, SettingTab::Audio, "GUIDE SE", SettingKind::Toggle, "Sound a cue on every judgement"),
    row(SettingId::Sort, SettingTab::Select, "SORT", SettingKind::Cycle { values: SORT_LABELS }, "How the song list is ordered"),
    row(SettingId::FavoriteOnly, SettingTab::Select, "FAVORITE ONLY", SettingKind::Toggle, "List only the charts marked as favourites"),
    row(
        SettingId::ShowMissingCharts,
        SettingTab::Select,
        "MISSING CHARTS",
        SettingKind::Toggle,
        "List the charts a difficulty table names that the library does not have, dimmed",
    ),
    row(SettingId::RankingKeys, SettingTab::Select, "RANKING KEYS", SettingKind::Toggle, "Hold a blue key on a chart to open its IR ranking"),
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
    row(
        SettingId::DownloadFolder,
        SettingTab::Select,
        "DOWNLOAD FOLDER",
        SettingKind::Action,
        "Folder the charts downloaded from the song list are unpacked into; scanned with the song folders",
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
    SettingDescriptor { id, tab, label, kind, help, host_value: false, visible: always, skinned: None }
}

const fn host_row(id: SettingId, tab: SettingTab, label: &'static str, kind: SettingKind, help: &'static str) -> SettingDescriptor {
    SettingDescriptor { id, tab, label, kind, help, host_value: true, visible: always, skinned: None }
}

/// A row whose value nothing acts on yet.
///
/// It keeps its descriptor — its label, its vocabulary and its place in the table — so the value can
/// still be stored and read back, and so building the feature is a one-line change here. It is not
/// shown, because a row a player can move that changes nothing about the run reads as a chosen
/// option rather than an unbuilt one. What is still missing for each is in the divergence ledger.
const fn unbuilt_row(id: SettingId, tab: SettingTab, label: &'static str, kind: SettingKind, help: &'static str) -> SettingDescriptor {
    SettingDescriptor { id, tab, label, kind, help, host_value: false, visible: never, skinned: None }
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

/// What the settings screen says beside one row, or `None` for a row that reads as it always has.
///
/// `drawn_by_skin` answers, for one `SkinType` id, whether a skin draws that screen. A row only the
/// built-in screens read is annotated as soon as a skin draws any one of the screens it is for: the
/// player can then walk into a screen the row does nothing on, which is what the note is there to
/// say. While the built-in screens draw every one of them the row is shown bare.
pub fn skinned_note(id: SettingId, drawn_by_skin: impl Fn(i32) -> bool) -> Option<&'static str> {
    let skinned = descriptor(id).skinned?;
    skinned.screens.iter().any(|screen| drawn_by_skin(*screen)).then_some(skinned.note)
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
        SettingId::KeepGreenNumber => on_off(config.play.keep_green_number),
        SettingId::GreenNumber => green_of_duration(config.play.duration_ms).to_string(),
        SettingId::Random => config.play.random.label().to_string(),
        SettingId::LaneOption => cycle_label(id, cycle_at(options::LaneOption::ALL, config.play.lane_option)),
        SettingId::LegacyNote => on_off(config.play.legacy_note),
        SettingId::FiveKeyLayout => on_off(config.display.five_key_layout),
        SettingId::PlayEscapeMode => cycle_label(id, cycle_at(options::PlayEscape::ALL, config.play.play_escape)),
        SettingId::Gauge => cycle_label(id, gauge_at(config.play.gauge)),
        SettingId::Lift | SettingId::LaneCover | SettingId::Hidden => number_text(config, id),
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
        | SettingId::SyncRecords
        | SettingId::SkinReload
        | SettingId::SkinReset => ACTION_VALUE.to_string(),
        SettingId::SkinPack => optional_text(config.skin.pack.as_deref()),
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
        SettingId::CourseGauge => cycle_label(id, cycle_at(CourseGaugeMode::ALL, config.judge.course_gauge)),
        SettingId::GaugeAutoShift => cycle_label(id, cycle_at(GaugeAutoShift::ALL, config.judge.gauge_auto_shift)),
        SettingId::BottomShiftableGauge => cycle_label(id, cycle_at(BOTTOM_SHIFTABLE_GAUGES, config.judge.bottom_shiftable_gauge)),
        SettingId::Target => config.judge.target.label().to_string(),
        SettingId::TargetFallback => config.judge.target_fallback.label().to_string(),
        SettingId::TargetRate => format!("{}{}", config.judge.target_rate_percent, unit_of(id)),
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
        SettingId::EnableHidden => on_off(config.play.enable_hidden),
        SettingId::LaneCoverFineStep => format!("{:.*}", LANE_SHADE_FINE_DECIMALS as usize, config.play.lanecover_step_fine),
        SettingId::WhiteNumber => on_off(config.display.show_white_number),
        SettingId::JudgeTextY => shade_value(config.display.judge_text_y, id),
        SettingId::Letterbox => on_off(config.display.letterbox),
        SettingId::WindowResolution => config.display.window_resolution.label().to_string(),
        SettingId::WindowMode => config.display.window_mode.label().to_string(),
        SettingId::Language => config.display.language.label().to_string(),
        SettingId::Sort => config.library.sort.label().to_string(),
        SettingId::FavoriteOnly => on_off(config.library.favorite_only),
        SettingId::ShowMissingCharts => on_off(config.library.show_missing_charts),
        SettingId::RankingKeys => on_off(config.library.ranking_keys),
        SettingId::DownloadFolder => optional_text(config.library.download_folder.as_deref()),
        SettingId::PreviewVolume => shade_value(config.library.preview_volume, id),
        SettingId::PreviewFade => format!("{}{}", config.library.preview_fade_ms, unit_of(id)),
        SettingId::KeyDebounce => format!("{}{}", config.play.key_debounce_ms, unit_of(id)),
        SettingId::PadDebounce => format!("{DEFAULT_PAD_DEBOUNCE_MS}{}", unit_of(id)),
        SettingId::ServerUrl => optional_text(config.network.server_url.as_deref()),
        SettingId::PlayerId => config.network.player_id.clone(),
        SettingId::Account => config.network.ir_login_id.clone().unwrap_or_else(|| DEFAULT_PLAYER_ID.to_string()),
        SettingId::Email => optional_text(config.network.ir_email.as_deref()),
        SettingId::Password => UNSET_VALUE.to_string(),
        SettingId::SyncSettings => on_off(config.network.sync_settings),
        SettingId::AutoUploadReplay => on_off(config.network.auto_upload_replay),
        SettingId::AutoUploadTrace => on_off(config.network.auto_upload_trace),
        SettingId::DiscordStatus => on_off(config.network.discord_status),
        SettingId::Rivals => config.network.rivals.len().to_string(),
        SettingId::IrProfiles => config.network.ir_profiles.len().to_string(),
        SettingId::MasterVolume => gain_value(config.audio.master, id),
        SettingId::KeyVolume => gain_value(config.audio.key, id),
        SettingId::BgmVolume => gain_value(config.audio.bg, id),
        SettingId::SystemVolume => gain_value(config.audio.system, id),
        SettingId::AudioDevice => config.audio.device.clone().unwrap_or_else(|| DEFAULT_VALUE.to_string()),
        SettingId::AudioBuffer => auto_or_number(config.audio.buffer_frames),
        SettingId::AudioSampleRate => auto_or_number(config.audio.sample_rate),
        SettingId::AudioPolyphony => config.audio.polyphony.to_string(),
        SettingId::SoundFolder => optional_text(config.audio.sound_folder.as_deref()),
        SettingId::GuideSe => on_off(config.audio.guide_se),
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

/// A share of one moved `delta` steps of `step` along the grid those steps make from nothing.
///
/// The share is counted in whole steps, moved, and divided back by how many steps make one, so a
/// row stepped a hundred times holds the nearest value there is to each hundredth. Adding the step
/// to what the row holds instead lets each addition's rounding pile up, and the row comes to stand
/// a hair short of the percent it shows, which a figure cut from it in thousandths then reads one
/// low.
fn stepped_share(share: f32, delta: i32, step: f32) -> f32 {
    let steps_in_one = (1.0 / step).round();
    ((share * steps_in_one).round() + delta as f32) / steps_in_one
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
        SettingId::KeepGreenNumber => toggle(&mut config.play.keep_green_number),
        SettingId::GreenNumber => {
            let next = duration_stepped_by_green(config.play.duration_ms, delta.saturating_mul(GREEN_NUMBER_ROW_STEP));
            store(&mut config.play.duration_ms, next)
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
            let next = stepped_share(config.play.lift, delta, LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
            store(&mut config.play.lift, next)
        }
        SettingId::LaneCover => {
            let next = stepped_share(config.play.cover, delta, LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
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
        SettingId::CourseGauge => {
            let at = stepped(cycle_at(CourseGaugeMode::ALL, config.judge.course_gauge), CourseGaugeMode::ALL.len(), delta);
            store(&mut config.judge.course_gauge, CourseGaugeMode::ALL[at])
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
        SettingId::TargetFallback => {
            let at = stepped(cycle_at(ScoreTarget::FALLBACKS, config.judge.target_fallback), ScoreTarget::FALLBACKS.len(), delta);
            store(&mut config.judge.target_fallback, ScoreTarget::FALLBACKS[at])
        }
        SettingId::TargetRate => {
            let next = (config.judge.target_rate_percent + delta * TARGET_RATE_STEP_PERCENT).clamp(TARGET_RATE_MIN_PERCENT, TARGET_RATE_MAX_PERCENT);
            store(&mut config.judge.target_rate_percent, next)
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
        SettingId::WindowResolution => {
            let at = stepped(cycle_at(WindowResolution::ALL, config.display.window_resolution), WindowResolution::ALL.len(), delta);
            store(&mut config.display.window_resolution, WindowResolution::ALL[at])
        }
        SettingId::WindowMode => {
            let at = stepped(cycle_at(WindowMode::ALL, config.display.window_mode), WindowMode::ALL.len(), delta);
            store(&mut config.display.window_mode, WindowMode::ALL[at])
        }
        SettingId::Language => {
            let at = stepped(cycle_at(LanguageChoice::ALL, config.display.language), LanguageChoice::ALL.len(), delta);
            store(&mut config.display.language, LanguageChoice::ALL[at])
        }
        SettingId::FavoriteOnly => toggle(&mut config.library.favorite_only),
        SettingId::ShowMissingCharts => toggle(&mut config.library.show_missing_charts),
        SettingId::RankingKeys => toggle(&mut config.library.ranking_keys),
        SettingId::Hidden => {
            let next = stepped_share(config.play.hidden, delta, LANE_SHADE_STEP).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX);
            store(&mut config.play.hidden, next)
        }
        SettingId::LaneCoverFineStep => {
            let next = (config.play.lanecover_step_fine + delta as f32 * LANE_SHADE_FINE_STEP_STEP).clamp(LANE_SHADE_FINE_STEP_MIN, LANE_SHADE_FINE_STEP_MAX);
            store(&mut config.play.lanecover_step_fine, next)
        }
        SettingId::JudgeTextY => {
            let next = stepped_share(config.display.judge_text_y, delta, JUDGE_TEXT_Y_STEP).clamp(JUDGE_TEXT_Y_MIN, JUDGE_TEXT_Y_MAX);
            store(&mut config.display.judge_text_y, next)
        }
        SettingId::Sort => {
            let at = stepped(cycle_at(SortMode::SELECTABLE, config.library.sort), SortMode::SELECTABLE.len(), delta);
            store(&mut config.library.sort, SortMode::SELECTABLE[at])
        }
        SettingId::PreviewVolume => {
            let next = stepped_share(config.library.preview_volume, delta, PREVIEW_VOLUME_STEP).clamp(PREVIEW_VOLUME_MIN, PREVIEW_VOLUME_MAX);
            store(&mut config.library.preview_volume, next)
        }
        SettingId::PreviewFade => {
            let moved = config.library.preview_fade_ms as i32 + delta * PREVIEW_FADE_STEP_MS as i32;
            let next = moved.clamp(PREVIEW_FADE_MIN_MS as i32, PREVIEW_FADE_MAX_MS as i32) as u32;
            store(&mut config.library.preview_fade_ms, next)
        }
        SettingId::KeyDebounce => {
            let moved = config.play.key_debounce_ms as i32 + delta * KEY_DEBOUNCE_STEP_MS as i32;
            let next = moved.clamp(KEY_DEBOUNCE_MIN_MS as i32, KEY_DEBOUNCE_MAX_MS as i32) as u32;
            store(&mut config.play.key_debounce_ms, next)
        }
        SettingId::SyncSettings => toggle(&mut config.network.sync_settings),
        SettingId::AutoUploadReplay => toggle(&mut config.network.auto_upload_replay),
        SettingId::AutoUploadTrace => toggle(&mut config.network.auto_upload_trace),
        SettingId::DiscordStatus => toggle(&mut config.network.discord_status),
        SettingId::GuideSe => toggle(&mut config.audio.guide_se),
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
        | SettingId::SkinPack
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
        | SettingId::SyncRecords
        | SettingId::Rivals
        | SettingId::IrProfiles
        | SettingId::SoundFolder
        | SettingId::DownloadFolder
        | SettingId::AudioDevice
        | SettingId::PadDebounce => AdjustOutcome::Action(id),
    }
}

/// How many steps of a row that holds a scroll speed make one: the row is counted in hundredths.
const HISPEED_STEPS_PER_ONE: i64 = 100;

/// How many steps of a lane shade -- the lane cover, the lift, the hidden cover -- make the whole
/// lane, and how many make one percent of it: a shade is counted in tenths of a percent, which is
/// one of the white number a cover reads as. It is shown to the one place that tells two steps
/// apart.
const SHADE_STEPS_PER_LANE: i64 = 1000;
const SHADE_STEPS_PER_PERCENT: i64 = 10;
const SHADE_DECIMALS: u8 = 1;

/// How many steps of the fine cover step make one: the row is counted in ten-thousandths.
const SHADE_FINE_STEPS_PER_ONE: i64 = 10_000;

/// How many steps of a share shown in whole percent make the whole of it.
const PERCENT_STEPS_PER_ONE: i64 = 100;

/// The largest TOTAL the row holds when it is set by hand, which is the ceiling the reference's
/// practice screen holds a total to (`PracticeConfiguration.java:616`). A larger one a settings
/// file brings is read as it is until the row is next set.
pub const TOTAL_ROW_MAX: i64 = 5000;

/// A number as a whole count of steps, `per_one` of them to one.
fn counted(value: f64, per_one: i64) -> i64 {
    (value * per_one as f64).round() as i64
}

/// A count of steps as the share of one it stands for, `per_one` of them to the whole.
fn share_of(steps: i64, per_one: i64) -> f32 {
    steps as f32 / per_one as f32
}

/// The most values above the nearest one that are tried for a share of the lane that is not read
/// short ([`shade_of`]).
const SHADE_NUDGES: usize = 4;

/// A count of steps of a lane shade as the share of the lane it stands for: the nearest value to
/// it that a skin, which shows a shade in thousandths with the rest cut off, does not read one
/// short. A quarter of the lane and a thousandth more is stored a hair above 0.251 rather than a
/// hair below it, which a figure cut from it would show as 250.
fn shade_of(steps: i64) -> f32 {
    let mut share = share_of(steps, SHADE_STEPS_PER_LANE);
    for _ in 0..SHADE_NUDGES {
        if share * SHADE_STEPS_PER_LANE as f32 >= steps as f32 {
            break;
        }
        share = share.next_up();
    }
    share
}

/// The steps of one row that holds a number, or `None` for a row that holds none: a switch, a
/// list, text, or something that is run.
///
/// The steps are the smallest change each row can hold, which is what one press of a key moves it
/// by on the screens that set a row by hand, what a slider dragged along the row lands on, and
/// what a figure typed into it is rounded to. [`adjust`] moves some rows further at a press: that
/// is the step of the option panel and of the events a skin sends, which this does not change.
pub fn number_spec(id: SettingId) -> Option<NumberSpec> {
    let spec = match id {
        SettingId::GreenNumber => NumberSpec::whole(i64::from(GREEN_NUMBER_MIN), i64::from(GREEN_NUMBER_MAX), NO_UNIT),
        SettingId::HiSpeed => NumberSpec::fine(
            counted(HISPEED_MIN, HISPEED_STEPS_PER_ONE),
            counted(HISPEED_MAX, HISPEED_STEPS_PER_ONE),
            HISPEED_STEPS_PER_ONE,
            HISPEED_DECIMALS,
            NO_UNIT,
        ),
        SettingId::HiSpeedStep => NumberSpec::fine(
            counted(HISPEED_STEP_MIN, HISPEED_STEPS_PER_ONE),
            counted(HISPEED_STEP_MAX, HISPEED_STEPS_PER_ONE),
            HISPEED_STEPS_PER_ONE,
            HISPEED_DECIMALS,
            NO_UNIT,
        ),
        SettingId::LaneCover | SettingId::Lift | SettingId::Hidden => NumberSpec::fine(
            counted(f64::from(LANE_SHADE_MIN), SHADE_STEPS_PER_LANE),
            counted(f64::from(LANE_SHADE_MAX), SHADE_STEPS_PER_LANE),
            SHADE_STEPS_PER_PERCENT,
            SHADE_DECIMALS,
            PERCENT_UNIT,
        ),
        SettingId::LaneCoverFineStep => NumberSpec::fine(
            counted(f64::from(LANE_SHADE_FINE_STEP_MIN), SHADE_FINE_STEPS_PER_ONE),
            counted(f64::from(LANE_SHADE_FINE_STEP_MAX), SHADE_FINE_STEPS_PER_ONE),
            SHADE_FINE_STEPS_PER_ONE,
            LANE_SHADE_FINE_DECIMALS,
            NO_UNIT,
        ),
        SettingId::JudgeOffset => NumberSpec::whole(i64::from(JUDGE_OFFSET_MIN_MS), i64::from(JUDGE_OFFSET_MAX_MS), MILLISECOND_UNIT).with_sign(),
        SettingId::JudgeWidthKeyPGreat
        | SettingId::JudgeWidthKeyGreat
        | SettingId::JudgeWidthKeyGood
        | SettingId::JudgeWidthScratchPGreat
        | SettingId::JudgeWidthScratchGreat
        | SettingId::JudgeWidthScratchGood => NumberSpec::whole(i64::from(JUDGE_RATE_MIN_PERCENT), i64::from(JUDGE_RATE_MAX_PERCENT), PERCENT_UNIT),
        SettingId::LongNoteMargin => NumberSpec::whole(i64::from(LN_MARGIN_MIN_PERCENT), i64::from(LN_MARGIN_MAX_PERCENT), PERCENT_UNIT),
        SettingId::TargetRate => NumberSpec::whole(i64::from(TARGET_RATE_MIN_PERCENT), i64::from(TARGET_RATE_MAX_PERCENT), PERCENT_UNIT),
        SettingId::Total => NumberSpec::whole(counted(TOTAL_FROM_CHART, 1), TOTAL_ROW_MAX, NO_UNIT),
        SettingId::JudgeTextY => NumberSpec::whole(
            counted(f64::from(JUDGE_TEXT_Y_MIN), PERCENT_STEPS_PER_ONE),
            counted(f64::from(JUDGE_TEXT_Y_MAX), PERCENT_STEPS_PER_ONE),
            PERCENT_UNIT,
        ),
        SettingId::PreviewVolume => NumberSpec::whole(
            counted(f64::from(PREVIEW_VOLUME_MIN), PERCENT_STEPS_PER_ONE),
            counted(f64::from(PREVIEW_VOLUME_MAX), PERCENT_STEPS_PER_ONE),
            PERCENT_UNIT,
        ),
        SettingId::PreviewFade => NumberSpec::whole(i64::from(PREVIEW_FADE_MIN_MS), i64::from(PREVIEW_FADE_MAX_MS), MILLISECOND_UNIT),
        SettingId::KeyDebounce => NumberSpec::whole(i64::from(KEY_DEBOUNCE_MIN_MS), i64::from(KEY_DEBOUNCE_MAX_MS), MILLISECOND_UNIT),
        SettingId::PadDebounce => NumberSpec::whole(i64::from(PAD_DEBOUNCE_MIN_MS), i64::from(PAD_DEBOUNCE_MAX_MS), MILLISECOND_UNIT),
        SettingId::MasterVolume | SettingId::KeyVolume | SettingId::BgmVolume | SettingId::SystemVolume => NumberSpec::whole(
            counted(f64::from(AUDIO_VOLUME_MIN_GAIN), PERCENT_STEPS_PER_ONE),
            counted(f64::from(AUDIO_VOLUME_MAX_GAIN), PERCENT_STEPS_PER_ONE),
            PERCENT_UNIT,
        ),
        SettingId::AudioPolyphony => NumberSpec::whole(AUDIO_POLYPHONY_MIN_VOICES as i64, AUDIO_POLYPHONY_MAX_VOICES as i64, NO_UNIT),
        _ => return None,
    };
    Some(spec)
}

/// The step one row that holds a number stands on, held to the row's two ends, or `None` for a row
/// that holds none. A value between two steps -- one a settings file was written with by hand, or
/// a cover moved by its fine step during a run -- reads as the nearer. A row the running program
/// owns ([`SettingDescriptor::host_value`]) has its number outside the document, and reads `None`
/// here: the program counts it in the steps [`number_spec`] gives it.
pub fn number_steps(config: &Config, id: SettingId) -> Option<i64> {
    let spec = number_spec(id)?;
    let steps = match id {
        SettingId::GreenNumber => i64::from(green_of_duration(config.play.duration_ms)),
        SettingId::HiSpeed => counted(config.play.hispeed, HISPEED_STEPS_PER_ONE),
        SettingId::HiSpeedStep => counted(config.play.hispeed_step, HISPEED_STEPS_PER_ONE),
        SettingId::LaneCover => counted(f64::from(config.play.cover), SHADE_STEPS_PER_LANE),
        SettingId::Lift => counted(f64::from(config.play.lift), SHADE_STEPS_PER_LANE),
        SettingId::Hidden => counted(f64::from(config.play.hidden), SHADE_STEPS_PER_LANE),
        SettingId::LaneCoverFineStep => counted(f64::from(config.play.lanecover_step_fine), SHADE_FINE_STEPS_PER_ONE),
        SettingId::JudgeOffset => i64::from(config.judge.offset_ms),
        SettingId::LongNoteMargin => i64::from(config.judge.longnote_margin_rate),
        SettingId::TargetRate => i64::from(config.judge.target_rate_percent),
        SettingId::Total => counted(config.play.total_override, 1),
        SettingId::JudgeTextY => counted(f64::from(config.display.judge_text_y), PERCENT_STEPS_PER_ONE),
        SettingId::PreviewVolume => counted(f64::from(config.library.preview_volume), PERCENT_STEPS_PER_ONE),
        SettingId::PreviewFade => i64::from(config.library.preview_fade_ms),
        SettingId::KeyDebounce => i64::from(config.play.key_debounce_ms),
        SettingId::PadDebounce => return None,
        SettingId::MasterVolume => i64::from(volume_percent(config.audio.master)),
        SettingId::KeyVolume => i64::from(volume_percent(config.audio.key)),
        SettingId::BgmVolume => i64::from(volume_percent(config.audio.bg)),
        SettingId::SystemVolume => i64::from(volume_percent(config.audio.system)),
        SettingId::AudioPolyphony => config.audio.polyphony as i64,
        _ => i64::from(judge_width(config, id)?),
    };
    Some(spec.clamp(steps))
}

/// Put one row that holds a number on the step `steps`, held to the row's two ends, and report
/// whether the document moved. A row already on that step is left exactly as it is stored, and a
/// row that holds no number is left alone.
///
/// What is stored is the count of steps divided by how many make one, never a sum of steps, so a
/// row moved a step at a time holds the nearest value there is to each step however long it is
/// moved for.
pub fn set_number_steps(config: &mut Config, id: SettingId, steps: i64) -> AdjustOutcome {
    let Some(spec) = number_spec(id) else {
        return AdjustOutcome::Unchanged;
    };
    let steps = spec.clamp(steps);
    if number_steps(config, id) == Some(steps) {
        return AdjustOutcome::Unchanged;
    }
    let whole = steps as i32;
    match id {
        SettingId::GreenNumber => store(&mut config.play.duration_ms, duration_of_green(whole)),
        SettingId::HiSpeed => store(&mut config.play.hispeed, steps as f64 / HISPEED_STEPS_PER_ONE as f64),
        SettingId::HiSpeedStep => store(&mut config.play.hispeed_step, steps as f64 / HISPEED_STEPS_PER_ONE as f64),
        SettingId::LaneCover => store(&mut config.play.cover, shade_of(steps)),
        SettingId::Lift => store(&mut config.play.lift, shade_of(steps)),
        SettingId::Hidden => store(&mut config.play.hidden, shade_of(steps)),
        SettingId::LaneCoverFineStep => store(&mut config.play.lanecover_step_fine, share_of(steps, SHADE_FINE_STEPS_PER_ONE)),
        SettingId::JudgeOffset => store(&mut config.judge.offset_ms, whole),
        SettingId::LongNoteMargin => store(&mut config.judge.longnote_margin_rate, whole),
        SettingId::TargetRate => store(&mut config.judge.target_rate_percent, whole),
        SettingId::Total => store(&mut config.play.total_override, steps as f64),
        SettingId::JudgeTextY => store(&mut config.display.judge_text_y, share_of(steps, PERCENT_STEPS_PER_ONE)),
        SettingId::PreviewVolume => store(&mut config.library.preview_volume, share_of(steps, PERCENT_STEPS_PER_ONE)),
        SettingId::PreviewFade => store(&mut config.library.preview_fade_ms, steps as u32),
        SettingId::KeyDebounce => store(&mut config.play.key_debounce_ms, steps as u32),
        SettingId::MasterVolume => store(&mut config.audio.master, share_of(steps, PERCENT_STEPS_PER_ONE)),
        SettingId::KeyVolume => store(&mut config.audio.key, share_of(steps, PERCENT_STEPS_PER_ONE)),
        SettingId::BgmVolume => store(&mut config.audio.bg, share_of(steps, PERCENT_STEPS_PER_ONE)),
        SettingId::SystemVolume => store(&mut config.audio.system, share_of(steps, PERCENT_STEPS_PER_ONE)),
        SettingId::AudioPolyphony => {
            let outcome = store(&mut config.audio.polyphony, steps as usize);
            reopen(config, outcome)
        }
        _ => match judge_width_mut(config, id) {
            Some(rate) => store(rate, whole),
            None => AdjustOutcome::Unchanged,
        },
    }
}

/// Move one row that holds a number `by` steps, stopping at the row's two ends, and report whether
/// the document moved.
pub fn move_number_steps(config: &mut Config, id: SettingId, by: i64) -> AdjustOutcome {
    match (number_spec(id), number_steps(config, id)) {
        (Some(spec), Some(steps)) => set_number_steps(config, id, spec.moved(steps, by)),
        _ => AdjustOutcome::Unchanged,
    }
}

/// The step one row that holds a number stands on in a configuration nothing has been changed in,
/// which is where the key that resets a row puts it back to, or `None` for a row that holds none.
pub fn shipped_number_steps(id: SettingId) -> Option<i64> {
    static SHIPPED: std::sync::LazyLock<Config> = std::sync::LazyLock::new(Config::default);
    number_steps(&SHIPPED, id)
}

/// What a row that holds a number shows for the step it stands on, and nothing for any other row.
fn number_text(config: &Config, id: SettingId) -> String {
    match (number_spec(id), number_steps(config, id)) {
        (Some(spec), Some(steps)) => spec.text(steps),
        _ => String::new(),
    }
}

/// Step the SKIN row between the bundled skins. The skin the run was started with is a launch
/// override the document does not hold, so the program clears that before calling this.
pub fn step_skin(config: &mut Config) {
    let selected = SKIN_LABELS.iter().position(|skin| config.display.skin.eq_ignore_ascii_case(skin)).unwrap_or(0);
    config.display.skin = SKIN_LABELS[(selected + 1) % SKIN_LABELS.len()].to_string();
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
        assert!(!tab_rows(SettingTab::PlayChart, &config).contains(&SettingId::LaneOption));
        assert_eq!(display_value(&config, SettingId::LaneOption), OFF_VALUE, "the value still reads back for the file it is stored in");
    }

    /// The rows a skin never reads on the screens it draws, each with the screens its built-in
    /// drawing is for. SCORE GRAPH is deliberately not one of them: the record panel the browser
    /// opens over a skin still shows or hides its rank by it, which is why its help line names the
    /// record lists and promises nothing about a result screen a skin draws.
    #[test]
    fn the_rows_a_skin_does_not_read_are_the_built_in_layout_rows() {
        let skinned: Vec<(SettingId, &[i32], &str)> =
            SETTINGS.iter().filter_map(|row| row.skinned.map(|skinned| (row.id, skinned.screens, skinned.note))).collect();
        assert_eq!(
            skinned,
            vec![
                (SettingId::Skin, PLAY_SCREENS, BUILT_IN_ONLY_NOTE),
                (SettingId::ResultGraphs, RESULT_SCREENS, BUILT_IN_ONLY_NOTE),
                (SettingId::WhiteNumber, PLAY_SCREENS, BUILT_IN_ONLY_NOTE),
                (SettingId::JudgeTextY, PLAY_SCREENS, BUILT_IN_ONLY_NOTE),
                (SettingId::FiveKeyLayout, FIVE_KEY_SCREENS, BUILT_IN_ONLY_NOTE),
                (SettingId::ScratchSide, SCRATCH_SCREENS, SKIN_OPTION_NOTE),
            ]
        );
        let score_graph = descriptor(SettingId::ScoreGraph).help;
        assert!(score_graph.contains("built-in screens") && score_graph.contains("record lists"), "{score_graph:?}");
        for (id, screens, note) in skinned {
            assert!(!screens.is_empty() && !note.is_empty(), "{id:?} names no screen or says nothing");
            for screen in screens {
                assert!(skin_screen_label(*screen).is_some(), "{id:?} names screen {screen}, which no skin document declares");
            }
        }
    }

    #[test]
    fn a_row_is_bare_while_the_built_in_screens_draw_everything() {
        for id in SettingId::ALL {
            assert_eq!(skinned_note(id, |_| false), None, "{id:?} is annotated with no skin drawing anything");
        }
    }

    #[test]
    fn a_row_a_skin_does_not_read_says_so_once_a_skin_draws_one_of_its_screens() {
        let seven_keys_only = |screen: i32| screen == SKIN_TYPE_PLAY_7KEYS;
        assert_eq!(skinned_note(SettingId::JudgeTextY, seven_keys_only), Some(BUILT_IN_ONLY_NOTE));
        assert_eq!(skinned_note(SettingId::WhiteNumber, seven_keys_only), Some(BUILT_IN_ONLY_NOTE));
        assert_eq!(skinned_note(SettingId::Skin, seven_keys_only), Some(BUILT_IN_ONLY_NOTE));
        assert_eq!(skinned_note(SettingId::ScratchSide, seven_keys_only), Some(SKIN_OPTION_NOTE), "the skin's own option is what its screen follows");
        assert_eq!(skinned_note(SettingId::FiveKeyLayout, seven_keys_only), None, "a seven-key skin leaves the five-key field built in");
        assert_eq!(skinned_note(SettingId::ResultGraphs, seven_keys_only), None, "the result screen is still the built-in one");

        let result_only = |screen: i32| screen == SKIN_TYPE_RESULT;
        assert_eq!(skinned_note(SettingId::ResultGraphs, result_only), Some(BUILT_IN_ONLY_NOTE));
        assert_eq!(skinned_note(SettingId::JudgeTextY, result_only), None);

        let nine_keys_only = |screen: i32| screen == SKIN_TYPE_PLAY_9KEYS;
        assert_eq!(skinned_note(SettingId::ScratchSide, nine_keys_only), None, "a nine-key chart has no scratch lane to place");
        assert_eq!(skinned_note(SettingId::WhiteNumber, nine_keys_only), Some(BUILT_IN_ONLY_NOTE));
    }

    #[test]
    fn a_row_every_screen_reads_is_never_annotated() {
        let everything = |_: i32| true;
        for id in
            [SettingId::HiSpeed, SettingId::LegacyNote, SettingId::Lift, SettingId::LaneCover, SettingId::Bga, SettingId::ScoreGraph, SettingId::Letterbox]
        {
            assert_eq!(skinned_note(id, everything), None, "{id:?} is read on a skin's screen too");
        }
    }

    /// A name is one tab of the strip, or one heading of the play menu: no two groups that are
    /// listed together share one. A group of the play menu may be called what a tab of the settings
    /// screen is, since no screen lists both.
    #[test]
    fn every_group_has_its_own_name_among_the_groups_it_is_listed_with() {
        let listed_together: [&[SettingTab]; 3] = [&SettingTab::SHOWN, &SettingTab::PLAY_MENU, &[SettingTab::Skin]];
        for groups in listed_together {
            let mut names: Vec<&str> = groups.iter().map(|tab| tab.label()).collect();
            let count = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), count, "{groups:?}");
        }
        assert_eq!(
            SettingTab::ALL.map(SettingTab::label).to_vec(),
            vec!["SPEED", "LANE", "JUDGE", "CHART", "JUDGE", "DISPLAY", "SKIN", "INPUT", "NETWORK", "AUDIO", "SELECT"]
        );
    }

    /// The settings screen shows every group but the ones the play menu and the skin menu list, in
    /// the order of the whole list, so a group added to one is not forgotten in the other.
    #[test]
    fn the_shown_tabs_are_every_tab_but_the_two_menus_in_the_same_order() {
        let expected: Vec<SettingTab> = SettingTab::ALL.into_iter().filter(|tab| *tab != SettingTab::Skin && !SettingTab::PLAY_MENU.contains(tab)).collect();
        assert_eq!(SettingTab::SHOWN.to_vec(), expected);
        assert_eq!(SettingTab::SHOWN.map(SettingTab::label).to_vec(), vec!["JUDGE", "DISPLAY", "INPUT", "NETWORK", "AUDIO", "SELECT"]);
    }

    /// The play menu lists four groups, which are the first four of the whole list, and every
    /// group is listed by exactly one screen.
    #[test]
    fn the_play_menus_groups_are_its_own_and_every_group_has_one_screen() {
        assert_eq!(SettingTab::PLAY_MENU.to_vec(), SettingTab::ALL[..SettingTab::PLAY_MENU.len()].to_vec());
        assert_eq!(SettingTab::PLAY_MENU.map(SettingTab::label).to_vec(), vec!["SPEED", "LANE", "JUDGE", "CHART"]);
        for tab in SettingTab::ALL {
            let screens = [SettingTab::SHOWN.contains(&tab), SettingTab::PLAY_MENU.contains(&tab), tab == SettingTab::Skin];
            assert_eq!(screens.iter().filter(|listed| **listed).count(), 1, "{tab:?}");
        }
    }

    /// The rows a player changes from one sitting to the next are the play menu's, and the rules a
    /// press is judged by and the layout of the built-in field stay on the settings screen.
    #[test]
    fn the_rows_of_a_sitting_are_the_play_menus_and_the_rules_are_the_settings_screens() {
        for id in [SettingId::HiSpeed, SettingId::GreenNumber, SettingId::LaneCover, SettingId::JudgeOffset, SettingId::Gauge, SettingId::ScratchAuto] {
            assert!(SettingTab::PLAY_MENU.contains(&tab_of(id)), "{id:?} is on {:?}", tab_of(id));
        }
        assert_eq!(tab_of(SettingId::Total), SettingTab::Judge);
        assert_eq!(tab_of(SettingId::JudgeAlgorithm), SettingTab::Judge);
        assert_eq!(tab_of(SettingId::FiveKeyLayout), SettingTab::Display);
        assert_eq!(tab_of(SettingId::WhiteNumber), SettingTab::Display);
        assert_eq!(tab_of(SettingId::ScratchSide), SettingTab::Input);
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
            SettingId::GreenNumber => f64::from(green_of_duration(config.play.duration_ms)),
            SettingId::LongNoteMargin => f64::from(config.judge.longnote_margin_rate),
            SettingId::TargetRate => f64::from(config.judge.target_rate_percent),
            _ if judge_width_slot(id).is_some() => f64::from(judge_width(config, id).unwrap_or_default()),
            SettingId::AudioPolyphony => config.audio.polyphony as f64,
            SettingId::Lift => f64::from(config.play.lift),
            SettingId::LaneCover => f64::from(config.play.cover),
            SettingId::Hidden => f64::from(config.play.hidden),
            SettingId::LaneCoverFineStep => f64::from(config.play.lanecover_step_fine),
            SettingId::JudgeTextY => f64::from(config.display.judge_text_y),
            SettingId::PreviewVolume => f64::from(config.library.preview_volume),
            SettingId::PreviewFade => f64::from(config.library.preview_fade_ms),
            SettingId::KeyDebounce => f64::from(config.play.key_debounce_ms),
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
            if descriptor(id).host_value {
                continue;
            }
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

    /// A share of the lane stepped a hundredth at a time stands on the hundredths: a hundred steps up
    /// from nothing pass through every whole percent, and each reads as ten times it in thousandths,
    /// which is what a skin shows the lane cover, the lift and the hidden cover as.
    #[test]
    fn a_share_stepped_from_nothing_stands_on_every_whole_percent() {
        const THOUSANDTHS: f32 = 1000.0;
        for id in [SettingId::LaneCover, SettingId::Lift, SettingId::Hidden] {
            for wide in [1, 10] {
                let mut config = Config::default();
                for _ in 0..STRESS_STEPS {
                    adjust(&mut config, id, -1);
                }
                for percent in (wide..=100).step_by(wide as usize) {
                    assert_eq!(adjust(&mut config, id, wide), AdjustOutcome::Changed, "{id:?} stopped short of {percent}%");
                    let share = ranged_value(&config, id) as f32;
                    assert_eq!((share * THOUSANDTHS) as i32, percent * 10, "{id:?} at {percent}% in steps of {wide} holds {share}");
                    assert_eq!(display_value(&config, id), format!("{percent}.0%"), "a shade is shown to the tenth it is counted in");
                }
            }
        }
    }

    #[test]
    fn a_ranged_row_at_the_end_of_its_range_reports_no_change() {
        for id in SettingId::ALL {
            if descriptor(id).host_value
                || !matches!(descriptor(id).kind, SettingKind::IntRange { .. } | SettingKind::FloatRange { .. } | SettingKind::Percent { .. })
            {
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
        assert_eq!(owned, vec![SettingId::PadDebounce, SettingId::Skin, SettingId::SkinPack, SettingId::SkinInfo, SettingId::Account, SettingId::Password]);
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
                SettingId::PadDebounce,
                SettingId::Skin,
                SettingId::SkinPack,
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
                SettingId::SyncRecords,
                SettingId::Rivals,
                SettingId::IrProfiles,
                SettingId::AudioDevice,
                SettingId::SoundFolder,
                SettingId::DownloadFolder,
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
            SettingId::CourseGauge,
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
    fn the_course_gauge_row_cycles_lr2_beatoraja_iidx() {
        let mut config = Config::default();
        assert_eq!(cycle_values(SettingId::CourseGauge), ["LR2", "BEATORAJA", "IIDX"]);
        assert_eq!(config.judge.course_gauge, CourseGaugeMode::Lr2);
        assert_eq!(adjust(&mut config, SettingId::CourseGauge, 1), AdjustOutcome::Changed);
        assert_eq!((config.judge.course_gauge, display_value(&config, SettingId::CourseGauge)), (CourseGaugeMode::Beatoraja, "BEATORAJA".to_string()));
        assert_eq!(adjust(&mut config, SettingId::CourseGauge, 1), AdjustOutcome::Changed);
        assert_eq!((config.judge.course_gauge, display_value(&config, SettingId::CourseGauge)), (CourseGaugeMode::Iidx, "IIDX".to_string()));
        assert_eq!(adjust(&mut config, SettingId::CourseGauge, 1), AdjustOutcome::Changed, "the list wraps at its end");
        assert_eq!(config.judge.course_gauge, CourseGaugeMode::Lr2);
        assert_eq!(adjust(&mut config, SettingId::CourseGauge, -1), AdjustOutcome::Changed, "the list wraps at its start");
        assert_eq!(config.judge.course_gauge, CourseGaugeMode::Iidx);
        assert_eq!(adjust(&mut config, SettingId::CourseGauge, -1), AdjustOutcome::Changed);
        assert_eq!(config.judge.course_gauge, CourseGaugeMode::Beatoraja);
        assert_eq!(config.judge.gauge_set, None, "the single-song gauge table is a row of its own");
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
    fn the_window_rows_show_the_stored_choice_and_step_through_every_one() {
        let mut config = Config::default();
        assert_eq!(display_value(&config, SettingId::WindowResolution), "1280x720");
        assert_eq!(display_value(&config, SettingId::WindowMode), "WINDOWED");
        for expected in &WINDOW_RESOLUTION_LABELS[1..] {
            assert_eq!(adjust(&mut config, SettingId::WindowResolution, 1), AdjustOutcome::Changed);
            assert_eq!(display_value(&config, SettingId::WindowResolution), *expected);
        }
        assert_eq!(adjust(&mut config, SettingId::WindowResolution, 1), AdjustOutcome::Changed, "the list wraps");
        assert_eq!(display_value(&config, SettingId::WindowResolution), "1280x720");
        adjust(&mut config, SettingId::WindowResolution, -1);
        assert_eq!(config.display.window_resolution, WindowResolution::UltraHd2160);
        assert_eq!(adjust(&mut config, SettingId::WindowMode, 1), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::WindowMode), "BORDERLESS");
        assert_eq!(cycle_values(SettingId::WindowMode), WINDOW_MODE_LABELS);
    }

    /// RANKING KEYS ships on, is on for a file written before the row, and is switched like any
    /// other switch of the SELECT group.
    #[test]
    fn the_ranking_keys_row_is_on_as_shipped_and_for_a_file_from_before_it() {
        let mut config = Config::default();
        assert_eq!(display_value(&config, SettingId::RankingKeys), "ON");
        assert_eq!(adjust(&mut config, SettingId::RankingKeys, 1), AdjustOutcome::Changed);
        assert!(!config.library.ranking_keys);
        assert_eq!(display_value(&config, SettingId::RankingKeys), "OFF");
        let before: crate::schema::LibraryOptions = ron::from_str("(show_missing_charts: false)").expect("a file from before the row reads");
        assert!(before.ranking_keys, "a file that never switched the keys off keeps them on");
        assert_eq!(descriptor(SettingId::RankingKeys).tab, SettingTab::Select);
    }

    #[test]
    fn the_language_row_names_each_language_in_itself_and_steps_through_every_one() {
        let mut config = Config::default();
        assert_eq!(display_value(&config, SettingId::Language), "AUTO");
        assert_eq!(cycle_values(SettingId::Language), LANGUAGE_LABELS);
        for expected in &LANGUAGE_LABELS[1..] {
            assert_eq!(adjust(&mut config, SettingId::Language, 1), AdjustOutcome::Changed);
            assert_eq!(display_value(&config, SettingId::Language), *expected);
        }
        assert_eq!(config.display.language, LanguageChoice::Japanese);
        assert_eq!(adjust(&mut config, SettingId::Language, 1), AdjustOutcome::Changed, "the list wraps");
        assert_eq!(config.display.language, LanguageChoice::Auto);
        adjust(&mut config, SettingId::Language, -1);
        assert_eq!(display_value(&config, SettingId::Language), "日本語");
        assert_eq!(tab_rows(SettingTab::Display, &config).first(), Some(&SettingId::Language), "the row heads its tab");
    }

    #[test]
    fn the_skin_row_steps_between_the_bundled_skins() {
        let mut config = Config::default();
        assert_eq!(display_value(&config, SettingId::Skin), "NORMAL");
        step_skin(&mut config);
        assert_eq!(display_value(&config, SettingId::Skin), "WIDE");
        step_skin(&mut config);
        assert_eq!(display_value(&config, SettingId::Skin), "NORMAL");
        assert_eq!(cycle_values(SettingId::Skin), SKIN_LABELS);
    }

    #[test]
    fn the_default_document_reads_as_the_shipped_settings() {
        let config = Config::default();
        assert_eq!(display_value(&config, SettingId::Autoplay), ON_VALUE);
        assert_eq!(display_value(&config, SettingId::HiSpeed), "2.00");
        assert_eq!(display_value(&config, SettingId::HiSpeedStep), "0.25");
        assert_eq!(display_value(&config, SettingId::SpeedFix), "FLOATING");
        assert_eq!(display_value(&config, SettingId::FixHiSpeed), "MAIN", "PlayConfig.java:43-49 ships MAINBPM");
        assert_eq!(display_value(&config, SettingId::KeepGreenNumber), OFF_VALUE, "a file that never asked for it scrolls as it always did");
        assert_eq!(display_value(&config, SettingId::GreenNumber), "300", "PlayConfig.java:24 ships a duration of 500");
        assert_eq!(display_value(&config, SettingId::LaneOption), "OFF");
        assert_eq!(display_value(&config, SettingId::PlayEscapeMode), "IMMEDIATE", "escaping a run stays the one press it has always been");
        assert_eq!(display_value(&config, SettingId::Hidden), "0.0%");
        assert_eq!(display_value(&config, SettingId::EnableCover), ON_VALUE, "PlayConfig.java:66 ships the cover applied");
        assert_eq!(display_value(&config, SettingId::EnableLift), OFF_VALUE, "PlayConfig.java:74");
        assert_eq!(display_value(&config, SettingId::EnableHidden), OFF_VALUE, "PlayConfig.java:82");
        assert_eq!(display_value(&config, SettingId::Sort), "DEFAULT");
        assert_eq!(display_value(&config, SettingId::PreviewVolume), "85%");
        assert_eq!(display_value(&config, SettingId::PreviewFade), "200 MS");
        assert_eq!(display_value(&config, SettingId::KeyDebounce), "16 MS");
        assert_eq!(display_value(&config, SettingId::PadDebounce), "16 MS", "what the key config file holds is the program's to read");
        assert_eq!(display_value(&config, SettingId::Random), "OFF");
        assert_eq!(display_value(&config, SettingId::Gauge), "NORMAL");
        assert_eq!(display_value(&config, SettingId::Lift), "0.0%");
        assert_eq!(display_value(&config, SettingId::ScratchSide), "RIGHT");
        assert_eq!(display_value(&config, SettingId::JudgeOffset), "+0 MS");
        assert_eq!(display_value(&config, SettingId::JudgeWidthKeyPGreat), "100%");
        assert_eq!(display_value(&config, SettingId::JudgeWidthScratchGood), "100%");
        assert_eq!(display_value(&config, SettingId::LongNoteMargin), "100%");
        assert_eq!(display_value(&config, SettingId::JudgeAlgorithm), "COMBO", "JudgeAlgorithm.java:42 lists Combo first, so it is the shipped default");
        assert_eq!(display_value(&config, SettingId::LnMode), "LN");
        assert_eq!(display_value(&config, SettingId::GaugeSet), AUTO_VALUE);
        assert_eq!(display_value(&config, SettingId::CourseGauge), "LR2");
        assert_eq!(display_value(&config, SettingId::GaugeAutoShift), "NONE");
        assert_eq!(display_value(&config, SettingId::BottomShiftableGauge), "ASSIST EASY");
        assert_eq!(display_value(&config, SettingId::Target), "RATE AAA", "the shipped target is the rate the spec names");
        assert_eq!(display_value(&config, SettingId::TargetFallback), "RATE AAA", "the shipped fallback is the shipped target");
        assert_eq!(display_value(&config, SettingId::TargetRate), "85%");
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
    fn the_green_number_row_steps_the_number_it_shows_and_stores_the_travel_time() {
        let mut config = Config::default();
        assert_eq!(adjust(&mut config, SettingId::GreenNumber, 1), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::GreenNumber), "301");
        assert_eq!(config.play.duration_ms, 502, "the shortest travel time that reads as 301");
        assert_eq!(adjust(&mut config, SettingId::GreenNumber, -1), AdjustOutcome::Changed);
        assert_eq!(config.play.duration_ms, crate::schema::DEFAULT_DURATION_MS);
        config.play.duration_ms = crate::schema::DURATION_MAX_MS;
        assert_eq!(adjust(&mut config, SettingId::GreenNumber, 1), AdjustOutcome::Unchanged, "the row stops at its end");
        assert!((config.play.hispeed - crate::schema::DEFAULT_HISPEED).abs() < f64::EPSILON, "the row moves no speed by itself");
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
            tab_rows(SettingTab::PlaySpeed, &config),
            vec![SettingId::KeepGreenNumber, SettingId::GreenNumber, SettingId::HiSpeed, SettingId::HiSpeedStep, SettingId::FixHiSpeed, SettingId::SpeedFix,]
        );
        assert_eq!(
            tab_rows(SettingTab::PlayLane, &config),
            vec![
                SettingId::EnableCover,
                SettingId::LaneCover,
                SettingId::EnableLift,
                SettingId::Lift,
                SettingId::EnableHidden,
                SettingId::Hidden,
                SettingId::LaneCoverFineStep,
            ]
        );
        assert_eq!(tab_rows(SettingTab::PlayJudge, &config), vec![SettingId::JudgeOffset, SettingId::AutoCal]);
        assert_eq!(
            tab_rows(SettingTab::PlayChart, &config),
            vec![
                SettingId::Random,
                SettingId::Gauge,
                SettingId::LegacyNote,
                SettingId::Target,
                SettingId::TargetFallback,
                SettingId::TargetRate,
                SettingId::Autoplay,
                SettingId::ScratchAuto,
                SettingId::AutoReplay,
                SettingId::PlayEscapeMode,
            ]
        );
        assert_eq!(
            tab_rows(SettingTab::Judge, &config),
            vec![
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
                SettingId::CourseGauge,
                SettingId::GaugeAutoShift,
                SettingId::BottomShiftableGauge,
                SettingId::Total,
            ]
        );
        assert_eq!(
            tab_rows(SettingTab::Display, &config),
            vec![
                SettingId::Language,
                SettingId::Skin,
                SettingId::Font,
                SettingId::ScoreGraph,
                SettingId::ResultGraphs,
                SettingId::ReplayAnalysis,
                SettingId::Preview,
                SettingId::WhiteNumber,
                SettingId::JudgeTextY,
                SettingId::FiveKeyLayout,
                SettingId::Letterbox,
                SettingId::WindowResolution,
                SettingId::WindowMode,
                SettingId::Bga,
                SettingId::DebugMode,
            ]
        );
        assert_eq!(
            tab_rows(SettingTab::Select, &config),
            vec![
                SettingId::Sort,
                SettingId::FavoriteOnly,
                SettingId::ShowMissingCharts,
                SettingId::RankingKeys,
                SettingId::PreviewVolume,
                SettingId::PreviewFade,
                SettingId::DownloadFolder
            ]
        );
        assert_eq!(tab_rows(SettingTab::Input, &config), vec![SettingId::ScratchSide, SettingId::KeyConfig, SettingId::KeyDebounce, SettingId::PadDebounce]);
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
                SettingId::SyncRecords,
                SettingId::AutoUploadReplay,
                SettingId::AutoUploadTrace,
                SettingId::Rivals,
                SettingId::IrProfiles,
                SettingId::DiscordStatus,
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
                SettingId::SoundFolder,
                SettingId::GuideSe,
            ]
        );
        for tab in SettingTab::ALL {
            for id in tab_rows(tab, &config) {
                assert_eq!(tab_of(id), tab);
            }
        }
    }

    /// Every row that holds a number, with the smallest change it can hold as it is shown and the
    /// two ends of the row as they are shown.
    const NUMBER_ROWS: [(SettingId, &str, &str, &str); 26] = [
        (SettingId::GreenNumber, "1", "1", "6000"),
        (SettingId::HiSpeed, "0.01", "0.01", "20.00"),
        (SettingId::HiSpeedStep, "0.01", "0.01", "1.00"),
        (SettingId::LaneCover, "0.1%", "0.0%", "100.0%"),
        (SettingId::Lift, "0.1%", "0.0%", "100.0%"),
        (SettingId::Hidden, "0.1%", "0.0%", "100.0%"),
        (SettingId::LaneCoverFineStep, "0.0001", "0.0001", "0.0100"),
        (SettingId::JudgeOffset, "+1 MS", "-200 MS", "+200 MS"),
        (SettingId::JudgeWidthKeyPGreat, "1%", "50%", "200%"),
        (SettingId::JudgeWidthKeyGreat, "1%", "50%", "200%"),
        (SettingId::JudgeWidthKeyGood, "1%", "50%", "200%"),
        (SettingId::JudgeWidthScratchPGreat, "1%", "50%", "200%"),
        (SettingId::JudgeWidthScratchGreat, "1%", "50%", "200%"),
        (SettingId::JudgeWidthScratchGood, "1%", "50%", "200%"),
        (SettingId::LongNoteMargin, "1%", "50%", "200%"),
        (SettingId::TargetRate, "1%", "1%", "100%"),
        (SettingId::Total, "1", "0", "5000"),
        (SettingId::JudgeTextY, "1%", "0%", "100%"),
        (SettingId::PreviewVolume, "1%", "0%", "100%"),
        (SettingId::PreviewFade, "1 MS", "0 MS", "1000 MS"),
        (SettingId::KeyDebounce, "1 MS", "0 MS", "100 MS"),
        (SettingId::PadDebounce, "1 MS", "0 MS", "100 MS"),
        (SettingId::MasterVolume, "1%", "0%", "100%"),
        (SettingId::KeyVolume, "1%", "0%", "100%"),
        (SettingId::BgmVolume, "1%", "0%", "100%"),
        (SettingId::SystemVolume, "1%", "0%", "100%"),
    ];

    /// The one row that holds a number and is not in [`NUMBER_ROWS`]: its range is a count of
    /// voices, checked on its own.
    const VOICES_ROW: SettingId = SettingId::AudioPolyphony;

    #[test]
    fn the_rows_that_hold_a_number_are_the_rows_of_a_range_and_no_other() {
        for row in SETTINGS {
            let ranged = matches!(row.kind, SettingKind::IntRange { .. } | SettingKind::FloatRange { .. } | SettingKind::Percent { .. });
            assert_eq!(number_spec(row.id).is_some(), ranged, "{:?} is a {:?}", row.id, row.kind);
            assert_eq!(number_steps(&Config::default(), row.id).is_some(), ranged && !row.host_value, "{:?}", row.id);
        }
        let listed: Vec<SettingId> = NUMBER_ROWS.iter().map(|(id, ..)| *id).chain([VOICES_ROW]).collect();
        for row in SETTINGS {
            assert_eq!(listed.contains(&row.id), number_spec(row.id).is_some(), "{:?} is not checked row by row", row.id);
        }
    }

    #[test]
    fn every_number_row_is_counted_in_its_smallest_change_between_its_two_ends() {
        for (id, smallest, low, high) in NUMBER_ROWS {
            let spec = number_spec(id).expect("the row holds a number");
            assert_eq!(spec.text(spec.min), low, "{id:?}");
            assert_eq!(spec.text(spec.max), high, "{id:?}");
            assert_eq!(spec.text(1), smallest, "{id:?}");
        }
        let voices = number_spec(VOICES_ROW).expect("the row holds a number");
        assert_eq!((voices.min, voices.max, voices.per_one), (AUDIO_POLYPHONY_MIN_VOICES as i64, AUDIO_POLYPHONY_MAX_VOICES as i64, 1));
    }

    /// Every step of every row can be stored and reads back as itself, and what the row shows for
    /// it is the step: nothing is lost between the count and the field it is stored in.
    #[test]
    fn every_step_of_every_number_row_is_stored_and_read_back() {
        for id in NUMBER_ROWS.iter().map(|(id, ..)| *id).chain([VOICES_ROW]).filter(|id| !descriptor(*id).host_value) {
            let spec = number_spec(id).expect("the row holds a number");
            let mut config = Config::default();
            for steps in spec.min..=spec.max {
                set_number_steps(&mut config, id, steps);
                assert_eq!(number_steps(&config, id), Some(steps), "{id:?}");
            }
            for steps in (spec.min..=spec.max).rev() {
                set_number_steps(&mut config, id, steps);
                assert_eq!(number_steps(&config, id), Some(steps), "{id:?} on the way down");
            }
        }
    }

    /// A row moved one step at a time stands on every step on the way and shows it, however many
    /// steps it is moved: the value stored is a count divided, not a sum of steps.
    #[test]
    fn a_row_moved_a_step_at_a_time_never_drifts_off_its_steps() {
        let mut config = Config::default();
        set_number_steps(&mut config, SettingId::LaneCover, 0);
        for steps in 1..=1000 {
            assert_eq!(move_number_steps(&mut config, SettingId::LaneCover, 1), AdjustOutcome::Changed);
            assert_eq!(display_value(&config, SettingId::LaneCover), format!("{}.{}%", steps / 10, steps % 10));
            assert_eq!((config.play.cover * 1000.0) as i64, steps, "the white number a skin cuts from a cover of {steps} tenths of a percent");
        }
        assert_eq!(move_number_steps(&mut config, SettingId::LaneCover, 1), AdjustOutcome::Unchanged, "the whole lane is the end of the row");

        set_number_steps(&mut config, SettingId::HiSpeed, 1);
        for steps in 2..=2000 {
            move_number_steps(&mut config, SettingId::HiSpeed, 1);
            assert_eq!(display_value(&config, SettingId::HiSpeed), format!("{}.{:02}", steps / 100, steps % 100));
        }
        assert_eq!(move_number_steps(&mut config, SettingId::HiSpeed, 10), AdjustOutcome::Unchanged);
        assert_eq!(move_number_steps(&mut config, SettingId::HiSpeed, -10), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::HiSpeed), "19.90");
    }

    #[test]
    fn a_number_row_stops_at_its_ends_and_is_held_to_them() {
        let mut config = Config::default();
        assert_eq!(set_number_steps(&mut config, SettingId::JudgeOffset, 999), AdjustOutcome::Changed);
        assert_eq!(config.judge.offset_ms, JUDGE_OFFSET_MAX_MS);
        assert_eq!(move_number_steps(&mut config, SettingId::JudgeOffset, 1), AdjustOutcome::Unchanged);
        assert_eq!(set_number_steps(&mut config, SettingId::JudgeOffset, i64::MIN), AdjustOutcome::Changed);
        assert_eq!(config.judge.offset_ms, JUDGE_OFFSET_MIN_MS);
        assert_eq!(set_number_steps(&mut config, SettingId::MasterVolume, 37), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::MasterVolume), "37%");
        assert_eq!(set_number_steps(&mut config, SettingId::AudioPolyphony, 500), AdjustOutcome::Changed);
        assert_eq!(config.audio.polyphony, 500);
        assert!(config.audio.reopen_pending(), "a count of voices reaches the stream only when it is opened again");
        assert_eq!(set_number_steps(&mut config, SettingId::Total, 0), AdjustOutcome::Unchanged);
        assert_eq!(display_value(&config, SettingId::Total), AUTO_VALUE);
        assert_eq!(set_number_steps(&mut config, SettingId::Total, 321), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::Total), "321");
        assert_eq!(set_number_steps(&mut config, SettingId::Autoplay, 1), AdjustOutcome::Unchanged, "a switch holds no number");
        assert_eq!(move_number_steps(&mut config, SettingId::Gauge, 1), AdjustOutcome::Unchanged, "a list holds no number");
    }

    /// A value between two steps is left as it is stored by a row put on the step it already reads
    /// as, and goes onto the steps the first time it is moved.
    #[test]
    fn a_value_between_two_steps_reads_as_the_nearer_and_is_kept_until_it_is_moved() {
        let mut config = Config::default();
        config.play.cover = 0.25349;
        assert_eq!(number_steps(&config, SettingId::LaneCover), Some(253));
        assert_eq!(set_number_steps(&mut config, SettingId::LaneCover, 253), AdjustOutcome::Unchanged);
        assert!((config.play.cover - 0.25349).abs() < f32::EPSILON, "a row put where it already reads was rewritten");
        assert_eq!(move_number_steps(&mut config, SettingId::LaneCover, 1), AdjustOutcome::Changed);
        assert!((config.play.cover - 0.254).abs() < f32::EPSILON);
        assert_eq!(number_steps(&config, SettingId::LaneCover), Some(254));

        config.play.duration_ms = 501;
        assert_eq!(number_steps(&config, SettingId::GreenNumber), Some(300));
        assert_eq!(set_number_steps(&mut config, SettingId::GreenNumber, 300), AdjustOutcome::Unchanged);
        assert_eq!(config.play.duration_ms, 501);

        config.play.total_override = 9000.0;
        assert_eq!(number_steps(&config, SettingId::Total), Some(TOTAL_ROW_MAX), "a total past the row reads as its end");
        assert_eq!(display_value(&config, SettingId::Total), "9000", "and is shown as what is stored");
    }

    /// The step [`adjust`] takes is the option panel's and a skin event's, which the count of steps
    /// leaves as it was.
    #[test]
    fn the_step_of_the_option_panel_is_what_it_was() {
        let mut config = Config::default();
        config.play.hispeed = 2.0;
        config.play.hispeed_step = 0.25;
        adjust(&mut config, SettingId::HiSpeed, 1);
        assert!((config.play.hispeed - 2.25).abs() < 1e-9);
        config.play.cover = 0.0;
        adjust(&mut config, SettingId::LaneCover, 1);
        assert_eq!(display_value(&config, SettingId::LaneCover), "1.0%");
    }
}
