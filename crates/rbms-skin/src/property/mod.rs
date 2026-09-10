//! The property registry: the values a skin reads game state through, and the routing table that
//! says which part of the running game answers each one.
//!
//! A skin addresses game state by integer id alone, so the id space is the whole contract. The ids
//! themselves are machine-extracted into [`generated`] and never hand-written; what this module
//! adds is the rbms side of the contract: which kind of value an id carries ([`PropertyKind`]),
//! which subsystem owns it ([`StateSource`] and [`MAPPINGS`]), what an id this build does not
//! answer returns, and a place to count those misses ([`UnmappedLog`]).
//!
//! The game implements [`SkinStateSource`]. Because that trait builds on
//! [`DrawStateSource`](crate::dst::DrawStateSource), one implementation serves both the value reads
//! here and the draw gating in [`crate::dst`].

pub mod generated;

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::dst::{DrawStateSource, OffsetSource, SkinOffset};
use crate::timer::{ALL_TIMER, timer_id};
use generated::*;

/// What kind of value a skin property id carries, and therefore which accessor of
/// [`SkinStateSource`] reads it.
///
/// Each kind owns its own id space: id 41 is a different property as a boolean, an integer and a
/// timer. Nothing here is a global id lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PropertyKind {
    /// `OPTION_*`, read by [`SkinStateSource::boolean`]. A negative id negates the answer.
    Boolean,
    /// `NUMBER_*`, read by [`SkinStateSource::integer`].
    Integer,
    /// `RATE_*`, `SLIDER_*`, `BARGRAPH_*` and `FLOAT_*`, read by [`SkinStateSource::float`].
    Float,
    /// `STRING_*`, read by [`SkinStateSource::string`].
    String,
    /// `TIMER_*`, read by [`SkinStateSource::timer`].
    Timer,
}

impl PropertyKind {
    /// Every kind, in the order this module documents them.
    pub const ALL: [PropertyKind; 5] = [PropertyKind::Boolean, PropertyKind::Integer, PropertyKind::Float, PropertyKind::String, PropertyKind::Timer];

    /// The generated tables this kind reads, in lookup order.
    ///
    /// [`PropertyKind::Float`] has four because the reference names one id space with three
    /// prefixes (`RATE_*` is canonical, `SLIDER_*` and `BARGRAPH_*` are older aliases) and adds a
    /// second, non-overlapping space under `FLOAT_*`.
    pub const fn tables(self) -> &'static [&'static [(i32, &'static str)]] {
        match self {
            PropertyKind::Boolean => &[ALL_OPTION],
            PropertyKind::Integer => &[ALL_NUMBER],
            PropertyKind::Float => &[ALL_RATE, ALL_FLOAT, ALL_SLIDER, ALL_BARGRAPH],
            PropertyKind::String => &[ALL_STRING],
            PropertyKind::Timer => &[ALL_TIMER],
        }
    }

    /// The extracted name of an id, for logs and the skin debug view.
    ///
    /// The name comes back as its table records it: with the declaring prefix for every kind but
    /// [`PropertyKind::Timer`], whose table strips `TIMER_` because it is the only prefix there. A
    /// float id declared under more than one prefix reports the first match in [`Self::tables`].
    pub fn name(self, id: i32) -> Option<&'static str> {
        self.tables().iter().flat_map(|table| table.iter()).find(|(value, _)| *value == id).map(|(_, name)| *name)
    }

    /// Whether the reference declares this id for this kind.
    pub fn is_declared(self, id: i32) -> bool {
        self.name(id).is_some()
    }

    /// How many declarations the reference holds for this kind, aliases included.
    pub fn declaration_count(self) -> usize {
        self.tables().iter().map(|table| table.len()).sum()
    }
}

/// What [`SkinStateSource::boolean`] answers for an id this build does not implement.
pub const UNMAPPED_BOOLEAN: bool = false;

/// What [`SkinStateSource::integer`] answers for an id this build does not implement.
pub const UNMAPPED_INTEGER: i32 = 0;

/// What [`SkinStateSource::float`] answers for an id this build does not implement.
pub const UNMAPPED_FLOAT: f32 = 0.0;

/// What [`SkinStateSource::string`] answers for an id this build does not implement.
pub const UNMAPPED_STRING: &str = "";

/// What [`SkinStateSource::now_ms`] answers before a frame clock exists.
pub const UNMAPPED_CLOCK_MS: i64 = 0;

/// The low end of the range [`SkinStateSource::float`] answers in.
pub const FLOAT_MIN: f32 = 0.0;

/// The high end of the range [`SkinStateSource::float`] answers in.
pub const FLOAT_MAX: f32 = 1.0;

/// Forces a rate into the range a skin expects, mapping a NaN to [`UNMAPPED_FLOAT`].
///
/// Sliders and bar graphs multiply this by a pixel length, so an out-of-range or NaN value would
/// draw outside its object rather than fail visibly. It is for the rate half of the float id space
/// alone: a `FLOAT_*` id such as a hi-speed multiplier or an average timing in milliseconds is not
/// a share of anything and passes through [`sanitize_float`] instead.
pub fn clamp_float(value: f32) -> f32 {
    if value.is_nan() { UNMAPPED_FLOAT } else { value.clamp(FLOAT_MIN, FLOAT_MAX) }
}

/// Replaces a value a number line has no room for with [`UNMAPPED_FLOAT`], leaving every finite one
/// as it is.
///
/// This is what a drawn `floatvalue` reads through, because `SkinFloat.prepare` scales the property
/// by the object's `gain` without narrowing it: a hi-speed of 2.5 and a timing average of -12.4 ms
/// are both ordinary values there.
pub fn sanitize_float(value: f32) -> f32 {
    if value.is_finite() { value } else { UNMAPPED_FLOAT }
}

/// The id a boolean read actually looks up: a negative id addresses `abs(id)` and negates the
/// answer (`BooleanPropertyFactory.getBooleanProperty`).
pub fn normalize_boolean_id(id: i32) -> i32 {
    id.saturating_abs()
}

/// Everything a skin can ask the running game.
///
/// The boolean and offset reads come from the supertraits, so one implementation serves both this
/// registry and the draw gating in [`crate::dst`]. Every accessor answers for an id it does not
/// implement rather than failing: the `UNMAPPED_*` constants above, or `None` for a timer.
pub trait SkinStateSource: DrawStateSource {
    /// The integer under `id`, or [`UNMAPPED_INTEGER`].
    fn integer(&self, id: i32) -> i32;
    /// The float under `id`, or [`UNMAPPED_FLOAT`].
    ///
    /// A rate id -- `RATE_*` and its `SLIDER_*`/`BARGRAPH_*` aliases -- answers in
    /// [`FLOAT_MIN`]..=[`FLOAT_MAX`], because that is the share a slider or bar graph multiplies by
    /// its own length. The separate `FLOAT_*` id space carries plain measurements and is not held
    /// to that range.
    fn float(&self, id: i32) -> f32;
    /// The text under `id`, or [`UNMAPPED_STRING`].
    fn string(&self, id: i32) -> &str;
    /// When the timer under `id` switched on, or `None` while it is off.
    fn timer(&self, id: i32) -> Option<i64>;
    /// The clock the frame is being drawn against, in milliseconds.
    ///
    /// This is what `skin.time()` returns. It must be the same clock the frame passes to
    /// [`crate::dst::prepare`], so an expression and the animation it gates never disagree about
    /// what time it is.
    fn now_ms(&self) -> i64;
}

/// A state source that answers nothing.
///
/// Every read returns its documented default, so a screen can resolve a skin before a play session
/// or a score exists, and a test can supply a baseline without writing one out.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultState;

impl OffsetSource for DefaultState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for DefaultState {
    fn boolean(&self, _id: i32) -> bool {
        UNMAPPED_BOOLEAN
    }
}

impl SkinStateSource for DefaultState {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
    }

    fn string(&self, _id: i32) -> &str {
        UNMAPPED_STRING
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        UNMAPPED_CLOCK_MS
    }
}

/// Which part of rbms owns a property's value.
///
/// This routes rather than reads: it says where the implementation of [`SkinStateSource`] should
/// go looking, and therefore which shipping stage wires a band of ids up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StateSource {
    /// The live play session in `rbms-play`: score, combo, gauge, lane input, chart clock.
    PlaySession,
    /// The judgement counters in `rbms-judge`: per-judgement totals, rates and timing statistics.
    JudgeCounters,
    /// The select stage: the bar list, the folder and table it sits in, and the chosen chart.
    SongSelect,
    /// The result stage: the finished score and the best and target scores it compares against.
    ScoreResult,
    /// The loading stage, which stands in for the reference's decide screen.
    Decide,
    /// The key configuration stage: the live binding of each lane.
    KeyConfig,
    /// The score store in `rbms-store`: saved scores and lifetime totals.
    ScoreStore,
    /// The ranking client in `rbms-ir`.
    InternetRanking,
    /// Player configuration in `rbms-config`: options, volumes, hispeed and lane cover.
    PlayerConfig,
    /// Skin selection and the per-skin customise state the skin settings screen edits.
    SkinCustomize,
}

/// The first `STRING_*` id of the ten primary key names the key config screen shows.
///
/// The reference generates these ids in `StringPropertyFactory.java:80` rather than declaring
/// constants for them, so the generator cannot see them and they are named here instead.
pub const STRING_KEYNAME_FIRST: i32 = 40;

/// The last `STRING_*` id of the ten primary key names.
pub const STRING_KEYNAME_LAST: i32 = 49;

/// The first `STRING_*` id of the forty-four extended key names
/// (`StringPropertyFactory.java:81`).
pub const STRING_KEYNAME_EXTENDED_FIRST: i32 = 240;

/// The last `STRING_*` id of the extended key names.
pub const STRING_KEYNAME_EXTENDED_LAST: i32 = 283;

/// One run of property ids that a single part of rbms answers.
///
/// The reference numbers related properties consecutively, so a run is the natural unit: a run of
/// one is written with equal endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    /// Which id space the run lives in.
    pub kind: PropertyKind,
    /// The lowest id the run claims, inclusive.
    pub first_id: i32,
    /// The highest id the run claims, inclusive.
    pub last_id: i32,
    /// The part of rbms that answers every id in the run.
    pub source: StateSource,
}

impl Mapping {
    /// A run from its endpoints, both inclusive.
    const fn new(kind: PropertyKind, first_id: i32, last_id: i32, source: StateSource) -> Self {
        Self { kind, first_id, last_id, source }
    }

    /// A run of a single id.
    const fn one(kind: PropertyKind, id: i32, source: StateSource) -> Self {
        Self::new(kind, id, id, source)
    }

    /// Whether this run covers `id`.
    pub const fn covers(&self, id: i32) -> bool {
        self.first_id <= id && id <= self.last_id
    }
}

/// Which part of rbms answers each property this build implements.
///
/// Ordered by shipping stage, play first, so the bands a play skin needs stand together at the top.
/// Ids outside every run are the ones this build does not answer; they read as the `UNMAPPED_*`
/// defaults and [`UnmappedLog`] counts them.
pub const MAPPINGS: &[Mapping] = &[
    Mapping::new(PropertyKind::Integer, NUMBER_POINT, NUMBER_DIFF_EXSCORE, StateSource::PlaySession),
    Mapping::new(PropertyKind::Integer, NUMBER_TOTAL_RATE, NUMBER_TOTAL_RATE_AFTERDOT, StateSource::PlaySession),
    Mapping::new(PropertyKind::Integer, NUMBER_NOWBPM, NUMBER_TIMELEFT_SECOND, StateSource::PlaySession),
    Mapping::new(PropertyKind::Integer, NUMBER_PERFECT, NUMBER_POOR, StateSource::JudgeCounters),
    Mapping::one(PropertyKind::Float, RATE_MUSIC_PROGRESS, StateSource::PlaySession),
    Mapping::one(PropertyKind::Float, RATE_MUSIC_PROGRESS_BAR, StateSource::PlaySession),
    Mapping::new(PropertyKind::Float, RATE_SCORE, RATE_SCORE_FINAL, StateSource::PlaySession),
    Mapping::one(PropertyKind::Float, FLOAT_SCORE_RATE, StateSource::PlaySession),
    Mapping::one(PropertyKind::Float, FLOAT_TOTAL_RATE, StateSource::PlaySession),
    Mapping::one(PropertyKind::Float, FLOAT_GROOVEGAUGE_1P, StateSource::PlaySession),
    Mapping::new(PropertyKind::Float, FLOAT_PERFECT_RATE, FLOAT_POOR_RATE, StateSource::JudgeCounters),
    Mapping::new(PropertyKind::Float, RATE_PGREAT, RATE_EXSCORE, StateSource::JudgeCounters),
    Mapping::new(PropertyKind::Float, FLOAT_DURATION_AVERAGE, FLOAT_TIMIGN_STDDEV, StateSource::JudgeCounters),
    Mapping::new(PropertyKind::Boolean, OPTION_AUTOPLAYOFF, OPTION_AUTOPLAYON, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_GAUGE_GROOVE, OPTION_GAUGE_HARD_2P, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_GAUGE_EX, OPTION_GAUGE_EX_2P, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_1P_AAA, OPTION_F, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_1P_0_9, OPTION_1P_100, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_1P_PERFECT, OPTION_1P_MISS, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_2P_PERFECT, OPTION_2P_GOOD, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_3P_PERFECT, OPTION_3P_GOOD, StateSource::PlaySession),
    Mapping::one(PropertyKind::Boolean, OPTION_1P_BORDER_OR_MORE, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_1P_EARLY, OPTION_1P_LATE, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_2P_EARLY, OPTION_2P_LATE, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_3P_EARLY, OPTION_3P_LATE, StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_PERFECT_EXIST, OPTION_MISS_EXIST, StateSource::JudgeCounters),
    Mapping::new(PropertyKind::Timer, timer_id::STARTINPUT.get(), timer_id::FAILED.get(), StateSource::PlaySession),
    Mapping::new(PropertyKind::Timer, timer_id::READY.get(), timer_id::FULLCOMBO_2P.get(), StateSource::PlaySession),
    Mapping::new(PropertyKind::Timer, timer_id::BOMB_1P_SCRATCH.get(), timer_id::HOLD_2P_KEY1.get(), StateSource::PlaySession),
    Mapping::new(PropertyKind::Timer, timer_id::KEYON_1P_SCRATCH.get(), timer_id::RHYTHM.get(), StateSource::PlaySession),
    Mapping::new(PropertyKind::Timer, timer_id::ENDOFNOTE_1P.get(), timer_id::ENDOFNOTE_2P.get(), StateSource::PlaySession),
    Mapping::one(PropertyKind::Timer, timer_id::JUDGE_3P.get(), StateSource::PlaySession),
    Mapping::new(PropertyKind::Timer, timer_id::COMBO_1P.get(), timer_id::COMBO_3P.get(), StateSource::PlaySession),
    Mapping::new(PropertyKind::Boolean, OPTION_FOLDERBAR, OPTION_PLAYABLEBAR, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_PANEL1, OPTION_PANEL3, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_LEVEL_BEGINNER, OPTION_LEVEL_INSANE_EXCEED, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_SELECT_BAR_NOT_PLAYED, OPTION_SELECT_BAR_FULL_COMBO_CLEARED, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_CLEAR_GROOVE, OPTION_CLEAR_ALLSCR, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_NO_BGA, OPTION_RANDOMSEQUENCE, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_NO_STAGEFILE, OPTION_BACKBMP, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_COURSE_STAGE1, OPTION_MODE_GRADE, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_GRADEBAR_CLASS, OPTION_GRADEBAR_HCN, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_RANDOMSELECTBAR, OPTION_RANDOMCOURSEBAR, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_SELECT_BAR_ASSIST_EASY_CLEARED, OPTION_SELECT_BAR_MAX_CLEARED, StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_CLEAR_RRANDOM, OPTION_CLEAR_EXSRANDOM, StateSource::SongSelect),
    Mapping::one(PropertyKind::Boolean, OPTION_BPMSTOP, StateSource::SongSelect),
    Mapping::new(PropertyKind::Integer, NUMBER_FOLDER_BEGINNER, NUMBER_FOLDER_INSANE, StateSource::SongSelect),
    Mapping::one(PropertyKind::Integer, NUMBER_TOTALNOTES, StateSource::SongSelect),
    Mapping::new(PropertyKind::Integer, NUMBER_MAXBPM, NUMBER_MAINBPM, StateSource::SongSelect),
    Mapping::one(PropertyKind::Integer, NUMBER_PLAYLEVEL, StateSource::SongSelect),
    Mapping::one(PropertyKind::Float, RATE_MUSICSELECT_POSITION, StateSource::SongSelect),
    Mapping::one(PropertyKind::Float, RATE_LEVEL, StateSource::SongSelect),
    Mapping::new(PropertyKind::Float, RATE_LEVEL_BEGINNER, RATE_LEVEL_INSANE, StateSource::SongSelect),
    Mapping::new(PropertyKind::Float, FLOAT_CHART_PEAKDENSITY, FLOAT_CHART_TOTALGAUGE, StateSource::SongSelect),
    Mapping::new(PropertyKind::String, STRING_TITLE, STRING_FULLARTIST, StateSource::SongSelect),
    Mapping::one(PropertyKind::String, STRING_SEARCHWORD, StateSource::SongSelect),
    Mapping::new(PropertyKind::String, STRING_DIRECTORY, STRING_TABLE_FULL, StateSource::SongSelect),
    Mapping::new(PropertyKind::String, STRING_SONG_HASH_MD5, STRING_SONG_HASH_SHA256, StateSource::SongSelect),
    Mapping::new(PropertyKind::Timer, timer_id::SONGBAR_MOVE.get(), timer_id::README_END.get(), StateSource::SongSelect),
    Mapping::new(PropertyKind::Timer, timer_id::PANEL1_ON.get(), timer_id::PANEL6_OFF.get(), StateSource::SongSelect),
    Mapping::new(PropertyKind::Boolean, OPTION_RESULT_CLEAR, OPTION_RESULT_FAIL, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_NO_SAVE_CLEAR, OPTION_FULLCOMBO_SAVE_CLEAR, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_RESULT_AAA_1P, OPTION_RESULT_0_2P, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_BEST_AAA_1P, OPTION_UPDATE_TARGET, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_NOW_AAA_1P, OPTION_NOW_F_1P, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_DISABLE_RESULTFLIP, OPTION_DRAW, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_DRAW_SCORE, OPTION_DRAW_TARGET, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Integer, NUMBER_TARGET_SCORE, NUMBER_TARGET_SCORE_RATE_AFTERDOT, StateSource::ScoreResult),
    Mapping::one(PropertyKind::Integer, NUMBER_DIFF_EXSCORE2, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Integer, NUMBER_TARGET_TOTAL_RATE, NUMBER_TARGET_TOTAL_RATE_AFTERDOT, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Integer, NUMBER_TARGET_SCORE2, NUMBER_TARGET_SCORE_RATE_AFTERDOT2, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Integer, NUMBER_HIGHSCORE2, NUMBER_DIFF_MISSCOUNT, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Float, RATE_BESTSCORE_NOW, RATE_TARGETSCORE, StateSource::ScoreResult),
    Mapping::one(PropertyKind::Float, FLOAT_TARGET_RATE, StateSource::ScoreResult),
    Mapping::one(PropertyKind::Float, FLOAT_SCORE_RATE2, StateSource::ScoreResult),
    Mapping::one(PropertyKind::Float, FLOAT_TARGET_RATE2, StateSource::ScoreResult),
    Mapping::one(PropertyKind::Float, FLOAT_BEST_RATE, StateSource::ScoreResult),
    Mapping::new(PropertyKind::Timer, timer_id::RESULTGRAPH_BEGIN.get(), timer_id::RESULT_UPDATESCORE.get(), StateSource::ScoreResult),
    Mapping::new(PropertyKind::Timer, timer_id::SCORE_A.get(), timer_id::SCORE_TARGET.get(), StateSource::ScoreResult),
    Mapping::new(PropertyKind::Boolean, OPTION_NOW_LOADING, OPTION_LOADED, StateSource::Decide),
    Mapping::one(PropertyKind::Integer, NUMBER_LOADING_PROGRESS, StateSource::Decide),
    Mapping::one(PropertyKind::Float, RATE_LOAD_PROGRESS, StateSource::Decide),
    Mapping::one(PropertyKind::Float, FLOAT_LOADING_PROGRESS, StateSource::Decide),
    Mapping::new(PropertyKind::String, STRING_KEYNAME_FIRST, STRING_KEYNAME_LAST, StateSource::KeyConfig),
    Mapping::new(PropertyKind::String, STRING_KEYNAME_EXTENDED_FIRST, STRING_KEYNAME_EXTENDED_LAST, StateSource::KeyConfig),
    Mapping::new(PropertyKind::Integer, NUMBER_TOTALPLAYTIME_HOUR, NUMBER_TOTALPLAYTIME_SECOND, StateSource::ScoreStore),
    Mapping::new(PropertyKind::Integer, NUMBER_TOTALPLAYCOUNT, NUMBER_TOTALPOOR, StateSource::ScoreStore),
    Mapping::new(PropertyKind::Integer, NUMBER_SCORE, NUMBER_MAXSCORE, StateSource::ScoreStore),
    Mapping::new(PropertyKind::Integer, NUMBER_MAXCOMBO, NUMBER_POOR_RATE, StateSource::ScoreStore),
    Mapping::one(PropertyKind::Integer, NUMBER_HIGHSCORE, StateSource::ScoreStore),
    Mapping::new(PropertyKind::Boolean, OPTION_OFFLINE, OPTION_ONLINE, StateSource::InternetRanking),
    Mapping::new(PropertyKind::Boolean, OPTION_IR_LOADING, OPTION_IR_BUSY, StateSource::InternetRanking),
    Mapping::one(PropertyKind::Float, RATE_RANKING_POSITION, StateSource::InternetRanking),
    Mapping::new(PropertyKind::Float, FLOAT_IR_PLAYER_NOPLAY_RATE, FLOAT_IR_TOTALFULLCOMBORATE, StateSource::InternetRanking),
    Mapping::new(PropertyKind::String, STRING_RANKING1_NAME, STRING_RANKING10_NAME, StateSource::InternetRanking),
    Mapping::new(PropertyKind::String, STRING_IR_NAME, STRING_IR_USER_NAME, StateSource::InternetRanking),
    Mapping::new(PropertyKind::Timer, timer_id::IR_CONNECT_BEGIN.get(), timer_id::IR_CONNECT_FAIL.get(), StateSource::InternetRanking),
    Mapping::new(PropertyKind::Boolean, OPTION_BGANORMAL, OPTION_BGAEXTEND, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Boolean, OPTION_GHOST_OFF, OPTION_SCOREGRAPHON, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Boolean, OPTION_BGAOFF, OPTION_BGAON, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Boolean, OPTION_DISABLE_SAVE_SCORE, OPTION_ENABLE_SAVE_SCORE, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Boolean, OPTION_JUDGE_VERYHARD, OPTION_JUDGE_VERYEASY, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Boolean, OPTION_LANECOVER1_CHANGING, OPTION_HIDDEN1_ON, StateSource::PlayerConfig),
    Mapping::one(PropertyKind::Boolean, OPTION_CONSTANT, StateSource::PlayerConfig),
    Mapping::one(PropertyKind::Integer, NUMBER_HISPEED_LR2, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Integer, NUMBER_JUDGETIMING, NUMBER_LANECOVER1, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Integer, NUMBER_MASTER_VOLUME, NUMBER_BGM_VOLUME, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Float, RATE_LANECOVER, RATE_LANECOVER2, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::Float, RATE_MASTERVOLUME, RATE_BGMVOLUME, StateSource::PlayerConfig),
    Mapping::one(PropertyKind::Float, FLOAT_HISPEED, StateSource::PlayerConfig),
    Mapping::new(PropertyKind::String, STRING_RIVAL, STRING_PLAYER, StateSource::PlayerConfig),
    Mapping::one(PropertyKind::String, STRING_VERSION, StateSource::PlayerConfig),
    Mapping::one(PropertyKind::Float, RATE_SKINSELECT_POSITION, StateSource::SkinCustomize),
    Mapping::new(PropertyKind::String, STRING_SKIN_NAME, STRING_SKIN_AUTHOR, StateSource::SkinCustomize),
    Mapping::new(PropertyKind::String, STRING_SKIN_CUSTOMIZE_CATEGORY1, STRING_SKIN_CUSTOMIZE_ITEM10, StateSource::SkinCustomize),
];

/// The id a lookup of this kind actually uses: only a boolean read folds a negative id.
fn normalize_boolean_id_for(kind: PropertyKind, id: i32) -> i32 {
    if kind == PropertyKind::Boolean { normalize_boolean_id(id) } else { id }
}

/// Which part of rbms answers this property, or `None` when this build does not.
///
/// A negative boolean id is normalised first, so an option and its negation route the same way.
pub fn source_of(kind: PropertyKind, id: i32) -> Option<StateSource> {
    let id = normalize_boolean_id_for(kind, id);
    MAPPINGS.iter().find(|mapping| mapping.kind == kind && mapping.covers(id)).map(|mapping| mapping.source)
}

/// Counts the property reads this build cannot answer.
///
/// A skin reads its properties every frame, so a miss must not log every frame. [`Self::check`]
/// reports whether a miss is the first for that id; the caller logs only then, and the totals stay
/// available for the skin debug view.
#[derive(Debug, Default)]
pub struct UnmappedLog {
    seen: Mutex<BTreeSet<(PropertyKind, i32)>>,
    misses: AtomicU64,
}

impl UnmappedLog {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Routes one read, counting it when this build answers no such property.
    ///
    /// `Ok` carries the owning subsystem. `Err` marks a miss and carries whether it is the first
    /// for that id, so a caller can log it once and stay silent on every later frame.
    pub fn check(&self, kind: PropertyKind, id: i32) -> Result<StateSource, bool> {
        match source_of(kind, id) {
            Some(source) => Ok(source),
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Err(self.lock().insert((kind, normalize_boolean_id_for(kind, id))))
            }
        }
    }

    /// How many reads have missed, repeats included.
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Every id that has missed, each once, in kind then id order.
    pub fn distinct(&self) -> Vec<(PropertyKind, i32)> {
        self.lock().iter().copied().collect()
    }

    /// Forgets every recorded miss, for a debug view that wants a fresh count.
    pub fn clear(&self) {
        self.lock().clear();
        self.misses.store(0, Ordering::Relaxed);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeSet<(PropertyKind, i32)>> {
        self.seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
