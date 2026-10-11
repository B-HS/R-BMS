//! Practice mode: the property one chart is practised under, the panel that edits it, and the
//! rule that keeps a practice run out of the records.
//!
//! A practice run plays one slice of a chart — everything before `start_ms` is skipped and the run
//! ends the moment `end_ms` goes by — from a gauge the player picked rather than the one the chart
//! would hand out, and an emptied gauge does not end it. That makes it a rehearsal rather than a
//! play, so decision 12 keeps it out of the score book and off the IR: [`practice_block_reason`]
//! is the single predicate both gates ask.
//!
//! What a row may hold is the reference implementation's, taken from `PracticeConfiguration.java`
//! element by element: the two ends of every number, the rounding of the two time bounds, the
//! shortest slice they may enclose, and the pop'n rule that caps the starting gauge once a survival
//! gauge is selected -- including the asymmetry that only an increase of START TIME pushes END TIME
//! along with it.
//!
//! How a number is moved is not the reference's. Its panel is worked from the keys and the
//! turntable of a controller, in steps sized for that (a tenth of a second, five of TOTAL, five
//! percent of the speed, and coarser ones with a turbo key held). This panel is worked from the
//! keyboard and the mouse, and a row that holds a number is set as such a row is on every built-in
//! screen ([`crate::number_row`]): counted in whole steps of the smallest change it shows
//! ([`PracticePanel::number`]) and put on a step by a key, a slider or a typed figure
//! ([`PracticePanel::set_number`]). Whatever step it is put on, it is held to what the row may
//! hold, so a number that runs into another -- an END TIME brought down onto START TIME -- stops at
//! the nearest value it may have. A START TIME dragged along its slider pushes END TIME only while
//! it is past it ([`PracticePanel::drag_number`]), where the reference's keys push it for good.
//!
//! The property is remembered per chart in `practice.ron`, keyed by chart md5 — the key the score
//! book and the favourites already use, so a rescan or a moved file keeps the settings. The
//! reference keys the same file by SHA-256; the key differs, the contents do not.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rbms_chart::shuffle::NoteOption;
use rbms_config::{NO_UNIT, NumberSpec};
use rbms_judge::gauge::GaugeIndex;
use rbms_judge::gauge_tables::{GaugeSetId, element_of};
use rbms_model::{Mode, Model};
use serde::{Deserialize, Serialize};

use crate::format::gauge_name;
use crate::notify::{Level, notify};
use crate::number_row::NumberValue;
use crate::write_atomic;

/// Name of the file the per-chart practice properties are kept in, next to `scores.ron`.
pub(crate) const PRACTICE_FILE: &str = "practice.ron";

/// What [`crate::ir_submission_block_reason`] reports for a practice run. Decision 12 keeps such a
/// run out of the records, and this is the wording the result screen shows for it.
pub(crate) const PRACTICE_BLOCK_REASON: &str = "practice";

/// Milliseconds per second, the unit the reference edits and displays practice times in.
const MS_PER_SECOND: i32 = 1_000;
/// Microseconds per millisecond: the property is authored in ms, the play clock runs in us.
const US_PER_MS: i64 = 1_000;

/// Granularity both bounds are rounded down to (`PracticeConfiguration.java:533,547,548`) and
/// shown to (`:543-544`), which makes it the smallest change either of them takes.
const TIME_ROUND_MS: i32 = 100;
/// How many of those make one second, and how many places after the point that is written to.
const TIME_STEPS_PER_SECOND: i64 = (MS_PER_SECOND / TIME_ROUND_MS) as i64;
const TIME_DECIMALS: u8 = 1;
/// Earliest START TIME (`:534`).
const START_TIME_MIN_MS: i32 = 0;
/// How far short of the last timeline START TIME may reach (`:533`).
const START_TIME_TAIL_MS: i32 = 2_000;
/// How far past the last timeline END TIME may reach (`:547`).
const END_TIME_TAIL_MS: i32 = 1_000;
/// Shortest slice the two bounds may enclose (`:539,548`).
const MIN_PLAY_SPAN_MS: i32 = 1_000;

/// Gauge index the reference starts a practice on: NORMAL (`:662`).
const DEFAULT_GAUGE_TYPE: u8 = 2;
/// Starting gauge amount before a chart is loaded (`:666`).
const DEFAULT_START_GAUGE: i32 = 20;
/// Lowest starting gauge the panel offers (`:583`).
const GAUGE_VALUE_MIN: i32 = 1;
/// First survival gauge. On a pop'n chart, selecting this or anything past it caps the starting
/// gauge (`:565`).
const PMS_SURVIVAL_GAUGE_INDEX: u8 = 3;
/// The cap that rule applies (`:566`).
const PMS_START_GAUGE_CAP: i32 = 100;

/// JUDGERANK bounds (`:589-595`).
const JUDGE_RATE_MIN: i32 = 1;
const JUDGE_RATE_MAX: i32 = 400;
/// JUDGERANK of a chart played as authored (`:686`).
const JUDGE_RATE_UNMODIFIED: i32 = 100;

/// TOTAL bounds (`:597-603`).
const TOTAL_MIN: f64 = 10.0;
const TOTAL_MAX: f64 = 5_000.0;
/// TOTAL that means "take the chart's own" (`:70`).
const TOTAL_FROM_CHART: f64 = 0.0;
/// Where the slider of TOTAL and its arrows end. A chart's own TOTAL is some hundreds, so a slider
/// run over the whole of what the row may hold would have every chart's in its first few pixels;
/// past this the row is typed into.
const TOTAL_SLIDER_MAX: i64 = 1_000;

/// FREQUENCY bounds (`:605-611`).
const FREQ_MIN: i32 = 50;
const FREQ_MAX: i32 = 200;
const FREQ_UNMODIFIED: i32 = 100;

/// Whether a practice run may end because the gauge emptied. It may not: the point of practising a
/// slice is to reach its end.
const PRACTICE_GAUGE_LOCK: bool = true;

/// Round down to a multiple of `base`, the reference's `roundDownTo`
/// (`PracticeConfiguration.java:520`). Java's integer division truncates toward zero and so does
/// Rust's, so a bound that lands before zero — a chart shorter than [`START_TIME_TAIL_MS`] — rounds
/// the same way in both.
fn round_down_to(value: i32, base: i32) -> i32 {
    value / base * base
}

/// The latest START TIME of a chart whose last timeline is at `last_tl_ms`
/// (`PracticeConfiguration.java:533`). A chart shorter than [`START_TIME_TAIL_MS`] starts at its
/// beginning and nowhere else: the reference works out a ceiling before zero for one and lets START
/// TIME follow it there, which asks for a slice that begins before the chart does.
fn max_start_ms(last_tl_ms: i32) -> i32 {
    round_down_to(last_tl_ms - START_TIME_TAIL_MS, TIME_ROUND_MS).max(START_TIME_MIN_MS)
}

/// The latest END TIME of a chart whose last timeline is at `last_tl_ms` (`:547`).
fn max_end_ms(last_tl_ms: i32) -> i32 {
    round_down_to(last_tl_ms + END_TIME_TAIL_MS, TIME_ROUND_MS)
}

/// The earliest END TIME of a slice that starts at `start_ms` (`:548`).
fn min_end_ms(start_ms: i32) -> i32 {
    round_down_to(start_ms + MIN_PLAY_SPAN_MS, TIME_ROUND_MS)
}

/// The most the gauge a property names can start at: the top of that gauge on its table (`:582`).
fn max_start_gauge(p: &PracticeProperty) -> i32 {
    (element_of(p.gauge_set(), p.gauge_index()).max as i32).max(GAUGE_VALUE_MIN)
}

/// The step of a time row a time in milliseconds stands on, counted in [`TIME_ROUND_MS`]. A time
/// between two steps stands on the earlier one, which is how it is shown.
fn time_steps(ms: i32) -> i64 {
    i64::from(ms / TIME_ROUND_MS)
}

/// The time in milliseconds a step of a time row stands for.
fn time_of_steps(steps: i64) -> i32 {
    i32::try_from(steps.saturating_mul(i64::from(TIME_ROUND_MS))).unwrap_or(if steps < 0 { i32::MIN } else { i32::MAX })
}

/// How a time row is counted between `min_ms` and `max_ms`: in tenths of a second, written as
/// minutes and seconds (`:543-544`).
fn time_spec(min_ms: i32, max_ms: i32) -> NumberSpec {
    NumberSpec::fine(time_steps(min_ms), time_steps(max_ms), TIME_STEPS_PER_SECOND, TIME_DECIMALS, NO_UNIT).as_clock()
}

/// How a row of whole numbers from `min` to `max` is counted.
fn whole_spec(min: i32, max: i32) -> NumberSpec {
    NumberSpec::whole(i64::from(min), i64::from(max), NO_UNIT)
}

/// A step of a row of whole numbers as the value it stands for, held to what a property stores.
fn value_of_steps(steps: i64) -> i32 {
    i32::try_from(steps).unwrap_or(if steps < 0 { i32::MIN } else { i32::MAX })
}

/// The gauge tables, as tokens stable enough to write to a file.
const GAUGE_SET_TOKENS: [(GaugeSetId, &str); GaugeSetId::COUNT] =
    [(GaugeSetId::FiveKeys, "5KEYS"), (GaugeSetId::SevenKeys, "7KEYS"), (GaugeSetId::Pms, "PMS"), (GaugeSetId::Keyboard, "KEYBOARD"), (GaugeSetId::Lr2, "LR2")];

/// Token this gauge table is written under.
pub(crate) fn gauge_set_token(set: GaugeSetId) -> &'static str {
    GAUGE_SET_TOKENS.iter().find(|(id, _)| *id == set).map(|(_, token)| *token).unwrap_or(GAUGE_SET_TOKENS[0].1)
}

/// The gauge table a token names, or `None` when the file holds something this build does not know.
pub(crate) fn gauge_set_from_token(token: &str) -> Option<GaugeSetId> {
    GAUGE_SET_TOKENS.iter().find(|(_, name)| *name == token).map(|(id, _)| *id)
}

/// Name of one of the nine gauges. The six playable ones reuse the browser's own spelling; the
/// three course gauges are named as the reference's practice panel names them
/// (`PracticeConfiguration.java:31-32`).
pub(crate) fn gauge_index_name(index: GaugeIndex) -> &'static str {
    match index.kind() {
        Some(kind) => gauge_name(kind),
        None => match index {
            GaugeIndex::Class => "GRADE",
            GaugeIndex::ExClass => "EX GRADE",
            _ => "EXHARD GRADE",
        },
    }
}

/// Gauge the property's index names, falling back to the default when a file carries an index this
/// build has no gauge for.
fn gauge_index_of(gauge_type: u8) -> GaugeIndex {
    GaugeIndex::from_index(usize::from(gauge_type)).unwrap_or(GaugeIndex::Normal)
}

/// Whether this chart is played on the pop'n gauge table, which is what the starting-gauge cap
/// keys off (`PracticeConfiguration.java:565`).
fn is_pms(mode: Mode) -> bool {
    GaugeSetId::for_mode(&mode) == GaugeSetId::Pms
}

/// The shuffles this mode may practise under. A mode with no scratch lane cannot play ALL-SCRATCH,
/// which is the same restriction the reference puts on its pop'n option list
/// (`PracticeConfiguration.java:615`).
fn options_for(mode: Mode) -> &'static [NoteOption] {
    match mode.scratch.is_empty() {
        true => &NoteOption::ALL[..NoteOption::ALL.len() - 1],
        false => &NoteOption::ALL,
    }
}

/// Time of the chart's last timeline in milliseconds, which is what both time bounds are measured
/// against (`PracticeConfiguration.java:533,547`).
pub(crate) fn last_timeline_ms(model: &Model) -> i32 {
    let last_us = model.timelines.last().map_or(0, |tl| tl.time_us);
    i32::try_from(last_us / US_PER_MS).unwrap_or(i32::MAX)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PracticeProperty {
    /// First millisecond of the slice.
    pub(crate) start_ms: i32,
    /// Millisecond the slice ends at.
    pub(crate) end_ms: i32,
    /// Index into the nine gauges, `0` being ASSIST EASY.
    pub(crate) gauge_type: u8,
    /// Gauge table the run is played on, as a [`gauge_set_token`].
    pub(crate) gauge_set: String,
    /// Gauge amount the slice starts at.
    pub(crate) start_gauge: i32,
    /// JUDGE WIDTH percentage, the same axis the judge settings use.
    pub(crate) judge_rate: i32,
    /// Playback speed percentage.
    pub(crate) freq: i32,
    /// TOTAL override, or [`TOTAL_FROM_CHART`] for the chart's own.
    pub(crate) total: f64,
    /// Note shuffle, as a [`NoteOption::label`].
    pub(crate) random: String,
}

impl Default for PracticeProperty {
    fn default() -> PracticeProperty {
        PracticeProperty {
            start_ms: START_TIME_MIN_MS,
            end_ms: START_TIME_MIN_MS + MIN_PLAY_SPAN_MS,
            gauge_type: DEFAULT_GAUGE_TYPE,
            gauge_set: gauge_set_token(GaugeSetId::default()).to_string(),
            start_gauge: DEFAULT_START_GAUGE,
            judge_rate: JUDGE_RATE_UNMODIFIED,
            freq: FREQ_UNMODIFIED,
            total: TOTAL_FROM_CHART,
            random: NoteOption::Off.label().to_string(),
        }
    }
}

impl PracticeProperty {
    /// The property a chart is practised under the first time, before any saved one is read: the
    /// whole chart, and the gauge table its mode plays on (`PracticeConfiguration.java:55-57,69-71`).
    ///
    /// The reference seeds its JUDGERANK from the chart's own `#RANK`-derived judgerank; here the
    /// same row is a percentage of the chart's own windows, so the unaltered value is
    /// [`JUDGE_RATE_UNMODIFIED`] whatever the chart states.
    pub(crate) fn for_chart(mode: Mode, last_timeline_ms: i32) -> PracticeProperty {
        PracticeProperty {
            end_ms: last_timeline_ms + END_TIME_TAIL_MS,
            gauge_set: gauge_set_token(GaugeSetId::for_mode(&mode)).to_string(),
            ..PracticeProperty::default()
        }
    }

    /// The gauge table this property names.
    pub(crate) fn gauge_set(&self) -> GaugeSetId {
        gauge_set_from_token(&self.gauge_set).unwrap_or_default()
    }

    /// The gauge this property names.
    pub(crate) fn gauge_index(&self) -> GaugeIndex {
        gauge_index_of(self.gauge_type)
    }

    /// The shuffle this property names.
    pub(crate) fn option(&self) -> NoteOption {
        NoteOption::from_str(&self.random)
    }

    /// Pull a property read from a file back inside the bounds this chart allows, so a chart that
    /// got shorter — or a hand-edited file — cannot ask for a slice that does not exist.
    pub(crate) fn sanitise(&mut self, mode: Mode, last_timeline_ms: i32) {
        if gauge_set_from_token(&self.gauge_set).is_none() {
            self.gauge_set = gauge_set_token(GaugeSetId::for_mode(&mode)).to_string();
        }
        if usize::from(self.gauge_type) >= GaugeIndex::COUNT {
            self.gauge_type = DEFAULT_GAUGE_TYPE;
        }
        if !options_for(mode).contains(&self.option()) {
            self.random = NoteOption::Off.label().to_string();
        }
        self.start_ms = self.start_ms.clamp(START_TIME_MIN_MS, max_start_ms(last_timeline_ms));
        let min_end = min_end_ms(self.start_ms);
        self.end_ms = self.end_ms.clamp(min_end, max_end_ms(last_timeline_ms).max(min_end));
        self.start_gauge = self.start_gauge.clamp(GAUGE_VALUE_MIN, max_start_gauge(self));
        cap_pms_start_gauge(self, mode);
        self.judge_rate = self.judge_rate.clamp(JUDGE_RATE_MIN, JUDGE_RATE_MAX);
        self.freq = self.freq.clamp(FREQ_MIN, FREQ_MAX);
        if self.total != TOTAL_FROM_CHART {
            self.total = self.total.clamp(TOTAL_MIN, TOTAL_MAX);
        }
    }
}

/// Apply the pop'n cap: once a survival gauge is selected on a pop'n chart the starting gauge may
/// not exceed [`PMS_START_GAUGE_CAP`] (`PracticeConfiguration.java:564-567`).
fn cap_pms_start_gauge(p: &mut PracticeProperty, mode: Mode) {
    if is_pms(mode) && p.gauge_type >= PMS_SURVIVAL_GAUGE_INDEX && p.start_gauge > PMS_START_GAUGE_CAP {
        p.start_gauge = PMS_START_GAUGE_CAP;
    }
}

/// Put START TIME on `start_ms`, held between the beginning of the chart and the latest a slice may
/// start (`PracticeConfiguration.java:531-543`).
///
/// Only an increase drags END TIME along, and that drag is a plain `max` with no rounding -- the
/// reference rounds the two bounds where it derives them, not where it pushes one with the other.
pub(crate) fn set_start_ms(p: &mut PracticeProperty, last_tl_ms: i32, start_ms: i32) {
    let before = p.start_ms;
    p.start_ms = start_ms.clamp(START_TIME_MIN_MS, max_start_ms(last_tl_ms));
    if p.start_ms > before {
        p.end_ms = p.end_ms.max(p.start_ms + MIN_PLAY_SPAN_MS);
    }
}

/// Put START TIME on `start_ms` while its slider is being dragged, END TIME having stood on
/// `end_before_ms` when the slider was taken hold of.
///
/// A drag passes over every place between where it began and where it is let go of, and a pointer
/// that overshoots is brought back. So END TIME is not pushed for good by a START TIME that is only
/// passing: for as long as the slider is held it stands where it stood, or a slice's shortest span
/// past START TIME when that is later, and comes back as START TIME does. What it is left on when
/// the slider is let go of is what a key or a typed figure putting START TIME there would leave.
pub(crate) fn drag_start_ms(p: &mut PracticeProperty, last_tl_ms: i32, start_ms: i32, end_before_ms: i32) {
    p.start_ms = start_ms.clamp(START_TIME_MIN_MS, max_start_ms(last_tl_ms));
    p.end_ms = end_before_ms.max(p.start_ms + MIN_PLAY_SPAN_MS);
}

/// Put END TIME on `end_ms`, held between the shortest slice START TIME leaves room for and the
/// latest a slice may end (`PracticeConfiguration.java:545-557`). Both of those are rounded down,
/// the lower one off the current START TIME.
pub(crate) fn set_end_ms(p: &mut PracticeProperty, last_tl_ms: i32, end_ms: i32) {
    let min_end = min_end_ms(p.start_ms);
    p.end_ms = end_ms.clamp(min_end, max_end_ms(last_tl_ms).max(min_end));
}

/// Cycle the gauge, wrapping both ways, and apply the pop'n cap the new selection may trigger
/// (`PracticeConfiguration.java:559-568`).
pub(crate) fn cycle_gauge_type(p: &mut PracticeProperty, mode: Mode, inc: bool) {
    let count = u8::try_from(GaugeIndex::COUNT).unwrap_or(u8::MAX);
    let step = if inc { 1 } else { count - 1 };
    p.gauge_type = ((p.gauge_type % count) + step) % count;
    cap_pms_start_gauge(p, mode);
}

/// Cycle the gauge table, resetting the starting gauge to the new table's own initial value
/// (`PracticeConfiguration.java:569-579`).
pub(crate) fn cycle_gauge_set(p: &mut PracticeProperty, inc: bool) {
    let count = GaugeSetId::COUNT;
    let step = if inc { 1 } else { count - 1 };
    let next = GaugeSetId::ALL[(p.gauge_set().index() + step) % count];
    p.gauge_set = gauge_set_token(next).to_string();
    p.start_gauge = element_of(next, p.gauge_index()).init as i32;
}

/// Put GAUGE VALUE on `start_gauge`, held between one and the top of the selected gauge
/// (`PracticeConfiguration.java:580-587`).
pub(crate) fn set_start_gauge(p: &mut PracticeProperty, start_gauge: i32) {
    p.start_gauge = start_gauge.clamp(GAUGE_VALUE_MIN, max_start_gauge(p));
}

/// Put JUDGERANK on `judge_rate`, held to its two ends (`PracticeConfiguration.java:588-596`).
pub(crate) fn set_judge_rate(p: &mut PracticeProperty, judge_rate: i32) {
    p.judge_rate = judge_rate.clamp(JUDGE_RATE_MIN, JUDGE_RATE_MAX);
}

/// Put TOTAL on `total`, held to its two ends (`PracticeConfiguration.java:597-603`).
pub(crate) fn set_total(p: &mut PracticeProperty, total: f64) {
    p.total = total.clamp(TOTAL_MIN, TOTAL_MAX);
}

/// Put FREQUENCY on `freq`, held to its two ends (`PracticeConfiguration.java:604-611`).
pub(crate) fn set_freq(p: &mut PracticeProperty, freq: i32) {
    p.freq = freq.clamp(FREQ_MIN, FREQ_MAX);
}

/// Cycle the note shuffle over the ones this mode may use
/// (`PracticeConfiguration.java:612-616`).
pub(crate) fn cycle_option(p: &mut PracticeProperty, mode: Mode, inc: bool) {
    let options = options_for(mode);
    let count = options.len();
    let at = options.iter().position(|o| *o == p.option()).unwrap_or(0);
    let step = if inc { 1 } else { count - 1 };
    p.random = options[(at + step) % count].label().to_string();
}

/// One editable row of the practice panel. The reference's OPTION-2P, OPTION-DP and GRAPHTYPE rows
/// are absent: this engine applies one shuffle to the whole chart and draws one result graph, so
/// those rows would edit values nothing reads.
///
/// Six of the rows hold a number ([`PracticePanel::number`]) and three a choice out of a ring
/// ([`PracticePanel::cycle`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PracticeElement {
    StartTime,
    EndTime,
    GaugeType,
    GaugeCategory,
    GaugeValue,
    JudgeRank,
    Total,
    Freq,
    Random,
}

impl PracticeElement {
    /// Every row, top to bottom, in the reference's own order
    /// (`PracticeConfiguration.java:531-620`).
    pub(crate) const ALL: [PracticeElement; 9] = [
        PracticeElement::StartTime,
        PracticeElement::EndTime,
        PracticeElement::GaugeType,
        PracticeElement::GaugeCategory,
        PracticeElement::GaugeValue,
        PracticeElement::JudgeRank,
        PracticeElement::Total,
        PracticeElement::Freq,
        PracticeElement::Random,
    ];

    /// Label drawn on the left of the row.
    pub(crate) fn label(self) -> &'static str {
        match self {
            PracticeElement::StartTime => "START TIME",
            PracticeElement::EndTime => "END TIME",
            PracticeElement::GaugeType => "GAUGE TYPE",
            PracticeElement::GaugeCategory => "GAUGE CATEGORY",
            PracticeElement::GaugeValue => "GAUGE VALUE",
            PracticeElement::JudgeRank => "JUDGERANK",
            PracticeElement::Total => "TOTAL",
            PracticeElement::Freq => "FREQUENCY",
            PracticeElement::Random => "OPTION-1P",
        }
    }
}

/// Which half of the practice screen the player is on.
///
/// A practice run always comes back here: reaching `end_ms`, giving up, or emptying the gauge all
/// return to [`PracticePhase::Panel`] with the property as it was, so the next attempt starts from
/// the same slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PracticePhase {
    /// Editing the property.
    Panel,
    /// A slice is being played.
    Playing,
}

/// What the panel hands the play screen when a slice starts.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PracticeSession {
    /// First microsecond of the slice. Notes and keysounds before it are skipped.
    pub(crate) start_us: i64,
    /// Microsecond the slice ends at.
    pub(crate) end_us: i64,
    /// Gauge to play on.
    pub(crate) gauge: GaugeIndex,
    /// Gauge table it is taken from.
    pub(crate) gauge_set: GaugeSetId,
    /// Value the gauge starts at, in the same units the gauge itself carries.
    pub(crate) start_gauge: f32,
    /// JUDGE WIDTH percentage for the run.
    pub(crate) judge_rate_percent: i32,
    pub(crate) freq_percent: i32,
    /// TOTAL override, or `None` for the chart's own.
    pub(crate) total: Option<f64>,
    /// Shuffle the chart is laid out with.
    pub(crate) option: NoteOption,
    /// Whether an emptied gauge leaves the run running. Always true — that is what makes it
    /// practice rather than a play.
    pub(crate) gauge_locked: bool,
}

impl PracticeSession {
    /// Whether the slice is over at this song time, which is the practice run's own end condition:
    /// it stops here rather than at the last note.
    pub(crate) fn is_past_end(&self, song_us: i64) -> bool {
        song_us >= self.end_us
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PracticeClock {
    start_us: i64,
    freq_percent: i32,
}

impl PracticeClock {
    pub(crate) const fn normal() -> PracticeClock {
        PracticeClock { start_us: 0, freq_percent: FREQ_UNMODIFIED }
    }

    pub(crate) fn new(start_us: i64, freq_percent: i32) -> PracticeClock {
        PracticeClock { start_us, freq_percent: freq_percent.clamp(FREQ_MIN, FREQ_MAX) }
    }

    pub(crate) fn from_session(session: &PracticeSession) -> PracticeClock {
        PracticeClock::new(session.start_us, session.freq_percent)
    }

    pub(crate) fn chart_time_us(self, engine_elapsed_us: i64) -> i64 {
        self.start_us.saturating_add(scale_time_us(engine_elapsed_us, i64::from(self.freq_percent), i64::from(FREQ_UNMODIFIED)))
    }

    pub(crate) fn engine_time_us(self, chart_time_us: i64) -> i64 {
        scale_time_us(chart_time_us.saturating_sub(self.start_us), i64::from(FREQ_UNMODIFIED), i64::from(self.freq_percent))
    }

    pub(crate) fn pitch(self, pitch: f32) -> f32 {
        pitch * self.freq_percent as f32 / FREQ_UNMODIFIED as f32
    }
}

fn scale_time_us(value: i64, numerator: i64, denominator: i64) -> i64 {
    let scaled = i128::from(value) * i128::from(numerator) / i128::from(denominator);
    scaled.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// The practice screen's model: the property being edited, the chart it belongs to, and which half
/// of the screen is up.
#[derive(Debug, Clone)]
pub(crate) struct PracticePanel {
    /// The property under edit, saved back under [`PracticePanel::chart_key`] when the screen closes.
    pub(crate) property: PracticeProperty,
    /// The property this chart is practised under the first time, which is what the key that
    /// resets a row puts the row back to.
    fresh: PracticeProperty,
    /// Chart md5 the property is remembered under.
    chart_key: String,
    /// Mode of the chart, which decides the pop'n cap and the shuffle list.
    mode: Mode,
    /// Time of the chart's last timeline, which both time bounds are measured against.
    last_timeline_ms: i32,
    /// Panel or a running slice.
    phase: PracticePhase,
}

impl PracticePanel {
    /// Open the panel on a chart, starting from the property last saved for it.
    pub(crate) fn new(chart_key: String, mode: Mode, last_timeline_ms: i32, saved: Option<PracticeProperty>, chart_total: f64) -> PracticePanel {
        let mut fresh = PracticeProperty::for_chart(mode, last_timeline_ms);
        fresh.sanitise(mode, last_timeline_ms);
        fresh.total = chart_total.clamp(TOTAL_MIN, TOTAL_MAX);
        let mut property = saved.unwrap_or_else(|| fresh.clone());
        property.sanitise(mode, last_timeline_ms);
        if property.total == TOTAL_FROM_CHART {
            property.total = fresh.total;
        }
        PracticePanel { property, fresh, chart_key, mode, last_timeline_ms, phase: PracticePhase::Panel }
    }

    /// Chart md5 this panel edits.
    pub(crate) fn chart_key(&self) -> &str {
        &self.chart_key
    }

    /// Which half of the screen is up.
    #[cfg(test)]
    pub(crate) fn phase(&self) -> PracticePhase {
        self.phase
    }

    /// The number one row holds: how the row is counted, the step it stands on and the step a
    /// fresh panel on this chart has it on. `None` for a row that holds a choice.
    ///
    /// The ends of a row are what the chart and the other rows leave it. START TIME ends two
    /// seconds short of the chart's last timeline and END TIME a second past it; GAUGE VALUE ends at
    /// the top of the gauge that is selected, on the table that is selected. END TIME is counted
    /// from the earliest a slice can end at all, so its slider keeps its scale while START TIME
    /// moves, and is held above START TIME where it is set ([`PracticePanel::set_number`]).
    ///
    /// The slider of TOTAL and its arrows end at [`TOTAL_SLIDER_MAX`], short of what the row may
    /// hold, so that a place along the slider is a TOTAL a chart could have; a figure typed into the
    /// row reaches the rest.
    pub(crate) fn number(&self, element: PracticeElement) -> Option<NumberValue> {
        let (p, fresh) = (&self.property, &self.fresh);
        let (spec, steps, shipped) = match element {
            PracticeElement::StartTime => {
                (time_spec(START_TIME_MIN_MS, max_start_ms(self.last_timeline_ms)), time_steps(p.start_ms), time_steps(fresh.start_ms))
            }
            PracticeElement::EndTime => {
                (time_spec(min_end_ms(START_TIME_MIN_MS), max_end_ms(self.last_timeline_ms)), time_steps(p.end_ms), time_steps(fresh.end_ms))
            }
            PracticeElement::GaugeValue => (whole_spec(GAUGE_VALUE_MIN, max_start_gauge(p)), i64::from(p.start_gauge), i64::from(fresh.start_gauge)),
            PracticeElement::JudgeRank => (whole_spec(JUDGE_RATE_MIN, JUDGE_RATE_MAX), i64::from(p.judge_rate), i64::from(fresh.judge_rate)),
            PracticeElement::Total => (whole_spec(TOTAL_MIN as i32, TOTAL_MAX as i32), p.total as i64, fresh.total as i64),
            PracticeElement::Freq => (whole_spec(FREQ_MIN, FREQ_MAX), i64::from(p.freq), i64::from(fresh.freq)),
            PracticeElement::GaugeType | PracticeElement::GaugeCategory | PracticeElement::Random => return None,
        };
        let value = NumberValue::of(spec, steps, spec.clamp(shipped));
        Some(if element == PracticeElement::Total { value.with_slider_to(TOTAL_SLIDER_MAX) } else { value })
    }

    /// Put one row that holds a number on the step `steps`, held to what the row may hold now, and
    /// answer whether that changed the property. A row that holds a choice is left as it is.
    ///
    /// A row asked for the step it already stands on is not written: what it holds may be finer
    /// than its steps -- a chart's own TOTAL with a fraction, a time from a file edited by hand --
    /// and a slider that was only pressed, or a figure typed as it stood, must not round it.
    pub(crate) fn set_number(&mut self, element: PracticeElement, steps: i64) -> bool {
        if self.number(element).is_some_and(|number| number.steps == steps) {
            return false;
        }
        let before = self.property.clone();
        let p = &mut self.property;
        match element {
            PracticeElement::StartTime => set_start_ms(p, self.last_timeline_ms, time_of_steps(steps)),
            PracticeElement::EndTime => set_end_ms(p, self.last_timeline_ms, time_of_steps(steps)),
            PracticeElement::GaugeValue => set_start_gauge(p, value_of_steps(steps)),
            PracticeElement::JudgeRank => set_judge_rate(p, value_of_steps(steps)),
            PracticeElement::Total => set_total(p, steps as f64),
            PracticeElement::Freq => set_freq(p, value_of_steps(steps)),
            PracticeElement::GaugeType | PracticeElement::GaugeCategory | PracticeElement::Random => {}
        }
        self.property != before
    }

    /// Carry one row that holds a number to the step `steps` along its slider, `taken` being the
    /// property as it was when the slider was taken hold of, and answer whether that changed the
    /// property. It is [`PracticePanel::set_number`], but for what a row pushes out of its way: a
    /// START TIME carried past END TIME pushes it only for as long as it is past it
    /// ([`drag_start_ms`]).
    pub(crate) fn drag_number(&mut self, element: PracticeElement, steps: i64, taken: &PracticeProperty) -> bool {
        if element != PracticeElement::StartTime {
            return self.set_number(element, steps);
        }
        if self.number(element).is_some_and(|number| number.steps == steps) {
            return false;
        }
        let before = self.property.clone();
        drag_start_ms(&mut self.property, self.last_timeline_ms, time_of_steps(steps), taken.end_ms);
        self.property != before
    }

    /// Put one row that holds a number back to what a fresh panel on this chart has it on, held to
    /// what the row may hold now, and answer whether that changed the property. TOTAL goes back to
    /// the chart's own to the fraction, which a step of the row cannot say.
    pub(crate) fn reset_number(&mut self, element: PracticeElement) -> bool {
        let before = self.property.clone();
        let (p, fresh) = (&mut self.property, &self.fresh);
        match element {
            PracticeElement::StartTime => set_start_ms(p, self.last_timeline_ms, fresh.start_ms),
            PracticeElement::EndTime => set_end_ms(p, self.last_timeline_ms, fresh.end_ms),
            PracticeElement::GaugeValue => set_start_gauge(p, fresh.start_gauge),
            PracticeElement::JudgeRank => set_judge_rate(p, fresh.judge_rate),
            PracticeElement::Total => set_total(p, fresh.total),
            PracticeElement::Freq => set_freq(p, fresh.freq),
            PracticeElement::GaugeType | PracticeElement::GaugeCategory | PracticeElement::Random => {}
        }
        self.property != before
    }

    /// Move one row that holds a choice to the next of its ring, or to the one before, and answer
    /// whether the row is such a row. A row that holds a number is left as it is.
    pub(crate) fn cycle(&mut self, element: PracticeElement, inc: bool) -> bool {
        match element {
            PracticeElement::GaugeType => cycle_gauge_type(&mut self.property, self.mode, inc),
            PracticeElement::GaugeCategory => cycle_gauge_set(&mut self.property, inc),
            PracticeElement::Random => cycle_option(&mut self.property, self.mode, inc),
            PracticeElement::StartTime
            | PracticeElement::EndTime
            | PracticeElement::GaugeValue
            | PracticeElement::JudgeRank
            | PracticeElement::Total
            | PracticeElement::Freq => return false,
        }
        true
    }

    /// Value drawn on the right of a row: the choice by name, or the number as its row writes it.
    pub(crate) fn value_text(&self, element: PracticeElement) -> String {
        let p = &self.property;
        match element {
            PracticeElement::GaugeType => gauge_index_name(p.gauge_index()).to_string(),
            PracticeElement::GaugeCategory => gauge_set_token(p.gauge_set()).to_string(),
            PracticeElement::Random => p.option().label().to_string(),
            PracticeElement::StartTime
            | PracticeElement::EndTime
            | PracticeElement::GaugeValue
            | PracticeElement::JudgeRank
            | PracticeElement::Total
            | PracticeElement::Freq => self.number(element).map(|number| number.spec.text(number.steps)).unwrap_or_default(),
        }
    }

    /// Start the slice, and report what the play screen needs to set it up.
    pub(crate) fn start(&mut self) -> PracticeSession {
        self.phase = PracticePhase::Playing;
        PracticeSession {
            start_us: i64::from(self.property.start_ms) * US_PER_MS,
            end_us: i64::from(self.property.end_ms) * US_PER_MS,
            gauge: self.property.gauge_index(),
            gauge_set: self.property.gauge_set(),
            start_gauge: self.property.start_gauge as f32,
            judge_rate_percent: self.property.judge_rate,
            freq_percent: self.property.freq,
            total: match self.property.total {
                TOTAL_FROM_CHART => None,
                total => Some(total),
            },
            option: self.property.option(),
            gauge_locked: PRACTICE_GAUGE_LOCK,
        }
    }

    /// Come back from a slice with the property untouched, however the slice ended.
    pub(crate) fn finish(&mut self) {
        self.phase = PracticePhase::Panel;
    }
}

/// Why a run may not be recorded or submitted, when that run is a practice one. Decision 12: a
/// practice run rehearses a slice under a gauge the player chose, so it is neither a score nor a
/// submission. `None` when the run is not a practice one, so the caller can chain it with the
/// other reasons.
pub(crate) fn practice_block_reason(practice: bool) -> Option<&'static str> {
    match practice {
        true => Some(PRACTICE_BLOCK_REASON),
        false => None,
    }
}

/// The practice properties of every chart that has one, keyed by chart md5.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PracticeBook {
    /// Ordered so the file reads the same twice running.
    charts: BTreeMap<String, PracticeProperty>,
}

impl PracticeBook {
    /// Read the book at `path`. A file that is not there yet is an empty book — the normal state of
    /// a fresh install — and one that cannot be read is reported and treated the same, so a corrupt
    /// file never stops practice from opening.
    pub(crate) fn load(path: &Path) -> PracticeBook {
        let Ok(text) = std::fs::read_to_string(path) else {
            return PracticeBook::default();
        };
        match ron::from_str::<PracticeBook>(&text) {
            Ok(book) => book,
            Err(e) => {
                notify(Level::Warn, format!("practice settings not loaded ({e}); starting with none"));
                PracticeBook::default()
            }
        }
    }

    /// Write the book to `path`, reporting a failed write rather than losing it silently.
    pub(crate) fn save(&self, path: &Path) {
        let text = match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(text) => text,
            Err(e) => {
                notify(Level::Error, format!("practice settings save failed: {e}"));
                return;
            }
        };
        if let Err(e) = write_atomic(path, &text) {
            notify(Level::Error, format!("practice settings save failed ({}): {e}", path.display()));
        }
    }

    /// The property saved for this chart, if any.
    pub(crate) fn get(&self, chart_key: &str) -> Option<PracticeProperty> {
        self.charts.get(chart_key).cloned()
    }

    /// Remember this chart's property, replacing the one it had.
    pub(crate) fn put(&mut self, chart_key: &str, property: PracticeProperty) {
        self.charts.insert(chart_key.to_string(), property);
    }

    /// How many charts have been practised, which is what the settings screen reports.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.charts.len()
    }

    /// Whether nothing has been practised yet.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.charts.is_empty()
    }
}

/// Where the practice properties live: beside the settings file, like the favourites and the
/// scores.
pub(crate) fn practice_path(settings_path: &Path) -> PathBuf {
    settings_path.parent().map(|dir| dir.join(PRACTICE_FILE)).unwrap_or_else(|| PathBuf::from(PRACTICE_FILE))
}

#[cfg(test)]
mod tests;
