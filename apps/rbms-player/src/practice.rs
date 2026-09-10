//! Practice mode: the property one chart is practised under, the panel that edits it, and the
//! rule that keeps a practice run out of the records.
//!
//! A practice run plays one slice of a chart — everything before `start_ms` is skipped and the run
//! ends the moment `end_ms` goes by — from a gauge the player picked rather than the one the chart
//! would hand out, and an emptied gauge does not end it. That makes it a rehearsal rather than a
//! play, so decision 12 keeps it out of the score book and off the IR: [`practice_block_reason`]
//! is the single predicate both gates ask.
//!
//! The editing rules are the reference implementation's, taken from `PracticeConfiguration.java`
//! element by element: the step sizes (plain, turbo, turbo on an analog control), the rounding of
//! the two time bounds, and the pop'n rule that caps the starting gauge once a survival gauge is
//! selected. They are reproduced exactly, including the asymmetry that only an increase of START
//! TIME pushes END TIME along with it.
//!
//! The property is remembered per chart in `practice.ron`, keyed by chart md5 — the key the score
//! book and the favourites already use, so a rescan or a moved file keeps the settings. The
//! reference keys the same file by SHA-256; the key differs, the contents do not.
//!
//! Every item here is reached from the screens the integration branch wires up, so until those
//! call sites land the module carries its own `dead_code` allowance, as its sibling branches do.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rbms_chart::shuffle::NoteOption;
use rbms_judge::gauge::GaugeIndex;
use rbms_judge::gauge_tables::{GaugeSetId, element_of};
use rbms_model::{Mode, Model};
use serde::{Deserialize, Serialize};

use crate::format::gauge_name;
use crate::notify::{Level, notify};
use crate::write_atomic;

/// Name of the file the per-chart practice properties are kept in, next to `scores.ron`.
pub(crate) const PRACTICE_FILE: &str = "practice.ron";

/// What [`crate::ir_submission_block_reason`] reports for a practice run. Decision 12 keeps such a
/// run out of the records, and this is the wording the result screen shows for it.
pub(crate) const PRACTICE_BLOCK_REASON: &str = "practice";

/// Milliseconds per second, the unit the reference edits and displays practice times in.
const MS_PER_SECOND: i32 = 1_000;
/// Milliseconds per minute, for the `mm:ss.d` readout.
const MS_PER_MINUTE: i32 = 60_000;
/// Microseconds per millisecond: the property is authored in ms, the play clock runs in us.
const US_PER_MS: i64 = 1_000;

/// Step both time bounds move by on a plain press (`PracticeConfiguration.java:536,550`).
const TIME_STEP_MS: i32 = 100;
/// Step they move by while turbo is held on a digital control (`:536`).
const TIME_STEP_TURBO_MS: i32 = 2_500;
/// Step they move by while turbo is held on an analog control, which reports far more often (`:536`).
const TIME_STEP_TURBO_ANALOG_MS: i32 = 1_000;
/// Granularity both bounds are rounded down to (`:533,547,548`).
const TIME_ROUND_MS: i32 = 100;
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
/// GAUGE VALUE step on a plain press and while turbo is held (`:580`).
const GAUGE_VALUE_STEP: i32 = 1;
const GAUGE_VALUE_STEP_TURBO: i32 = 10;
/// First survival gauge. On a pop'n chart, selecting this or anything past it caps the starting
/// gauge (`:565`).
const PMS_SURVIVAL_GAUGE_INDEX: u8 = 3;
/// The cap that rule applies (`:566`).
const PMS_START_GAUGE_CAP: i32 = 100;

/// JUDGERANK bounds and steps (`:589-595`).
const JUDGE_RATE_MIN: i32 = 1;
const JUDGE_RATE_MAX: i32 = 400;
const JUDGE_RATE_STEP: i32 = 1;
const JUDGE_RATE_STEP_TURBO: i32 = 25;
/// JUDGERANK of a chart played as authored (`:686`).
const JUDGE_RATE_UNMODIFIED: i32 = 100;

/// TOTAL bounds and steps, digital and analog (`:597-603`).
const TOTAL_MIN: f64 = 10.0;
const TOTAL_MAX: f64 = 5_000.0;
const TOTAL_STEP: f64 = 5.0;
const TOTAL_STEP_TURBO: f64 = 25.0;
const TOTAL_STEP_ANALOG: f64 = 1.0;
const TOTAL_STEP_ANALOG_TURBO: f64 = 20.0;
/// TOTAL that means "take the chart's own" (`:70`).
const TOTAL_FROM_CHART: f64 = 0.0;

/// FREQUENCY bounds and steps (`:605-611`).
const FREQ_MIN: i32 = 50;
const FREQ_MAX: i32 = 200;
const FREQ_STEP: i32 = 5;
const FREQ_STEP_TURBO: i32 = 25;
const FREQ_STEP_ANALOG: i32 = 1;
const FREQ_STEP_ANALOG_TURBO: i32 = 10;
/// Playback speed of an unaltered run, and the only speed this engine can actually play at.
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

/// Step the two time bounds move by for this press (`PracticeConfiguration.java:536`). Turbo on an
/// analog control moves less per press because such a control reports many presses per turn.
fn time_step_ms(turbo: bool, analog: bool) -> i32 {
    match (turbo, analog) {
        (true, true) => TIME_STEP_TURBO_ANALOG_MS,
        (true, false) => TIME_STEP_TURBO_MS,
        (false, _) => TIME_STEP_MS,
    }
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

/// `mm:ss.d`, the readout the reference's two time rows use
/// (`PracticeConfiguration.java:543-544`).
pub(crate) fn format_practice_time(ms: i32) -> String {
    format!("{:2}:{:02}.{:1}", ms / MS_PER_MINUTE, (ms / MS_PER_SECOND) % 60, (ms / TIME_ROUND_MS) % 10)
}

/// Everything one chart is practised under, remembered between visits.
///
/// `total` of [`TOTAL_FROM_CHART`] means the chart's own TOTAL, which is how the reference spells
/// "unset" (`PracticeConfiguration.java:70`). `freq` is edited and displayed but a run always plays
/// at [`FREQ_UNMODIFIED`] — see [`PracticeSession::freq_percent`].
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
        let max_start = round_down_to(last_timeline_ms - START_TIME_TAIL_MS, TIME_ROUND_MS);
        self.start_ms = self.start_ms.clamp(START_TIME_MIN_MS, max_start.max(START_TIME_MIN_MS));
        let max_end = round_down_to(last_timeline_ms + END_TIME_TAIL_MS, TIME_ROUND_MS);
        let min_end = round_down_to(self.start_ms + MIN_PLAY_SPAN_MS, TIME_ROUND_MS);
        self.end_ms = self.end_ms.clamp(min_end, max_end.max(min_end));
        let gauge_max = element_of(self.gauge_set(), self.gauge_index()).max as i32;
        self.start_gauge = self.start_gauge.clamp(GAUGE_VALUE_MIN, gauge_max.max(GAUGE_VALUE_MIN));
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

/// Move START TIME one step (`PracticeConfiguration.java:531-543`).
///
/// Only an increase drags END TIME along, and that drag is a plain `max` with no rounding — the
/// reference rounds the two bounds where it derives them, not where it pushes one with the other.
pub(crate) fn clamp_start(p: &mut PracticeProperty, last_tl_ms: i32, turbo: bool, analog: bool, inc: bool) {
    let max_start = round_down_to(last_tl_ms - START_TIME_TAIL_MS, TIME_ROUND_MS);
    let change = time_step_ms(turbo, analog);
    if inc {
        p.start_ms = (p.start_ms + change).min(max_start);
        p.end_ms = p.end_ms.max(p.start_ms + MIN_PLAY_SPAN_MS);
    } else {
        p.start_ms = (p.start_ms - change).max(START_TIME_MIN_MS);
    }
}

/// Move END TIME one step (`PracticeConfiguration.java:545-557`). Both of its bounds are rounded
/// down, the lower one off the current START TIME.
pub(crate) fn clamp_end(p: &mut PracticeProperty, last_tl_ms: i32, turbo: bool, analog: bool, inc: bool) {
    let max_end = round_down_to(last_tl_ms + END_TIME_TAIL_MS, TIME_ROUND_MS);
    let min_end = round_down_to(p.start_ms + MIN_PLAY_SPAN_MS, TIME_ROUND_MS);
    let change = time_step_ms(turbo, analog);
    if inc {
        p.end_ms = (p.end_ms + change).min(max_end);
    } else {
        p.end_ms = (p.end_ms - change).max(min_end);
    }
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

/// Move GAUGE VALUE one step (`PracticeConfiguration.java:580-587`). Turbo from the very bottom
/// jumps to one whole step rather than to `1 + step`, which is what makes the bottom of the range
/// reachable in both directions.
pub(crate) fn adjust_gauge_value(p: &mut PracticeProperty, turbo: bool, inc: bool) {
    let change = if turbo { GAUGE_VALUE_STEP_TURBO } else { GAUGE_VALUE_STEP };
    let max = (element_of(p.gauge_set(), p.gauge_index()).max as i32).max(GAUGE_VALUE_MIN);
    if inc && turbo && p.start_gauge == GAUGE_VALUE_MIN {
        p.start_gauge = change.min(max);
    } else {
        p.start_gauge = (p.start_gauge + if inc { change } else { -change }).clamp(GAUGE_VALUE_MIN, max);
    }
}

/// Move JUDGERANK one step (`PracticeConfiguration.java:588-596`), with the same bottom-of-range
/// turbo jump as GAUGE VALUE.
pub(crate) fn adjust_judge_rate(p: &mut PracticeProperty, turbo: bool, inc: bool) {
    let change = if turbo { JUDGE_RATE_STEP_TURBO } else { JUDGE_RATE_STEP };
    if inc && turbo && p.judge_rate == JUDGE_RATE_MIN {
        p.judge_rate = change.min(JUDGE_RATE_MAX);
    } else {
        p.judge_rate = (p.judge_rate + if inc { change } else { -change }).clamp(JUDGE_RATE_MIN, JUDGE_RATE_MAX);
    }
}

/// Move TOTAL one step (`PracticeConfiguration.java:597-603`).
pub(crate) fn adjust_total(p: &mut PracticeProperty, turbo: bool, analog: bool, inc: bool) {
    let change = match (turbo, analog) {
        (true, true) => TOTAL_STEP_ANALOG_TURBO,
        (true, false) => TOTAL_STEP_TURBO,
        (false, true) => TOTAL_STEP_ANALOG,
        (false, false) => TOTAL_STEP,
    };
    p.total = (p.total + if inc { change } else { -change }).clamp(TOTAL_MIN, TOTAL_MAX);
}

/// Move FREQUENCY one step (`PracticeConfiguration.java:604-611`).
pub(crate) fn adjust_freq(p: &mut PracticeProperty, turbo: bool, analog: bool, inc: bool) {
    let change = match (turbo, analog) {
        (true, true) => FREQ_STEP_ANALOG_TURBO,
        (true, false) => FREQ_STEP_TURBO,
        (false, true) => FREQ_STEP_ANALOG,
        (false, false) => FREQ_STEP,
    };
    p.freq = (p.freq + if inc { change } else { -change }).clamp(FREQ_MIN, FREQ_MAX);
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

    /// Playback speed the run actually plays at. The property carries the player's choice, but the
    /// mixer resamples nothing, so every run plays at [`FREQ_UNMODIFIED`] until it can.
    pub(crate) fn freq_percent(&self) -> i32 {
        FREQ_UNMODIFIED
    }
}

/// The practice screen: the property being edited, the chart it belongs to, and which half of the
/// screen is up.
#[derive(Debug, Clone)]
pub(crate) struct PracticePanel {
    /// The property under edit, saved back under [`PracticePanel::chart_key`] when the screen closes.
    pub(crate) property: PracticeProperty,
    /// Chart md5 the property is remembered under.
    chart_key: String,
    /// Mode of the chart, which decides the pop'n cap and the shuffle list.
    mode: Mode,
    /// Time of the chart's last timeline, which both time bounds are measured against.
    last_timeline_ms: i32,
    /// Focused row.
    cursor: usize,
    /// Panel or a running slice.
    phase: PracticePhase,
}

impl PracticePanel {
    /// Open the panel on a chart, starting from the property last saved for it.
    pub(crate) fn new(chart_key: String, mode: Mode, last_timeline_ms: i32, saved: Option<PracticeProperty>, chart_total: f64) -> PracticePanel {
        let mut property = saved.unwrap_or_else(|| PracticeProperty::for_chart(mode, last_timeline_ms));
        property.sanitise(mode, last_timeline_ms);
        if property.total == TOTAL_FROM_CHART {
            property.total = chart_total.clamp(TOTAL_MIN, TOTAL_MAX);
        }
        PracticePanel { property, chart_key, mode, last_timeline_ms, cursor: 0, phase: PracticePhase::Panel }
    }

    /// Chart md5 this panel edits.
    pub(crate) fn chart_key(&self) -> &str {
        &self.chart_key
    }

    /// Which half of the screen is up.
    pub(crate) fn phase(&self) -> PracticePhase {
        self.phase
    }

    /// Focused row.
    pub(crate) fn focused(&self) -> PracticeElement {
        PracticeElement::ALL[self.cursor.min(PracticeElement::ALL.len() - 1)]
    }

    /// Move the focus, wrapping at both ends so the list is a ring like every other panel here.
    pub(crate) fn move_cursor(&mut self, down: bool) {
        let count = PracticeElement::ALL.len();
        let step = if down { 1 } else { count - 1 };
        self.cursor = (self.cursor + step) % count;
    }

    /// Edit the focused row one step. `analog` says the press came from a control that reports
    /// continuously, which shortens the turbo step of the rows that take one.
    pub(crate) fn adjust(&mut self, inc: bool, turbo: bool, analog: bool) {
        match self.focused() {
            PracticeElement::StartTime => clamp_start(&mut self.property, self.last_timeline_ms, turbo, analog, inc),
            PracticeElement::EndTime => clamp_end(&mut self.property, self.last_timeline_ms, turbo, analog, inc),
            PracticeElement::GaugeType => cycle_gauge_type(&mut self.property, self.mode, inc),
            PracticeElement::GaugeCategory => cycle_gauge_set(&mut self.property, inc),
            PracticeElement::GaugeValue => adjust_gauge_value(&mut self.property, turbo, inc),
            PracticeElement::JudgeRank => adjust_judge_rate(&mut self.property, turbo, inc),
            PracticeElement::Total => adjust_total(&mut self.property, turbo, analog, inc),
            PracticeElement::Freq => adjust_freq(&mut self.property, turbo, analog, inc),
            PracticeElement::Random => cycle_option(&mut self.property, self.mode, inc),
        }
    }

    /// Value drawn on the right of a row.
    pub(crate) fn value_text(&self, element: PracticeElement) -> String {
        let p = &self.property;
        match element {
            PracticeElement::StartTime => format_practice_time(p.start_ms),
            PracticeElement::EndTime => format_practice_time(p.end_ms),
            PracticeElement::GaugeType => gauge_index_name(p.gauge_index()).to_string(),
            PracticeElement::GaugeCategory => gauge_set_token(p.gauge_set()).to_string(),
            PracticeElement::GaugeValue => p.start_gauge.to_string(),
            PracticeElement::JudgeRank => p.judge_rate.to_string(),
            PracticeElement::Total => (p.total as i32).to_string(),
            PracticeElement::Freq => p.freq.to_string(),
            PracticeElement::Random => p.option().label().to_string(),
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
    pub(crate) fn len(&self) -> usize {
        self.charts.len()
    }

    /// Whether nothing has been practised yet.
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
