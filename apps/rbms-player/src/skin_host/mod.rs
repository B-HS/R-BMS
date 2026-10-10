//! The host a skin reads the running game through.
//!
//! A skin addresses the game by property id, and [`rbms_skin::property::SkinHost`] is the one place
//! those ids are answered. [`ScreenHost`] is the player's implementation of it. It answers nothing
//! itself: it borrows the state of the screen being drawn, cut into clusters by where a value comes
//! from, and hands each id to the cluster that owns it.
//!
//! Each cluster is a file of its own -- the chart in hand, the live score, the browser, the result
//! and so on -- holding the state that cluster reads and its answers. [`ROUTES`] says which cluster
//! is asked first for an id. An id no cluster knows reads as the absent value of its kind, which is
//! what the host contract asks for.
//!
//! A skin also tells the game things: run an event, move a slider, play a sound. Those arrive while
//! a frame is being drawn, when nothing may change, so they are recorded as they arrive and handed
//! over afterwards ([`ScreenHost::take_calls`]).
//!
//! Every skin screen is drawn through this host. Until each cluster has the state it reads, the
//! screen also hands over the adapter it was drawn from before ([`ScreenHost::fallback`]), and an id
//! no cluster knows is answered by that.

pub mod audio;
pub mod chart;
pub mod ir;
pub mod keyconfig;
pub mod loading;
pub mod options;
pub mod overview;
pub mod play;
pub mod play_timers;
pub mod result;
pub mod score;
pub mod select;
pub mod skin_config;
pub mod system;
pub mod writers;

use std::borrow::Cow;
use std::cell::RefCell;

use rbms_skin::dst::{DrawStateSource, OffsetSource, SkinOffset};
use rbms_skin::property::generated::*;
use rbms_skin::property::{
    AudioCommand, FLOAT_ABSENT, HostCall, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, STRING_KEYNAME_EXTENDED_FIRST, STRING_KEYNAME_EXTENDED_LAST,
    STRING_KEYNAME_FIRST, STRING_KEYNAME_LAST, ScoreSlot, ScoreSnapshot, SkinHost, StaticScreen, TEXT_ABSENT, VolumeBus, normalize_boolean_id, static_scope,
};
use rbms_skin::timer::{TimerId, TimerState};

/// The image index of the browser's difficulty filter (`IndexType.difficulty`), which the reference
/// numbers without declaring a constant for it.
const INDEX_DIFFICULTY_FILTER: i32 = 10;

/// The image indices of the options the target score was played with, first and last
/// (`IndexType.option_target1_1p` to `option_target1_dp`).
const INDEX_TARGET_OPTION_FIRST: i32 = 61;
const INDEX_TARGET_OPTION_LAST: i32 = 63;

/// The image index of the automatic judge timing adjustment
/// (`IndexType.notesdisplaytimingautoadjust`).
const INDEX_TIMING_AUTO_ADJUST: i32 = 75;

/// The image index of the guide sound switch (`IndexType.guidese`), the last of the run that starts
/// at [`BUTTON_JUDGEALGORITHM`].
const INDEX_GUIDE_SOUND: i32 = 343;

/// The image indices of the run's clear type and of the target's (`IndexType.cleartype` and
/// `cleartype_target`).
const INDEX_CLEAR: i32 = 370;
const INDEX_TARGET_CLEAR: i32 = 371;

/// The image index of the constant-speed switch (`IndexType.constant`).
const INDEX_CONSTANT: i32 = 400;

/// What one cluster of properties knows, each read answering `None` for an id it does not.
///
/// Every method has the "not known" answer as its default, so a cluster implements the reads it
/// has values for and nothing else. The ids of each kind are separate spaces, as they are in the
/// host contract: 41 is one thing as an option and another as a number.
pub trait ClusterState {
    /// The option under a positive `OPTION_*` id. The host negates the answer for a negative one.
    fn boolean(&self, _id: i32) -> Option<bool> {
        None
    }

    /// The integer under a `NUMBER_*` id.
    fn integer(&self, _id: i32) -> Option<i32> {
        None
    }

    /// The image index under `id`, which is a space of its own.
    fn image_index(&self, _id: i32) -> Option<i32> {
        None
    }

    /// The share under a `RATE_*`, `SLIDER_*` or `BARGRAPH_*` id.
    fn rate(&self, _id: i32) -> Option<f32> {
        None
    }

    /// The number under a `FLOAT_*` id. The host falls back to [`Self::rate`] on its own.
    fn float(&self, _id: i32) -> Option<f32> {
        None
    }

    /// The text under a `STRING_*` id.
    fn text(&self, _id: i32) -> Option<Cow<'_, str>> {
        None
    }

    /// The offset under an `OFFSET_*` id that the game itself sets, in output pixels.
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }

    /// The groove gauge's value, for `main_state.gauge`.
    fn gauge(&self) -> Option<f32> {
        None
    }

    /// The groove gauge's type, for `main_state.gauge_type`.
    fn gauge_type(&self) -> Option<i32> {
        None
    }

    /// How many times one judgement was given, for `main_state.judge`.
    fn judge(&self, _judge: i32) -> Option<i32> {
        None
    }

    /// One of the three scores `main_state.rate*` and `main_state.exscore*` report.
    fn score(&self, _slot: ScoreSlot) -> Option<ScoreSnapshot> {
        None
    }

    /// One of the three volumes, for `main_state.volume_*`.
    fn volume(&self, _bus: VolumeBus) -> Option<f32> {
        None
    }

    /// Whether the key with this libGDX key code is held.
    fn key_pressed(&self, _code: i32) -> Option<bool> {
        None
    }
}

/// One cluster of properties, named for where its values come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cluster {
    /// The load in progress ([`loading`]).
    Loading,
    /// The result of a finished run ([`result`]).
    Result,
    /// The song browser ([`select`]).
    Select,
    /// The judgement on show and the lanes ([`play`]).
    Play,
    /// The score of the run in progress ([`score`]).
    Score,
    /// The chart in hand ([`chart`]).
    Chart,
    /// The player's settings ([`options`]).
    Options,
    /// The internet ranking and rivals ([`ir`]).
    Ir,
    /// The clock, the player's totals and the machine ([`system`]).
    System,
    /// The skin settings screen ([`skin_config`]).
    SkinConfig,
    /// The key configuration screen ([`keyconfig`]).
    KeyConfig,
}

impl Cluster {
    /// Every cluster, in the order they are asked when more than one could answer an id: what only
    /// one screen knows before what every screen knows, so the result screen's own answer for a
    /// score wins over the live one, and the browser's over the chart's.
    pub const ALL: [Cluster; 11] = [
        Cluster::Loading,
        Cluster::Result,
        Cluster::Select,
        Cluster::Play,
        Cluster::Score,
        Cluster::Chart,
        Cluster::Options,
        Cluster::Ir,
        Cluster::System,
        Cluster::SkinConfig,
        Cluster::KeyConfig,
    ];

    /// This cluster's place in a set of clusters.
    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

/// Which of the host contract's id spaces an id is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdSpace {
    Boolean,
    Integer,
    ImageIndex,
    Rate,
    Float,
    Text,
    Offset,
}

/// One run of ids in one space that a cluster answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub space: IdSpace,
    /// The lowest id of the run, inclusive.
    pub first: i32,
    /// The highest id of the run, inclusive.
    pub last: i32,
    pub cluster: Cluster,
}

impl Route {
    /// A run from its endpoints, both inclusive.
    const fn band(space: IdSpace, first: i32, last: i32, cluster: Cluster) -> Route {
        Route { space, first, last, cluster }
    }

    /// A run of a single id.
    const fn one(space: IdSpace, id: i32, cluster: Cluster) -> Route {
        Route::band(space, id, id, cluster)
    }

    /// Whether this run holds `id` of `space`.
    fn covers(&self, space: IdSpace, id: i32) -> bool {
        self.space == space && self.first <= id && id <= self.last
    }
}

/// Which cluster answers which ids, grouped by cluster.
///
/// The runs follow the reference's own grouping of where a value comes from. Two clusters may claim
/// the same run -- a score read on the result screen and during play -- and both are then asked,
/// in the order of [`Cluster::ALL`]. An id outside every run is offered to every cluster in that
/// same order, so a cluster can answer an id this table has not caught up with.
pub const ROUTES: &[Route] = &[
    Route::band(IdSpace::Integer, NUMBER_FOLDER_BEGINNER, NUMBER_FOLDER_INSANE, Cluster::Chart),
    Route::band(IdSpace::Integer, NUMBER_MAXBPM, NUMBER_MAINBPM, Cluster::Chart),
    Route::one(IdSpace::Integer, NUMBER_PLAYLEVEL, Cluster::Chart),
    Route::one(IdSpace::Integer, NUMBER_TOTALNOTES, Cluster::Chart),
    Route::one(IdSpace::Integer, NUMBER_TOTALNOTES2, Cluster::Chart),
    Route::band(IdSpace::Integer, NUMBER_TOTALNOTE_NORMAL, NUMBER_TOTALNOTE_BSS, Cluster::Chart),
    Route::band(IdSpace::Integer, NUMBER_DENSITY_PEAK, NUMBER_DENSITY_AVERAGE_AFTERDOT, Cluster::Chart),
    Route::one(IdSpace::Integer, NUMBER_SONGGAUGE_TOTAL, Cluster::Chart),
    Route::one(IdSpace::Integer, NUMBER_JUDGERANK, Cluster::Chart),
    Route::band(IdSpace::Integer, NUMBER_SONGLENGTH_MINUTE, NUMBER_SONGLENGTH_SECOND, Cluster::Chart),
    Route::one(IdSpace::Float, FLOAT_CHART_PEAKDENSITY, Cluster::Chart),
    Route::one(IdSpace::Float, FLOAT_CHART_ENDDENSITY, Cluster::Chart),
    Route::one(IdSpace::Float, FLOAT_CHART_AVERAGEDENSITY, Cluster::Chart),
    Route::one(IdSpace::Float, FLOAT_CHART_TOTALGAUGE, Cluster::Chart),
    Route::band(IdSpace::Boolean, OPTION_DIFFICULTY0, OPTION_DIFFICULTY5, Cluster::Chart),
    Route::band(IdSpace::Boolean, OPTION_7KEYSONG, OPTION_9KEYSONG, Cluster::Chart),
    Route::band(IdSpace::Boolean, OPTION_NO_BGA, OPTION_JUDGE_VERYEASY, Cluster::Chart),
    Route::band(IdSpace::Boolean, OPTION_NO_STAGEFILE, OPTION_BACKBMP, Cluster::Chart),
    Route::one(IdSpace::Boolean, OPTION_TABLE_SONG, Cluster::Chart),
    Route::band(IdSpace::Boolean, OPTION_24KEYSONG, OPTION_24KEYDPSONG, Cluster::Chart),
    Route::one(IdSpace::Boolean, OPTION_BPMSTOP, Cluster::Chart),
    Route::band(IdSpace::Text, STRING_TITLE, STRING_FULLARTIST, Cluster::Chart),
    Route::band(IdSpace::Text, STRING_TABLE_NAME, STRING_TABLE_FULL, Cluster::Chart),
    Route::band(IdSpace::Text, STRING_SONG_HASH_MD5, STRING_SONG_HASH_SHA256, Cluster::Chart),
    Route::band(IdSpace::Integer, NUMBER_SCORE, NUMBER_MAXSCORE, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_TOTALNOTES, NUMBER_MAXCOMBO, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_POINT, NUMBER_DIFF_EXSCORE, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_TOTAL_RATE, NUMBER_TOTAL_RATE_AFTERDOT, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_TARGET_SCORE, NUMBER_TARGET_SCORE_RATE_AFTERDOT, Cluster::Score),
    Route::one(IdSpace::Integer, NUMBER_DIFF_EXSCORE2, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_TARGET_TOTAL_RATE, NUMBER_TARGET_TOTAL_RATE_AFTERDOT, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_HIGHSCORE, NUMBER_TARGET_SCORE_RATE_AFTERDOT2, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_HIGHSCORE2, NUMBER_DIFF_MISSCOUNT, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_BEST_RATE, NUMBER_BEST_RATE_AFTERDOT, Cluster::Score),
    Route::band(IdSpace::Integer, NUMBER_EARLY_PERFECT, NUMBER_BAD_PLUS_POOR_PLUS_MISS, Cluster::Score),
    Route::band(IdSpace::Float, FLOAT_PERFECT_RATE, FLOAT_POOR_RATE, Cluster::Score),
    Route::one(IdSpace::Float, FLOAT_SCORE_RATE2, Cluster::Score),
    Route::one(IdSpace::Float, FLOAT_TARGET_RATE2, Cluster::Score),
    Route::one(IdSpace::Float, FLOAT_BEST_RATE, Cluster::Score),
    Route::one(IdSpace::Float, FLOAT_SCORE_RATE, Cluster::Score),
    Route::one(IdSpace::Float, FLOAT_TOTAL_RATE, Cluster::Score),
    Route::band(IdSpace::Rate, RATE_SCORE, RATE_TARGETSCORE, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_1P_AAA, OPTION_1P_F, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_AAA, OPTION_F, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_1P_0_9, OPTION_1P_100, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_RESULT_AAA_1P, OPTION_RESULT_F_1P, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_BEST_AAA_1P, OPTION_BEST_F_1P, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_NOW_AAA_1P, OPTION_NOW_F_1P, Cluster::Score),
    Route::band(IdSpace::Boolean, OPTION_PERFECT_EXIST, OPTION_MISS_EXIST, Cluster::Score),
    Route::one(IdSpace::Boolean, OPTION_1P_PERFECT, Cluster::Play),
    Route::one(IdSpace::Boolean, OPTION_2P_PERFECT, Cluster::Play),
    Route::one(IdSpace::Boolean, OPTION_3P_PERFECT, Cluster::Play),
    Route::band(IdSpace::Boolean, OPTION_1P_EARLY, OPTION_1P_LATE, Cluster::Play),
    Route::band(IdSpace::Boolean, OPTION_2P_EARLY, OPTION_2P_LATE, Cluster::Play),
    Route::band(IdSpace::Boolean, OPTION_3P_EARLY, OPTION_3P_LATE, Cluster::Play),
    Route::band(IdSpace::Boolean, OPTION_LANECOVER1_CHANGING, OPTION_HIDDEN1_ON, Cluster::Play),
    Route::one(IdSpace::Integer, NUMBER_LANECOVER1, Cluster::Play),
    Route::band(IdSpace::Integer, NUMBER_LIFT1, NUMBER_LANECOVER2, Cluster::Play),
    Route::band(IdSpace::Integer, VALUE_JUDGE_1P_DURATION, VALUE_JUDGE_3P_DURATION, Cluster::Play),
    Route::band(IdSpace::Integer, NUMBER_DURATION_LANECOVER_ON, NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF, Cluster::Play),
    Route::band(IdSpace::Rate, RATE_LANECOVER, RATE_LANECOVER2, Cluster::Play),
    Route::band(IdSpace::ImageIndex, VALUE_JUDGE_1P_SCRATCH, VALUE_JUDGE_2P_KEY9, Cluster::Play),
    Route::band(IdSpace::ImageIndex, VALUE_JUDGE_1P_KEY10, VALUE_JUDGE_2P_KEY99, Cluster::Play),
    Route::band(IdSpace::Offset, OFFSET_SCRATCHANGLE_1P, OFFSET_HIDDEN_COVER, Cluster::Play),
    Route::band(IdSpace::ImageIndex, INDEX_DIFFICULTY_FILTER, BUTTON_SORT, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_GAUGE_1P, BUTTON_RANDOM_2P, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_DPOPTION, BUTTON_HSFIX, Cluster::Options),
    Route::band(IdSpace::ImageIndex, INDEX_TARGET_OPTION_FIRST, INDEX_TARGET_OPTION_LAST, Cluster::Options),
    Route::one(IdSpace::ImageIndex, BUTTON_BGA, Cluster::Options),
    Route::one(IdSpace::ImageIndex, INDEX_TIMING_AUTO_ADJUST, Cluster::Options),
    Route::one(IdSpace::ImageIndex, BUTTON_GAUGEAUTOSHIFT, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_FAVORITTE_SONG, BUTTON_FAVORITTE_CHART, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_ASSIST_EXJUDGE, BUTTON_LNMODE, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_AUTOSAVEREPLAY_1, BUTTON_AUTOSAVEREPLAY_4, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_LANECOVER, BUTTON_HIDDEN, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_JUDGEALGORITHM, INDEX_GUIDE_SOUND, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_EXTRANOTE, BUTTON_LONGNOTEMODE, Cluster::Options),
    Route::band(IdSpace::ImageIndex, BUTTON_SEVENTONINE_PATTERN, BUTTON_SEVENTONINE_TYPE, Cluster::Options),
    Route::one(IdSpace::ImageIndex, INDEX_CONSTANT, Cluster::Options),
    Route::one(IdSpace::Integer, NUMBER_HISPEED_LR2, Cluster::Options),
    Route::one(IdSpace::Integer, NUMBER_JUDGETIMING, Cluster::Options),
    Route::band(IdSpace::Integer, NUMBER_MASTER_VOLUME, NUMBER_BGM_VOLUME, Cluster::Options),
    Route::band(IdSpace::Integer, NUMBER_HISPEED, NUMBER_DURATION_GREEN, Cluster::Options),
    Route::one(IdSpace::Float, FLOAT_HISPEED, Cluster::Options),
    Route::band(IdSpace::Rate, RATE_MASTERVOLUME, RATE_BGMVOLUME, Cluster::Options),
    Route::band(IdSpace::Boolean, OPTION_DISABLE_SAVE_SCORE, OPTION_NO_SAVE_CLEAR, Cluster::Options),
    Route::band(IdSpace::Boolean, OPTION_FOLDERBAR, OPTION_PLAYABLEBAR, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_PANEL1, OPTION_PANEL3, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_SELECT_BAR_NOT_PLAYED, OPTION_SELECT_BAR_FULL_COMBO_CLEARED, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_IR_NOPLAYER, OPTION_IR_BUSY, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_NOT_COMPARE_RIVAL, OPTION_COMPARE_RIVAL, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_GRADEBAR_CLASS, OPTION_GRADEBAR_HCN, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_RANDOMSELECTBAR, OPTION_RANDOMCOURSEBAR, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_SELECT_BAR_ASSIST_EASY_CLEARED, OPTION_SELECT_BAR_MAX_CLEARED, Cluster::Select),
    Route::band(IdSpace::Boolean, OPTION_SELECT_REPLAYDATA, OPTION_SELECT_REPLAYDATA4, Cluster::Select),
    Route::band(IdSpace::Integer, NUMBER_PLAYCOUNT, NUMBER_FAILCOUNT, Cluster::Select),
    Route::band(IdSpace::Integer, NUMBER_LASTPLAY_TIMESTAMP, NUMBER_LASTPLAY_SECOND, Cluster::Select),
    Route::one(IdSpace::Integer, NUMBER_FOLDER_TOTALSONGS, Cluster::Select),
    Route::band(IdSpace::Integer, NUMBER_FOLDER_NOPLAY, NUMBER_FOLDER_MAX, Cluster::Select),
    Route::one(IdSpace::Rate, RATE_MUSICSELECT_POSITION, Cluster::Select),
    Route::one(IdSpace::Rate, RATE_RANKING_POSITION, Cluster::Select),
    Route::one(IdSpace::Integer, NUMBER_MISSCOUNT, Cluster::Result),
    Route::band(IdSpace::Integer, NUMBER_HIGHSCORE2, NUMBER_DIFF_MISSCOUNT, Cluster::Result),
    Route::band(IdSpace::Integer, NUMBER_CLEAR, NUMBER_STDDEV_TIMING_AFTERDOT, Cluster::Result),
    Route::band(IdSpace::Boolean, OPTION_RESULT_CLEAR, OPTION_RESULT_FAIL, Cluster::Result),
    Route::band(IdSpace::Boolean, OPTION_UPDATE_SCORE, OPTION_UPDATE_TARGET, Cluster::Result),
    Route::band(IdSpace::Boolean, OPTION_1PWIN, OPTION_DRAW, Cluster::Result),
    Route::band(IdSpace::Boolean, OPTION_DRAW_SCORE, OPTION_DRAW_TARGET, Cluster::Result),
    Route::band(IdSpace::ImageIndex, INDEX_CLEAR, INDEX_TARGET_CLEAR, Cluster::Result),
    Route::band(IdSpace::Integer, NUMBER_IR_RANK, NUMBER_IR_PREVRANK, Cluster::Ir),
    Route::band(IdSpace::Integer, NUMBER_IR_TOTALPLAYER2, NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT, Cluster::Ir),
    Route::band(IdSpace::Integer, NUMBER_RIVAL_SCORE, NUMBER_RIVAL_POOR_RATE, Cluster::Ir),
    Route::band(IdSpace::Integer, NUMBER_RANKING1_EXSCORE, NUMBER_RANKING10_CLEAR, Cluster::Ir),
    Route::band(IdSpace::Float, FLOAT_IR_PLAYER_NOPLAY_RATE, FLOAT_IR_TOTALFULLCOMBORATE, Cluster::Ir),
    Route::band(IdSpace::Float, FLOAT_RIVAL_PERFECT_RATE, FLOAT_RIVAL_POOR_RATE, Cluster::Ir),
    Route::band(IdSpace::Boolean, OPTION_OFFLINE, OPTION_ONLINE, Cluster::Ir),
    Route::band(IdSpace::Boolean, OPTION_IR_LOADING, OPTION_IR_BUSY, Cluster::Ir),
    Route::one(IdSpace::Text, STRING_RIVAL, Cluster::Ir),
    Route::band(IdSpace::Text, STRING_RANKING1_NAME, STRING_RANKING10_NAME, Cluster::Ir),
    Route::band(IdSpace::Text, STRING_IR_NAME, STRING_IR_USER_NAME, Cluster::Ir),
    Route::band(IdSpace::Integer, NUMBER_TOTALPLAYTIME_HOUR, NUMBER_TOTALPOOR, Cluster::System),
    Route::band(IdSpace::Integer, NUMBER_MASTER_VOLUME, NUMBER_BGM_VOLUME, Cluster::System),
    Route::one(IdSpace::Integer, NUMBER_TOTALPLAYNOTES, Cluster::System),
    Route::band(IdSpace::Rate, RATE_MASTERVOLUME, RATE_BGMVOLUME, Cluster::System),
    Route::band(IdSpace::Boolean, OPTION_COURSE_STAGE1, OPTION_COURSE_STAGE4, Cluster::System),
    Route::band(IdSpace::Boolean, OPTION_COURSE_STAGE_FINAL, OPTION_MODE_COURSE, Cluster::System),
    Route::one(IdSpace::Text, STRING_PLAYER, Cluster::System),
    Route::one(IdSpace::Text, STRING_VERSION, Cluster::System),
    Route::band(IdSpace::Text, STRING_SKIN_NAME, STRING_SKIN_AUTHOR, Cluster::SkinConfig),
    Route::band(IdSpace::Text, STRING_SKIN_CUSTOMIZE_CATEGORY1, STRING_SKIN_CUSTOMIZE_ITEM10, Cluster::SkinConfig),
    Route::one(IdSpace::Rate, RATE_SKINSELECT_POSITION, Cluster::SkinConfig),
    Route::band(IdSpace::ImageIndex, BUTTON_SKINSELECT_7KEY, BUTTON_SKINSELECT_COURSE_RESULT, Cluster::SkinConfig),
    Route::band(IdSpace::ImageIndex, BUTTON_SKINSELECT_24KEY, BUTTON_SKINSELECT_24KEY_BATTLE, Cluster::SkinConfig),
    Route::band(IdSpace::Text, STRING_KEYNAME_FIRST, STRING_KEYNAME_LAST, Cluster::KeyConfig),
    Route::band(IdSpace::Text, STRING_KEYNAME_EXTENDED_FIRST, STRING_KEYNAME_EXTENDED_LAST, Cluster::KeyConfig),
    Route::one(IdSpace::Integer, NUMBER_LOADING_PROGRESS, Cluster::Loading),
    Route::one(IdSpace::Float, FLOAT_LOADING_PROGRESS, Cluster::Loading),
    Route::one(IdSpace::Rate, RATE_LOAD_PROGRESS, Cluster::Loading),
    Route::band(IdSpace::Boolean, OPTION_NOW_LOADING, OPTION_LOADED, Cluster::Loading),
];

/// The clusters [`ROUTES`] sends an id of one space to, in the order they are asked.
pub fn clusters_of(space: IdSpace, id: i32) -> impl Iterator<Item = Cluster> {
    let routed = routed_set(space, id);
    Cluster::ALL.into_iter().filter(move |cluster| routed & cluster.bit() != 0)
}

/// The clusters [`ROUTES`] sends an id to, as a set.
fn routed_set(space: IdSpace, id: i32) -> u16 {
    ROUTES.iter().filter(|route| route.covers(space, id)).fold(0, |set, route| set | route.cluster.bit())
}

/// The player's host: the state of the screen being drawn, borrowed for one frame.
///
/// Everything is public so the screen that assembles a frame fills in the clusters it has state
/// for and leaves the rest at their defaults, which know nothing.
pub struct ScreenHost<'a> {
    /// The scene clock the frame is drawn against, in microseconds.
    pub now_us: i64,
    /// The scene's timers, which answer every timer read.
    pub timers: &'a TimerState,
    /// The player's own nudges for the document being drawn, asked after the offsets the game
    /// itself sets.
    pub offsets: Option<&'a dyn OffsetSource>,
    /// Which kind of screen this is, which decides the options that are settled once on it. `None`
    /// settles nothing once.
    pub static_screen: Option<StaticScreen>,
    /// The window size in pixels, or `None` for the contract's default.
    pub window: Option<(i32, i32)>,
    pub chart: chart::ChartState<'a>,
    pub score: score::ScoreState<'a>,
    pub play: play::PlayState<'a>,
    pub options: options::OptionsState<'a>,
    pub select: select::SelectState<'a>,
    pub result: result::ResultState<'a>,
    pub ir: ir::IrState<'a>,
    pub system: system::SystemState<'a>,
    pub skin_config: skin_config::SkinConfigState<'a>,
    pub keyconfig: keyconfig::KeyConfigState<'a>,
    pub loading: loading::LoadingState,
    /// The host this screen was drawn from before the clusters existed, asked for whatever no
    /// cluster knows. Its answer to a read is taken as it stands, absent value included; what a
    /// skin tells the game is never passed on to it.
    pub fallback: Option<&'a dyn SkinHost>,
    /// What the skin told the game to do this frame, oldest first.
    calls: RefCell<Vec<HostCall>>,
}

impl<'a> ScreenHost<'a> {
    /// A host for one frame at `now_us` over `timers`, with every cluster knowing nothing.
    pub fn new(now_us: i64, timers: &'a TimerState) -> ScreenHost<'a> {
        ScreenHost {
            now_us,
            timers,
            offsets: None,
            static_screen: None,
            window: None,
            chart: chart::ChartState::default(),
            score: score::ScoreState::default(),
            play: play::PlayState::default(),
            options: options::OptionsState::default(),
            select: select::SelectState::default(),
            result: result::ResultState::default(),
            ir: ir::IrState::default(),
            system: system::SystemState::default(),
            skin_config: skin_config::SkinConfigState::default(),
            keyconfig: keyconfig::KeyConfigState::default(),
            loading: loading::LoadingState::default(),
            fallback: None,
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Everything the skin told the game to do since this was last asked, oldest first: the events
    /// it ran, what it wrote back, and the sounds and volumes it asked for.
    pub fn take_calls(&self) -> Vec<HostCall> {
        self.calls.take()
    }

    /// The state one cluster reads.
    fn cluster(&self, cluster: Cluster) -> &dyn ClusterState {
        match cluster {
            Cluster::Loading => &self.loading,
            Cluster::Result => &self.result,
            Cluster::Select => &self.select,
            Cluster::Play => &self.play,
            Cluster::Score => &self.score,
            Cluster::Chart => &self.chart,
            Cluster::Options => &self.options,
            Cluster::Ir => &self.ir,
            Cluster::System => &self.system,
            Cluster::SkinConfig => &self.skin_config,
            Cluster::KeyConfig => &self.keyconfig,
        }
    }

    /// Asks the clusters for one id: the ones [`ROUTES`] names for it first, then the rest, each in
    /// the order of [`Cluster::ALL`], stopping at the first that knows it.
    fn ask<'s, T>(&'s self, space: IdSpace, id: i32, read: impl Fn(&'s dyn ClusterState) -> Option<T>) -> Option<T> {
        let routed = routed_set(space, id);
        let named = Cluster::ALL.into_iter().filter(|cluster| routed & cluster.bit() != 0);
        let rest = Cluster::ALL.into_iter().filter(|cluster| routed & cluster.bit() == 0);
        named.chain(rest).find_map(|cluster| read(self.cluster(cluster)))
    }

    /// Records one thing the skin told the game to do.
    fn record(&self, call: HostCall) {
        self.calls.borrow_mut().push(call);
    }
}

impl std::fmt::Debug for ScreenHost<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ScreenHost").field("now_us", &self.now_us).field("static_screen", &self.static_screen).finish_non_exhaustive()
    }
}

impl OffsetSource for ScreenHost<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.ask(IdSpace::Offset, id, |cluster| cluster.offset(id))
            .or_else(|| self.fallback.and_then(|older| older.offset(id)))
            .or_else(|| self.offsets.and_then(|nudges| nudges.offset(id)))
    }
}

impl DrawStateSource for ScreenHost<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let asked = normalize_boolean_id(id);
        match self.ask(IdSpace::Boolean, asked, |cluster| cluster.boolean(asked)) {
            Some(answer) => Some(if id < 0 { !answer } else { answer }),
            None => self.fallback.and_then(|older| older.boolean(id)),
        }
    }
}

impl SkinHost for ScreenHost<'_> {
    fn is_static(&self, id: i32) -> bool {
        self.static_screen.is_some_and(|screen| static_scope(id).holds_on(screen))
    }

    fn integer(&self, id: i32) -> i32 {
        self.ask(IdSpace::Integer, id, |cluster| cluster.integer(id)).or_else(|| self.fallback.map(|older| older.integer(id))).unwrap_or(INTEGER_ABSENT)
    }

    fn image_index(&self, id: i32) -> i32 {
        self.ask(IdSpace::ImageIndex, id, |cluster| cluster.image_index(id))
            .or_else(|| self.fallback.map(|older| older.image_index(id)))
            .unwrap_or(IMAGE_INDEX_ABSENT)
    }

    fn rate(&self, id: i32) -> Option<f32> {
        self.ask(IdSpace::Rate, id, |cluster| cluster.rate(id)).or_else(|| self.fallback.and_then(|older| older.rate(id)))
    }

    fn float(&self, id: i32) -> f32 {
        self.ask(IdSpace::Float, id, |cluster| cluster.float(id))
            .or_else(|| self.ask(IdSpace::Rate, id, |cluster| cluster.rate(id)))
            .or_else(|| self.fallback.map(|older| older.float(id)))
            .unwrap_or(FLOAT_ABSENT)
    }

    fn text(&self, id: i32) -> Cow<'_, str> {
        self.ask(IdSpace::Text, id, |cluster| cluster.text(id)).or_else(|| self.fallback.map(|older| older.text(id))).unwrap_or(Cow::Borrowed(TEXT_ABSENT))
    }

    fn timer_us(&self, id: i32) -> i64 {
        self.timers.value_us(TimerId(id))
    }

    fn now_us(&self) -> i64 {
        self.now_us
    }

    fn exec_event(&self, id: i32, arg1: i32, arg2: i32) {
        self.record(HostCall::Event { id, arg1, arg2 });
    }

    fn write_rate(&self, id: i32, value: f32) {
        self.record(HostCall::WriteRate { id, value });
    }

    fn write_text(&self, id: i32, value: &str) {
        self.record(HostCall::WriteText { id, value: value.to_owned() });
    }

    fn audio(&self, command: AudioCommand<'_>) {
        self.record(match command {
            AudioCommand::Play { path, volume, looped } => HostCall::AudioPlay { path: path.to_path_buf(), volume, looped },
            AudioCommand::Preload { path } => HostCall::AudioPreload { path: path.to_path_buf() },
            AudioCommand::Stop { path } => HostCall::AudioStop { path: path.to_path_buf() },
            AudioCommand::Dispose { path } => HostCall::AudioDispose { path: path.to_path_buf() },
        });
    }

    fn key_pressed(&self, code: i32) -> bool {
        self.system.key_pressed(code).or_else(|| self.fallback.map(|older| older.key_pressed(code))).unwrap_or_default()
    }

    fn screen_size(&self) -> (i32, i32) {
        self.window.unwrap_or((rbms_skin::model::DEFAULT_SKIN_WIDTH, rbms_skin::model::DEFAULT_SKIN_HEIGHT))
    }

    fn gauge(&self) -> f32 {
        self.score.gauge().or_else(|| self.fallback.map(|older| older.gauge())).unwrap_or_default()
    }

    fn gauge_type(&self) -> i32 {
        self.score.gauge_type().or_else(|| self.fallback.map(|older| older.gauge_type())).unwrap_or_default()
    }

    fn judge(&self, judge: i32) -> i32 {
        self.score.judge(judge).or_else(|| self.fallback.map(|older| older.judge(judge))).unwrap_or_default()
    }

    fn score(&self, slot: ScoreSlot) -> ScoreSnapshot {
        self.score.score(slot).or_else(|| self.fallback.map(|older| older.score(slot))).unwrap_or_default()
    }

    fn volume(&self, bus: VolumeBus) -> f32 {
        self.system.volume(bus).or_else(|| self.fallback.map(|older| older.volume(bus))).unwrap_or_default()
    }

    fn set_volume(&self, bus: VolumeBus, value: f32) {
        self.record(HostCall::SetVolume { bus, value });
    }
}

#[cfg(test)]
mod tests;
