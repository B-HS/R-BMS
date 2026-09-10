//! What a skin reads the running game through, one adapter per screen.
//!
//! A document addresses game state by property id alone, so something has to answer those ids. The
//! adapters here answer them from the very view structs the built-in screens are already handed --
//! [`HudView`] for play, [`SelectView`] for the browser, [`ResultView`] for the score -- which keeps
//! one description of a frame feeding both the built-in screen and a document that replaces it.
//!
//! Only the ids this build genuinely knows are answered. Everything else falls through to the
//! registry's documented defaults, so a document that asks for something rbms does not measure draws
//! a zero or an empty string instead of failing.

use rbms_skin::dst::{DrawStateSource, OffsetSource, SkinOffset};
use rbms_skin::property::generated::*;
use rbms_skin::property::{
    FLOAT_MAX, FLOAT_MIN, STRING_KEYNAME_EXTENDED_FIRST, STRING_KEYNAME_FIRST, SkinStateSource, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER,
    UNMAPPED_STRING, clamp_float,
};

use crate::hud::HudView;
use crate::result::{ResultView, TargetView, lane_kind_total};
use crate::select::{SelectDetail, SelectView};

/// How many judgements a run is counted in, best first.
const JUDGEMENTS: usize = 6;

/// The whole-number ids each judgement's count answers, in the order [`ResultView::counts`] holds
/// them.
const JUDGE_COUNT_IDS: [i32; JUDGEMENTS] = [NUMBER_PERFECT, NUMBER_GREAT, NUMBER_GOOD, NUMBER_BAD, NUMBER_POOR, NUMBER_MISS];

/// The rate id each judgement's share answers, paired with the count it is measured from.
///
/// There are five rather than six because the reference declares no rate for a miss: a missed note
/// is counted under `POOR`, which is where its share shows up too.
const JUDGE_RATE_IDS: [(i32, usize); 5] = [(RATE_PGREAT, 0), (RATE_GREAT, 1), (RATE_GOOD, 2), (RATE_BAD, 3), (RATE_POOR, 4)];

/// The option ids that report which judgement the last input took.
const JUDGE_OPTION_IDS: [i32; JUDGEMENTS] = [OPTION_1P_PERFECT, OPTION_1P_GREAT, OPTION_1P_GOOD, OPTION_1P_BAD, OPTION_1P_POOR, OPTION_1P_MISS];

/// Places a whole number keeps after the point when a document asks for the fraction separately.
const AFTERDOT_PLACES: f64 = 100.0;

/// Percent of a full gauge.
const GAUGE_FULL: f32 = 100.0;

/// EX points a single note is worth at its best judgement, which is what turns a note count into a
/// maximum score.
const POINTS_PER_NOTE: u32 = 2;

/// The whole part of a number a document splits across two objects.
fn whole(value: f64) -> i32 {
    value.trunc() as i32
}

/// The fractional part of such a number, as the two places the reference's `AFTERDOT` ids carry.
fn afterdot(value: f64) -> i32 {
    ((value.abs().fract() * AFTERDOT_PLACES) as i32).clamp(0, AFTERDOT_PLACES as i32 - 1)
}

/// One value as a share of another, in the range a skin property answers in.
fn share(part: f64, whole: f64) -> f32 {
    if whole <= 0.0 { UNMAPPED_FLOAT } else { clamp_float((part / whole) as f32) }
}

/// The player's own nudges, when any have been made.
type Offsets<'a> = Option<&'a dyn OffsetSource>;

/// Reads an offset from the player's configuration, or reports none.
fn offset_of(offsets: Offsets<'_>, id: i32) -> Option<SkinOffset> {
    offsets.and_then(|source| source.offset(id))
}

/// The play screen's state, read from the same HUD snapshot the built-in screen draws.
pub struct PlayViewState<'a> {
    pub hud: &'a HudView<'a>,
    /// The chart's title, for the documents that show it while it plays.
    pub title: &'a str,
    /// Where the play head is and how long the chart runs, in milliseconds.
    pub song_ms: i64,
    pub duration_ms: i64,
    /// The tempo under the play head.
    pub bpm: f64,
    /// The scroll speed multiplier.
    pub hispeed: f64,
    pub autoplay: bool,
    pub now_ms: i64,
    pub offsets: Offsets<'a>,
}

impl std::fmt::Debug for PlayViewState<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("PlayViewState").field("title", &self.title).field("now_ms", &self.now_ms).finish_non_exhaustive()
    }
}

impl PlayViewState<'_> {
    /// How many notes the run holds, derived from the full-scale EX the HUD carries.
    fn total_notes(&self) -> u32 {
        self.hud.max_ex / POINTS_PER_NOTE
    }
}

impl OffsetSource for PlayViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        offset_of(self.offsets, id)
    }
}

impl DrawStateSource for PlayViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let answer = match id.abs() {
            OPTION_AUTOPLAYON => self.autoplay,
            OPTION_AUTOPLAYOFF => !self.autoplay,
            OPTION_1P_EARLY => self.hud.last_judge.is_some() && self.hud.last_fast,
            OPTION_1P_LATE => self.hud.last_judge.is_some() && !self.hud.last_fast,
            other => match JUDGE_OPTION_IDS.iter().position(|option| *option == other) {
                Some(index) => self.hud.last_judge == Some(index as u8),
                None => UNMAPPED_BOOLEAN,
            },
        };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for PlayViewState<'_> {
    fn integer(&self, id: i32) -> i32 {
        if let Some(index) = JUDGE_COUNT_IDS.iter().position(|count| *count == id) {
            return self.hud.counts[index] as i32;
        }
        match id {
            NUMBER_COMBO => self.hud.combo as i32,
            NUMBER_SCORE => self.hud.ex_score as i32,
            NUMBER_MAXSCORE => self.hud.max_ex as i32,
            NUMBER_TOTALNOTES => self.total_notes() as i32,
            NUMBER_HIGHSCORE => self.hud.best_ex.unwrap_or_default() as i32,
            NUMBER_DIFF_HIGHSCORE => self.hud.best_ex.map_or(0, |best| self.hud.ex_score as i32 - best as i32),
            NUMBER_GROOVEGAUGE => whole(f64::from(self.hud.gauge)),
            NUMBER_GROOVEGAUGE_AFTERDOT => afterdot(f64::from(self.hud.gauge)),
            NUMBER_HISPEED => whole(self.hispeed),
            NUMBER_HISPEED_AFTERDOT => afterdot(self.hispeed),
            NUMBER_NOWBPM => whole(self.bpm),
            NUMBER_DURATION_GREEN => self.hud.green_number as i32,
            NUMBER_LANECOVER1 => self.hud.white_number as i32,
            _ => UNMAPPED_INTEGER,
        }
    }

    fn float(&self, id: i32) -> f32 {
        if let Some((_, index)) = JUDGE_RATE_IDS.iter().find(|(rate, _)| *rate == id) {
            return share(f64::from(self.hud.counts[*index]), f64::from(self.total_notes()));
        }
        match id {
            RATE_EXSCORE | FLOAT_SCORE_RATE => share(f64::from(self.hud.ex_score), f64::from(self.hud.max_ex)),
            FLOAT_GROOVEGAUGE_1P => clamp_float(self.hud.gauge / GAUGE_FULL),
            RATE_MUSIC_PROGRESS | RATE_MUSIC_PROGRESS_BAR => share(self.song_ms as f64, self.duration_ms as f64),
            _ => UNMAPPED_FLOAT,
        }
    }

    fn string(&self, id: i32) -> &str {
        match id {
            STRING_TITLE | STRING_FULLTITLE => self.title,
            _ => UNMAPPED_STRING,
        }
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}

/// The browser's state, read from the view the built-in browser draws.
pub struct SelectViewState<'a> {
    pub view: &'a SelectView,
    pub now_ms: i64,
    pub offsets: Offsets<'a>,
}

impl std::fmt::Debug for SelectViewState<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SelectViewState").field("now_ms", &self.now_ms).finish_non_exhaustive()
    }
}

impl SelectViewState<'_> {
    /// The focused chart's detail pane, when a chart rather than a folder is focused.
    fn song(&self) -> Option<&crate::select::DetailView> {
        match &self.view.detail {
            SelectDetail::Song(detail) => Some(detail),
            _ => None,
        }
    }
}

impl OffsetSource for SelectViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        offset_of(self.offsets, id)
    }
}

impl DrawStateSource for SelectViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let answer = match id.abs() {
            OPTION_FOLDERBAR => matches!(self.view.detail, SelectDetail::Folder { .. }),
            OPTION_SONGBAR => self.song().is_some(),
            _ => UNMAPPED_BOOLEAN,
        };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for SelectViewState<'_> {
    fn integer(&self, id: i32) -> i32 {
        match id {
            NUMBER_PLAYLEVEL => self.song().and_then(|song| song.level.parse().ok()).unwrap_or(UNMAPPED_INTEGER),
            _ => UNMAPPED_INTEGER,
        }
    }

    fn float(&self, id: i32) -> f32 {
        match id {
            RATE_MUSICSELECT_POSITION if !self.view.rows.is_empty() => clamp_float(self.view.sel as f32 / (self.view.rows.len() - 1).max(1) as f32),
            _ => UNMAPPED_FLOAT,
        }
    }

    fn string(&self, id: i32) -> &str {
        match id {
            STRING_DIRECTORY => &self.view.header,
            STRING_SEARCHWORD => self.view.search.as_deref().unwrap_or(UNMAPPED_STRING),
            STRING_TITLE | STRING_FULLTITLE => self.song().map_or(UNMAPPED_STRING, |song| song.title.as_str()),
            STRING_SUBTITLE => self.song().map_or(UNMAPPED_STRING, |song| song.subtitle.as_str()),
            STRING_ARTIST | STRING_FULLARTIST => self.song().map_or(UNMAPPED_STRING, |song| song.artist.as_str()),
            STRING_GENRE => self.song().map_or(UNMAPPED_STRING, |song| song.genre_maker.as_str()),
            _ => UNMAPPED_STRING,
        }
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}

/// The score screen's state, read from the view the built-in screen draws.
pub struct ResultViewState<'a> {
    pub view: &'a ResultView,
    /// What the run was paced against, when it was paced against anything.
    pub target: Option<&'a TargetView>,
    /// Whether the run counted as cleared.
    pub cleared: bool,
    pub now_ms: i64,
    pub offsets: Offsets<'a>,
}

impl std::fmt::Debug for ResultViewState<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ResultViewState").field("cleared", &self.cleared).field("now_ms", &self.now_ms).finish_non_exhaustive()
    }
}

impl OffsetSource for ResultViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        offset_of(self.offsets, id)
    }
}

impl DrawStateSource for ResultViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let answer = match id.abs() {
            OPTION_RESULT_CLEAR => self.cleared,
            OPTION_RESULT_FAIL => !self.cleared,
            _ => UNMAPPED_BOOLEAN,
        };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for ResultViewState<'_> {
    fn integer(&self, id: i32) -> i32 {
        if let Some(index) = JUDGE_COUNT_IDS.iter().position(|count| *count == id) {
            return self.view.counts[index] as i32;
        }
        match id {
            NUMBER_SCORE => self.view.ex_score as i32,
            NUMBER_MAXSCORE => self.view.max_score as i32,
            NUMBER_MAXCOMBO => self.view.max_combo as i32,
            NUMBER_TOTALNOTES => self.view.total_notes as i32,
            NUMBER_HIGHSCORE => self.view.prev_best_ex.unwrap_or_default() as i32,
            NUMBER_DIFF_HIGHSCORE => self.view.prev_best_ex.map_or(0, |best| self.view.ex_score as i32 - best as i32),
            NUMBER_TARGET_SCORE => self.target.map_or(UNMAPPED_INTEGER, |target| target.ex as i32),
            NUMBER_DIFF_TARGETSCORE => self.target.map_or(0, |target| self.view.ex_score as i32 - target.ex as i32),
            NUMBER_GROOVEGAUGE => whole(f64::from(self.view.gauge)),
            NUMBER_GROOVEGAUGE_AFTERDOT => afterdot(f64::from(self.view.gauge)),
            NUMBER_EARLY_PERFECT => lane_kind_total(self.view.fast) as i32,
            NUMBER_LATE_PERFECT => lane_kind_total(self.view.slow) as i32,
            _ => UNMAPPED_INTEGER,
        }
    }

    fn float(&self, id: i32) -> f32 {
        if let Some((_, index)) = JUDGE_RATE_IDS.iter().find(|(rate, _)| *rate == id) {
            return share(f64::from(self.view.counts[*index]), f64::from(self.view.total_notes));
        }
        match id {
            RATE_EXSCORE | FLOAT_SCORE_RATE => share(f64::from(self.view.ex_score), f64::from(self.view.max_score)),
            RATE_BESTSCORE => self.view.prev_best_ex.map_or(UNMAPPED_FLOAT, |best| share(f64::from(best), f64::from(self.view.max_score))),
            RATE_TARGETSCORE | FLOAT_TARGET_RATE => self.target.map_or(UNMAPPED_FLOAT, |target| share(f64::from(target.ex), f64::from(self.view.max_score))),
            FLOAT_GROOVEGAUGE_1P => clamp_float(self.view.gauge / GAUGE_FULL),
            _ => UNMAPPED_FLOAT,
        }
    }

    fn string(&self, id: i32) -> &str {
        match id {
            STRING_TITLE | STRING_FULLTITLE => &self.view.title,
            _ => UNMAPPED_STRING,
        }
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}

/// The loading screen's state, which stands in for the reference's decide screen.
pub struct DecideViewState<'a> {
    /// How far the load has got, from nothing to everything.
    pub progress: f32,
    pub done: bool,
    pub title: &'a str,
    pub now_ms: i64,
    pub offsets: Offsets<'a>,
}

impl std::fmt::Debug for DecideViewState<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("DecideViewState").field("progress", &self.progress).field("done", &self.done).finish_non_exhaustive()
    }
}

impl OffsetSource for DecideViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        offset_of(self.offsets, id)
    }
}

impl DrawStateSource for DecideViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let answer = match id.abs() {
            OPTION_NOW_LOADING => !self.done,
            OPTION_LOADED => self.done,
            _ => UNMAPPED_BOOLEAN,
        };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for DecideViewState<'_> {
    fn integer(&self, id: i32) -> i32 {
        match id {
            NUMBER_LOADING_PROGRESS => (self.progress.clamp(FLOAT_MIN, FLOAT_MAX) * GAUGE_FULL) as i32,
            _ => UNMAPPED_INTEGER,
        }
    }

    fn float(&self, id: i32) -> f32 {
        match id {
            RATE_LOAD_PROGRESS | FLOAT_LOADING_PROGRESS => clamp_float(self.progress),
            _ => UNMAPPED_FLOAT,
        }
    }

    fn string(&self, id: i32) -> &str {
        match id {
            STRING_TITLE | STRING_FULLTITLE => self.title,
            _ => UNMAPPED_STRING,
        }
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}

/// The key configuration screen's state: what each lane is currently bound to.
///
/// The reference generates the key-name ids rather than declaring constants for them, which is why
/// the two bands are named in the property registry and read here as plain ranges.
pub struct KeyConfigViewState<'a> {
    /// One binding label per lane, in lane order.
    pub keys: &'a [String],
    pub now_ms: i64,
    pub offsets: Offsets<'a>,
}

impl std::fmt::Debug for KeyConfigViewState<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("KeyConfigViewState").field("keys", &self.keys.len()).finish_non_exhaustive()
    }
}

impl OffsetSource for KeyConfigViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        offset_of(self.offsets, id)
    }
}

impl DrawStateSource for KeyConfigViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        if id < 0 { !UNMAPPED_BOOLEAN } else { UNMAPPED_BOOLEAN }
    }
}

impl SkinStateSource for KeyConfigViewState<'_> {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
    }

    fn string(&self, id: i32) -> &str {
        let lane = match id {
            id if id >= STRING_KEYNAME_EXTENDED_FIRST => id - STRING_KEYNAME_EXTENDED_FIRST,
            id if id >= STRING_KEYNAME_FIRST => id - STRING_KEYNAME_FIRST,
            _ => return UNMAPPED_STRING,
        };
        self.keys.get(lane.max(0) as usize).map_or(UNMAPPED_STRING, String::as_str)
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}
