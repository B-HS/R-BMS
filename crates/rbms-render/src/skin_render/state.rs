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

use super::events::SkinEventClick;
use crate::Rect;
use crate::hud::HudView;
use crate::playfield::{LaneShade, PlayfieldView};
use crate::result::{ResultView, TargetView, dj_rank, ex_delta_label, lane_kind_total};
use crate::select::{SelectDetail, SelectRow, SelectView};
use crate::skin::Skin;
use crate::theme::OPTIONS_ROW_COUNT;

/// How many judgements a run is counted in, best first.
const JUDGEMENTS: usize = 6;

/// The whole-number ids each judgement's count answers, in the order [`ResultView::counts`] and
/// [`crate::select::RecordRowView::counts`] both hold them.
///
/// The score screen answers them from the run that just ended and the browser from the best run
/// recorded on the focused chart, which is the same reading the reference implementation offers on
/// each screen.
const JUDGE_COUNT_IDS: [i32; JUDGEMENTS] = [NUMBER_PERFECT, NUMBER_GREAT, NUMBER_GOOD, NUMBER_BAD, NUMBER_POOR, NUMBER_MISS];

/// The rate id each judgement's share answers, paired with the count it is measured from.
///
/// There are five rather than six because the reference declares no rate for a miss: a missed note
/// is counted under `POOR`, which is where its share shows up too.
const JUDGE_RATE_IDS: [(i32, usize); 5] = [(RATE_PGREAT, 0), (RATE_GREAT, 1), (RATE_GOOD, 2), (RATE_BAD, 3), (RATE_POOR, 4)];

/// The first judgement option of each side, from which that side's other five follow in order.
///
/// rbms judges a double chart as one run over two fields rather than as two sides with a judgement
/// each, so both bands answer from the one judgement the run carries: a 10- or 14-key document that
/// puts a pop-up over each field shows that judgement on both. The reference numbers a third band
/// as well, which rbms has no player for.
const JUDGE_OPTION_BASES: [i32; 2] = [OPTION_1P_PERFECT, OPTION_2P_PERFECT];

/// The option ids that report the last hit landed early, one per side.
const JUDGE_EARLY_IDS: [i32; 2] = [OPTION_1P_EARLY, OPTION_2P_EARLY];

/// The option ids that report it landed late.
const JUDGE_LATE_IDS: [i32; 2] = [OPTION_1P_LATE, OPTION_2P_LATE];

/// The option ids that name the gauge as one that fills toward a clear, one per side.
///
/// rbms runs one gauge over a whole chart however many fields it is spread across, so both bands
/// answer from it: a 10- or 14-key document that labels the gauge over either field names the same
/// gauge, which is the one being played.
const GAUGE_GROOVE_IDS: [i32; 2] = [OPTION_GAUGE_GROOVE, OPTION_GAUGE_GROOVE_2P];

/// The option ids that name it as one that has to be survived.
const GAUGE_HARD_IDS: [i32; 2] = [OPTION_GAUGE_HARD, OPTION_GAUGE_HARD_2P];

/// The option ids that name it as one drained at the EX rate.
const GAUGE_EX_IDS: [i32; 2] = [OPTION_GAUGE_EX, OPTION_GAUGE_EX_2P];

/// The first gauge that is survived rather than filled, by the order the gauges are numbered in --
/// assist, easy, normal, hard, ex-hard, hazard, then the three course gauges. The first three are
/// cleared by reaching a threshold; everything from hard upward ends the run when it empties.
const FIRST_SURVIVAL_GAUGE: usize = 3;

/// The gauges drained at the EX rate, by the same numbering. Assist and easy drain that way at the
/// gentle end, ex-hard and hazard at the harsh end, and two of the three course gauges do as well,
/// which is why this is a list rather than a threshold (`BooleanPropertyFactory.gauge_ex`).
const EX_RATE_GAUGES: [usize; 6] = [0, 1, 4, 5, 7, 8];

/// How many DJ levels a run can reach, which is how long each band of rank option ids is.
const RANK_BAND_COUNT: i32 = 8;

/// Whether the rank option `asked` names the DJ level `ex` out of `max_ex` reaches, in the band that
/// starts at `first`.
///
/// Each band is numbered from its best level down (`first` is that band's AAA), while [`dj_rank`]
/// counts up from `F`, so the two are mirrored (`BooleanPropertyFactory.createNowRank`). Answers
/// `None` for an id outside the band, which is the caller's cue to keep looking.
fn rank_option(asked: i32, first: i32, ex: u32, max_ex: u32) -> Option<bool> {
    let step = asked - first;
    (0..RANK_BAND_COUNT).contains(&step).then(|| dj_rank(ex, max_ex) as i32 == RANK_BAND_COUNT - 1 - step)
}

/// Places a whole number keeps after the point when a document asks for the fraction separately.
const AFTERDOT_PLACES: f64 = 100.0;

/// Percent of a full gauge.
const GAUGE_FULL: f32 = 100.0;

/// EX points a single note is worth at its best judgement, which is what turns a note count into a
/// maximum score.
const POINTS_PER_NOTE: u32 = 2;

pub const RESULT_TEXT_SCORE: i32 = 20_001;
pub const RESULT_TEXT_COMBO: i32 = 20_002;
pub const RESULT_TEXT_NOTES: i32 = 20_003;
pub const RESULT_TEXT_CLEAR: i32 = 20_004;
pub const RESULT_TEXT_JUDGE_PERFECT: i32 = 20_005;
pub const RESULT_TEXT_JUDGE_GREAT: i32 = 20_006;
pub const RESULT_TEXT_JUDGE_GOOD: i32 = 20_007;
pub const RESULT_TEXT_JUDGE_BAD: i32 = 20_008;
pub const RESULT_TEXT_JUDGE_POOR: i32 = 20_009;
pub const RESULT_TEXT_JUDGE_MISS: i32 = 20_010;
pub const RESULT_TEXT_TARGET: i32 = 20_011;

const RESULT_TEXT_JUDGE_IDS: [i32; JUDGEMENTS] =
    [RESULT_TEXT_JUDGE_PERFECT, RESULT_TEXT_JUDGE_GREAT, RESULT_TEXT_JUDGE_GOOD, RESULT_TEXT_JUDGE_BAD, RESULT_TEXT_JUDGE_POOR, RESULT_TEXT_JUDGE_MISS];

const RESULT_JUDGE_LABELS: [&str; JUDGEMENTS] = ["PGREAT", "GREAT", "GOOD", "BAD", "POOR", "MISS"];

/// The first of the browser option panel's row-label ids, one per row in panel order.
pub const OPTION_ROW_LABEL_FIRST: i32 = 20_101;

/// The first of the option panel's row-value ids, one per row.
pub const OPTION_ROW_VALUE_FIRST: i32 = 20_121;

/// The first of the option panel's row-focus ids, one per row, answered as a boolean.
pub const OPTION_ROW_FOCUSED_FIRST: i32 = 20_141;

/// The name of whatever the run is being paced against, while it plays.
pub const PLAY_TEXT_TARGET_NAME: i32 = 20_201;

/// How far ahead of or behind that pace the run is, sign included.
pub const PLAY_TEXT_TARGET_DELTA: i32 = 20_202;

/// The browser's bottom-bar control hint.
pub const SELECT_TEXT_HINT: i32 = 20_203;

/// The score screen's bottom-bar control hint.
pub const RESULT_TEXT_HINT: i32 = 20_204;
/// The score server's status lines, joined into one.
pub const RESULT_TEXT_IR: i32 = 20_205;

/// The first of the browser detail pane's statistic cells, each `label value` as one string.
pub const SELECT_STAT_FIRST: i32 = 20_301;

/// How many statistic cells the detail pane carries.
pub const SELECT_STAT_COUNT: usize = 6;
/// The focused chart's local record, one line per id: best EX over its ceiling, its break count,
/// how often it was played and cleared, the lamp it holds, and when it was set.
pub const SELECT_RECORD_FIRST: i32 = 20_311;
pub const SELECT_RECORD_COUNT: usize = 5;

/// The whole part of a number a document splits across two objects.
fn whole(value: f64) -> i32 {
    value.trunc() as i32
}

/// Milliseconds in one minute and one second, for the clock a document splits into two numbers.
const MS_PER_MINUTE: i64 = 60_000;
const MS_PER_SECOND: i64 = 1_000;
const SECONDS_PER_MINUTE: i64 = 60;

/// The whole minutes a span of milliseconds holds.
fn minutes_of(ms: i64) -> i32 {
    (ms / MS_PER_MINUTE) as i32
}

/// The seconds left over once the whole minutes are taken out.
fn seconds_of(ms: i64) -> i32 {
    ((ms / MS_PER_SECOND) % SECONDS_PER_MINUTE) as i32
}

/// The fractional part of such a number, as the two places the reference's `AFTERDOT` ids carry.
fn afterdot(value: f64) -> i32 {
    ((value.abs().fract() * AFTERDOT_PLACES) as i32).clamp(0, AFTERDOT_PLACES as i32 - 1)
}

/// Which judgement one of the per-side judgement bands reports, or `None` when the id names no band.
fn judgement_of(id: i32) -> Option<(usize, usize)> {
    JUDGE_OPTION_BASES.iter().enumerate().find_map(|(side, base)| (id >= *base && id < base + JUDGEMENTS as i32).then(|| (side, (id - *base) as usize)))
}

/// Which side of a two-entry band `id` names, or `None` when it names neither.
fn band_of(id: i32, band: &[i32; 2]) -> Option<usize> {
    band.iter().position(|named| *named == id)
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
    /// The resolved field, for the cover offsets a document places its own lane covers with. `None`
    /// on a screen with no field running, which leaves all three to the player's own nudges.
    pub field: Option<&'a Skin>,
    /// How much of the field the player has covered and hidden.
    pub shade: LaneShade,
    /// Which field the last judgement landed in, `0` for the left-hand one and `1` for the right.
    pub judged_side: usize,
    /// Which gauge the run is being played on, by the order the gauges are numbered in, so a
    /// document can label the gauge it draws. Every gauge, including the course ones a gauge
    /// object's own cell table has no column for.
    pub gauge_kind: usize,
    /// The chart's artist and level, for the documents that show them while it plays.
    pub artist: &'a str,
    pub level: i32,
    /// The slowest, fastest and most common tempo the chart holds.
    pub bpm_min: f64,
    pub bpm_max: f64,
    pub bpm_main: f64,
    /// The EX the target would end on, when the run is paced against one.
    pub target_ex: Option<u32>,
    /// The signed distance from the target's pace, spelled out for a document's text object.
    pub target_delta: &'a str,
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

    /// Whether one of the per-side judgement bands names the field the last judgement landed in.
    ///
    /// rbms judges a double chart as one run over two fields rather than as two sides with a
    /// judgement each, so the one judgement the run carries is reported on the field it was played
    /// on and the other field stays quiet. A single-field run always answers the left-hand side.
    fn judged_on(&self, side: usize) -> bool {
        self.hud.last_judge.is_some() && side == usize::from(self.judged_side > 0)
    }

    /// The cover offsets the running field publishes, in the pixels the document was authored in.
    ///
    /// The reference measures all three off its own lane region (`LaneRenderer`): the lift is how
    /// far the judgement line has been raised, the lane cover pushes the band the player pulled down
    /// from the ceiling by that share of the visible field, and the hidden cover raises its band from
    /// the judgement line by its own share. A hidden band the player switched off is published as
    /// fully transparent rather than as no offset at all, which is how the reference hides the object
    /// carrying it.
    ///
    /// These answer before the player's own nudges, because a nudge moves an object around the field
    /// and these three say where the field itself is.
    fn cover_offset(&self, id: i32) -> Option<SkinOffset> {
        let field = self.field.filter(|field| field.lane_height() > 0.0)?;
        let height = field.lane_height();
        let flat = SkinOffset { x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, a: 0.0 };
        Some(match id {
            OFFSET_LIFT => SkinOffset { y: field.lift_height, ..flat },
            OFFSET_LANECOVER => SkinOffset { y: -height * self.shade.cover.clamp(0.0, 1.0), ..flat },
            OFFSET_HIDDEN_COVER if self.shade.hidden > 0.0 => SkinOffset { y: height * self.shade.hidden.min(1.0), ..flat },
            OFFSET_HIDDEN_COVER => SkinOffset { a: -f32::from(u8::MAX), ..flat },
            _ => return None,
        })
    }
}

impl OffsetSource for PlayViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.cover_offset(id).or_else(|| offset_of(self.offsets, id))
    }
}

impl DrawStateSource for PlayViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let asked = id.abs();
        let answer = match asked {
            OPTION_AUTOPLAYON => self.autoplay,
            OPTION_AUTOPLAYOFF => !self.autoplay,
            _ if let Some(side) = band_of(asked, &JUDGE_EARLY_IDS) => self.judged_on(side) && self.hud.last_fast,
            _ if let Some(side) = band_of(asked, &JUDGE_LATE_IDS) => self.judged_on(side) && !self.hud.last_fast,
            _ if let Some((side, index)) = judgement_of(asked) => self.judged_on(side) && self.hud.last_judge == Some(index as u8),
            _ if band_of(asked, &GAUGE_GROOVE_IDS).is_some() => self.gauge_kind < FIRST_SURVIVAL_GAUGE,
            _ if band_of(asked, &GAUGE_HARD_IDS).is_some() => self.gauge_kind >= FIRST_SURVIVAL_GAUGE,
            _ if band_of(asked, &GAUGE_EX_IDS).is_some() => EX_RATE_GAUGES.contains(&self.gauge_kind),
            _ => rank_option(asked, OPTION_NOW_AAA_1P, self.hud.ex_score, self.hud.max_ex).unwrap_or(UNMAPPED_BOOLEAN),
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
            NUMBER_MINBPM => whole(self.bpm_min),
            NUMBER_MAXBPM => whole(self.bpm_max),
            NUMBER_MAINBPM => whole(self.bpm_main),
            NUMBER_PLAYTIME_MINUTE => minutes_of(self.song_ms),
            NUMBER_PLAYTIME_SECOND => seconds_of(self.song_ms),
            NUMBER_TIMELEFT_MINUTE => minutes_of((self.duration_ms - self.song_ms).max(0)),
            NUMBER_TIMELEFT_SECOND => seconds_of((self.duration_ms - self.song_ms).max(0)),
            NUMBER_TOTALEARLY => lane_kind_total(self.hud.fast) as i32,
            NUMBER_TOTALLATE => lane_kind_total(self.hud.slow) as i32,
            NUMBER_PLAYLEVEL => self.level,
            NUMBER_TARGET_SCORE => self.target_ex.map_or(UNMAPPED_INTEGER, |ex| ex as i32),
            NUMBER_DIFF_TARGETSCORE => self.hud.pace.as_ref().map_or(0, |pace| pace.delta as i32),
            _ => UNMAPPED_INTEGER,
        }
    }

    fn float(&self, id: i32) -> f32 {
        if let Some((_, index)) = JUDGE_RATE_IDS.iter().find(|(rate, _)| *rate == id) {
            return share(f64::from(self.hud.counts[*index]), f64::from(self.total_notes()));
        }
        match id {
            RATE_EXSCORE | FLOAT_SCORE_RATE => share(f64::from(self.hud.ex_score), f64::from(self.hud.max_ex)),
            RATE_BESTSCORE => self.hud.best_ex.map_or(UNMAPPED_FLOAT, |best| share(f64::from(best), f64::from(self.hud.max_ex))),
            RATE_TARGETSCORE | FLOAT_TARGET_RATE => self.target_ex.map_or(UNMAPPED_FLOAT, |ex| share(f64::from(ex), f64::from(self.hud.max_ex))),
            FLOAT_GROOVEGAUGE_1P => clamp_float(self.hud.gauge / GAUGE_FULL),
            RATE_MUSIC_PROGRESS | RATE_MUSIC_PROGRESS_BAR => share(self.song_ms as f64, self.duration_ms as f64),
            _ => UNMAPPED_FLOAT,
        }
    }

    fn string(&self, id: i32) -> &str {
        match id {
            STRING_TITLE | STRING_FULLTITLE => self.title,
            STRING_ARTIST | STRING_FULLARTIST => self.artist,
            PLAY_TEXT_TARGET_NAME => self.hud.pace.as_ref().map_or(UNMAPPED_STRING, |pace| pace.name),
            PLAY_TEXT_TARGET_DELTA => self.target_delta,
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
    /// The option panel's rows, when the browser has them to show. A frame that carries none answers
    /// every option id as an unmapped one, which is what a document drawing no panel reads.
    pub options: Option<&'a OptionsRows<'a>>,
    stats: Vec<String>,
    records: [String; SELECT_RECORD_COUNT],
}

impl<'a> SelectViewState<'a> {
    /// The browser's state for one frame, with the focused chart's statistics and record spelled out
    /// for the document's text objects.
    pub fn new(view: &'a SelectView, now_ms: i64, offsets: Offsets<'a>, options: Option<&'a OptionsRows<'a>>) -> SelectViewState<'a> {
        let song = match &view.detail {
            SelectDetail::Song(detail) => Some(detail),
            _ => None,
        };
        let stats = song.map_or_else(Vec::new, |song| song.stats.iter().map(|cell| format!("{}  {}", cell.label, cell.value)).collect());
        let records = song.map_or_else(Default::default, |song| {
            let best = song.records.best.as_ref();
            [
                best.map_or_else(String::new, |best| format!("EX  {} / {}", best.ex, best.max_ex)),
                best.map_or_else(String::new, |best| format!("BP  {}", best.bp)),
                format!("PLAYS  {}   CLEARS  {}", song.records.plays, song.records.clears),
                best.map_or_else(String::new, |best| best.lamp_label.to_owned()),
                best.map_or_else(String::new, |best| best.when.clone()),
            ]
        });
        SelectViewState { view, now_ms, offsets, options, stats, records }
    }
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

    /// Which option row one of the panel's private id bands names, counted from `first`.
    fn option_row(id: i32, first: i32) -> Option<usize> {
        (id >= first && id < first + OPTIONS_ROW_COUNT as i32).then(|| (id - first) as usize)
    }

    /// The best run recorded on the focused chart, when a chart is focused and has been played.
    fn best(&self) -> Option<&crate::select::RecordRowView> {
        self.song().and_then(|song| song.records.best.as_ref())
    }
}

impl OffsetSource for SelectViewState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        offset_of(self.offsets, id)
    }
}

impl DrawStateSource for SelectViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let asked = id.abs();
        let answer = match asked {
            OPTION_FOLDERBAR => matches!(self.view.detail, SelectDetail::Folder { .. }),
            OPTION_SONGBAR => self.song().is_some(),
            OPTION_PANEL1 => self.options.is_some_and(|rows| rows.open),
            _ => match Self::option_row(asked, OPTION_ROW_FOCUSED_FIRST) {
                Some(row) => self.options.is_some_and(|rows| rows.focused == row),
                None => UNMAPPED_BOOLEAN,
            },
        };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for SelectViewState<'_> {
    fn integer(&self, id: i32) -> i32 {
        if let Some(judgement) = JUDGE_COUNT_IDS.iter().position(|count| *count == id) {
            return self.best().map_or(0, |best| best.counts[judgement] as i32);
        }
        match id {
            NUMBER_PLAYLEVEL => self.song().and_then(|song| song.level.parse().ok()).unwrap_or(UNMAPPED_INTEGER),
            NUMBER_MAXCOMBO => self.best().map_or(0, |best| best.max_combo as i32),
            _ => UNMAPPED_INTEGER,
        }
    }

    fn float(&self, id: i32) -> f32 {
        match id {
            RATE_MUSICSELECT_POSITION if !self.view.rows.is_empty() => clamp_float(self.view.sel as f32 / (self.view.rows.len() - 1).max(1) as f32),
            RATE_BESTSCORE => {
                self.song().and_then(|song| song.records.best.as_ref()).map_or(UNMAPPED_FLOAT, |best| share(f64::from(best.ex), f64::from(best.max_ex)))
            }
            _ => UNMAPPED_FLOAT,
        }
    }

    fn string(&self, id: i32) -> &str {
        if let Some(row) = Self::option_row(id, OPTION_ROW_LABEL_FIRST) {
            return self.options.map_or(UNMAPPED_STRING, |rows| rows.labels[row]);
        }
        if let Some(row) = Self::option_row(id, OPTION_ROW_VALUE_FIRST) {
            return self.options.map_or(UNMAPPED_STRING, |rows| rows.values[row].as_str());
        }
        if (SELECT_STAT_FIRST..SELECT_STAT_FIRST + SELECT_STAT_COUNT as i32).contains(&id) {
            return self.stats.get((id - SELECT_STAT_FIRST) as usize).map_or(UNMAPPED_STRING, String::as_str);
        }
        if (SELECT_RECORD_FIRST..SELECT_RECORD_FIRST + SELECT_RECORD_COUNT as i32).contains(&id) {
            return &self.records[(id - SELECT_RECORD_FIRST) as usize];
        }
        match id {
            STRING_DIRECTORY => &self.view.header,
            SELECT_TEXT_HINT => self.view.guide,
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
    score: String,
    combo: String,
    notes: String,
    clear: String,
    judges: [String; JUDGEMENTS],
    target_text: String,
    hint: String,
    ir_status: String,
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

impl<'a> ResultViewState<'a> {
    pub fn new(view: &'a ResultView, target: Option<&'a TargetView>, cleared: bool, now_ms: i64, offsets: Offsets<'a>) -> ResultViewState<'a> {
        let judges = std::array::from_fn(|index| format!("{}  {}", RESULT_JUDGE_LABELS[index], view.counts[index]));
        let target_text = target.map_or_else(String::new, |target| {
            let (delta, _) = ex_delta_label(i64::from(view.ex_score) - i64::from(target.ex));
            format!("{}  {}  {}", target.name, target.ex, delta)
        });
        ResultViewState {
            view,
            target,
            cleared,
            now_ms,
            offsets,
            score: format!("{} / {}", view.ex_score, view.max_score),
            combo: format!("{} / {}", view.max_combo, view.total_notes),
            notes: view.total_notes.to_string(),
            clear: view.clear_label.to_string(),
            judges,
            target_text,
            hint: String::new(),
            ir_status: String::new(),
        }
    }

    /// The same state with the key hint the built-in screen would show, for a document that draws it.
    pub fn with_hint(mut self, run_again: bool) -> Self {
        self.hint = crate::result::result_hint_text(run_again).to_owned();
        self
    }

    /// The same state with the score server's status, for a document that shows it.
    pub fn with_ir_status(mut self, text: String) -> Self {
        self.ir_status = text;
        self
    }
}

impl DrawStateSource for ResultViewState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let asked = id.abs();
        let answer = match asked {
            OPTION_RESULT_CLEAR => self.cleared,
            OPTION_RESULT_FAIL => !self.cleared,
            _ => rank_option(asked, OPTION_RESULT_AAA_1P, self.view.ex_score, self.view.max_score)
                .or_else(|| rank_option(asked, OPTION_NOW_AAA_1P, self.view.ex_score, self.view.max_score))
                .unwrap_or(UNMAPPED_BOOLEAN),
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
            STRING_ARTIST | STRING_FULLARTIST => &self.view.artist,
            RESULT_TEXT_SCORE => &self.score,
            RESULT_TEXT_COMBO => &self.combo,
            RESULT_TEXT_NOTES => &self.notes,
            RESULT_TEXT_CLEAR => &self.clear,
            RESULT_TEXT_TARGET => &self.target_text,
            RESULT_TEXT_HINT => &self.hint,
            RESULT_TEXT_IR => &self.ir_status,
            _ if let Some(index) = RESULT_TEXT_JUDGE_IDS.iter().position(|value| *value == id) => &self.judges[index],
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
    /// The rest of what the loading screen knows about the chart it is bringing in, empty while it
    /// waits for something that is not a chart.
    pub chart: DecideChart<'a>,
    pub now_ms: i64,
    pub offsets: Offsets<'a>,
}

/// What the loading screen can say about the chart it is loading before the chart itself is read.
#[derive(Debug, Clone, Copy, Default)]
pub struct DecideChart<'a> {
    pub subtitle: &'a str,
    pub artist: &'a str,
    pub genre: &'a str,
    pub level: i32,
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
            NUMBER_PLAYLEVEL => self.chart.level,
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
            STRING_SUBTITLE => self.chart.subtitle,
            STRING_ARTIST | STRING_FULLARTIST => self.chart.artist,
            STRING_GENRE => self.chart.genre,
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

/// Which native action one of a document's rectangles takes the place of.
///
/// The browser answers a click by hit-testing its own rows and buttons; a document that draws those
/// itself has to say where they ended up, which is what a hotspot is. The variants are the built-in
/// [`crate::select::SelectHot`] actions a document may stand in for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinHotAction {
    Search,
    Sort,
    Folders,
    Tables,
    Records,
    Settings,
    ModalReplay,
    ModalClose,
    /// One chart of the browser's own list, by the row index the browser selects with -- not the
    /// wheel slot the document happened to draw it on.
    Row(usize),
    /// An object the document gave an `act` to, which stands for no native action at all: what a
    /// click on it does is the document's own business, and the player only carries out what is
    /// left of it ([`super::events::DocumentEvents::click`]).
    Event(SkinEventClick),
}

impl SkinHotAction {
    /// The action one of a document's `hotspot` entries names, or `None` when it is not one this
    /// build takes.
    ///
    /// The loader drops the names it does not know while it reads the document
    /// ([`rbms_skin::loader::HOTSPOT_ACTIONS`]), so nothing that loaded cleanly reaches the `None`.
    /// A wheel slot is not here because no document names one: it is the wheel's own geometry.
    pub(crate) fn from_name(name: &str) -> Option<SkinHotAction> {
        Some(match name {
            "search" => SkinHotAction::Search,
            "sort" => SkinHotAction::Sort,
            "folders" => SkinHotAction::Folders,
            "tables" => SkinHotAction::Tables,
            "records" => SkinHotAction::Records,
            "settings" => SkinHotAction::Settings,
            "modal-replay" => SkinHotAction::ModalReplay,
            "modal-close" => SkinHotAction::ModalClose,
            _ => return None,
        })
    }
}

/// One clickable rectangle a document offers, in the document's own coordinates.
///
/// Document space rather than screen space because a draw list knows the size it was authored at
/// and not the canvas it will land on: the caller maps the rectangle onto the canvas with the same
/// [`super::SkinViewport`] the frame was drawn with, which also turns it the right way up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinHotspot {
    /// Document coordinates as [`super::SkinScreen::hotspots`] answers them, and screen coordinates
    /// once [`super::SkinScreen::hotspots_on_screen`] has placed them on the canvas.
    pub rect: Rect,
    pub action: SkinHotAction,
}

/// The play screen's per-frame state that no property id can carry.
///
/// A note, a cover and a hit-error strip each need a whole series rather than one number, and the
/// property registry answers scalars. These arrive beside the scalar source instead, so the
/// registry keeps the shape every existing document was written against.
pub struct PlayObjectState<'a> {
    /// The resolved lane geometry, which a document replaces rather than reads when it draws the
    /// field itself.
    pub field: &'a Skin,
    pub playfield: &'a PlayfieldView<'a>,
    pub shade: LaneShade,
    /// Which gauge is in play, as the gauge object's cell table is indexed.
    pub gauge_kind: usize,
    /// The key bombs still burning, as `(started at, lane)`.
    pub bomb: &'a [(i64, u8)],
    /// Which lanes are held, in lane order.
    pub keys_down: &'a [bool],
    /// Recent hits as `(error in milliseconds, judgement)`, most recent last.
    pub recent_hits: &'a [(i64, u8)],
}

/// The option panel's rows, as the browser's document draws them.
pub struct OptionsRows<'a> {
    pub labels: [&'a str; OPTIONS_ROW_COUNT],
    pub values: [String; OPTIONS_ROW_COUNT],
    pub focused: usize,
    pub open: bool,
}

/// The browser's per-frame state that no property id can carry: the rows themselves.
pub struct SelectListState<'a> {
    pub rows: &'a [SelectRow],
    pub sel: usize,
    pub detail: &'a SelectDetail,
    pub options: Option<&'a OptionsRows<'a>>,
}

/// The score screen's per-frame series, for the objects that draw a run rather than a number.
pub struct ResultSeriesState<'a> {
    /// The gauge at each sample of the run, in the percent the views carry it as.
    pub gauge_series: &'a [f32],
    /// How many hits landed in each timing bucket.
    pub timing_hist: &'a [u32],
    pub judge_dist: &'a [u32; JUDGEMENTS],
    /// The tempo timeline as `(progress through the chart, bpm)`.
    pub bpm_points: &'a [(f32, f64)],
}

/// The screen-shaped state a frame carries beside its scalar property source.
///
/// [`FrameExtra::None`] is what every existing caller passes and what a screen with nothing extra to
/// say keeps passing, so a document that draws only scalar objects is unaffected by any of this.
#[derive(Default, Clone, Copy)]
pub enum FrameExtra<'a> {
    #[default]
    None,
    Play(&'a PlayObjectState<'a>),
    Select(&'a SelectListState<'a>),
    Result(&'a ResultSeriesState<'a>),
}

impl FrameExtra<'_> {
    /// The play state, when this frame is a play frame.
    pub fn play(&self) -> Option<&PlayObjectState<'_>> {
        match self {
            Self::Play(state) => Some(state),
            _ => None,
        }
    }

    /// The browser state, when this frame is a browser frame.
    pub fn select(&self) -> Option<&SelectListState<'_>> {
        match self {
            Self::Select(state) => Some(state),
            _ => None,
        }
    }

    /// The score series, when this frame is a score frame.
    pub fn result(&self) -> Option<&ResultSeriesState<'_>> {
        match self {
            Self::Result(state) => Some(state),
            _ => None,
        }
    }
}
