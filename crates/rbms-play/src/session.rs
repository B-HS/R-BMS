//! One run of a chart, driven headlessly.
//!
//! [`PlaySession`] owns everything that only exists while a chart is being played: the judge
//! [`Player`], the replay being reproduced, the inputs being recorded, the judge offset as the run
//! moves it, the replay-analysis clock, the recent judgement errors and the BGA timeline. Sound
//! leaves through the [`SoundSink`] trait, so this crate stays free of any audio backend and the
//! whole run can be reproduced against [`NullSink`] with no device present.

use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeAutoShift, GaugeIndex};
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;
use rbms_judge::matcher::{ScratchDir, UNMODIFIED_PLAYBACK_PERCENT};
use rbms_judge::windows::JudgeWindowRule;
use rbms_judge::{ClearType, GaugeKind, Judge, JudgeEngine, JudgeResult};
use rbms_model::{Model, TimeLine};
use rbms_store::{Replay, ReplayEvent, ReplayOffsetStep};

use crate::record::{GAUGE_LOG_INTERVAL_MS, GaugeLogRecorder, PlayRecord, ScoreProgress, play_time_ms};
use crate::{JUDGE_WIDTH_TIER_COUNT, PlayEvent, PlaySource, Player, UNMODIFIED_JUDGE_RATES, UNMODIFIED_RATE_PERCENT};

/// Neutral per-voice parameters for a chart keysound: reference level, centred, unpitched. The
/// balance between keysounds and accompaniment belongs on the output buses, not on the voice.
pub const KEYSOUND_GAIN: f32 = 1.0;
/// Stereo position of a chart keysound.
pub const KEYSOUND_PAN: f32 = 0.0;
/// Playback rate of a chart keysound.
pub const KEYSOUND_PITCH: f32 = 1.0;

/// How long the run keeps going past the last note before the result screen is due.
const PLAY_TAIL_US: i64 = 2_000_000;

/// How many judged inputs the analysis overlay keeps.
const TIMING_MARK_CAPACITY: usize = 16;

/// Inputs the recording makes room for, per judged object of the chart, when a run played by hand
/// starts: the press and the release that take the object, and as many again for presses that hit
/// nothing. A run inside that never grows the recording, and so never moves it, in the middle of
/// play, where growing it on a long chart is a copy of a megabyte or more on the input that does it.
const RECORDED_INPUTS_PER_NOTE: usize = 4;

/// Slowest and fastest analysis playback rates, and the step the controls move by.
pub const ANALYSIS_RATE_MIN: f64 = 0.25;
/// Fastest analysis playback rate.
pub const ANALYSIS_RATE_MAX: f64 = 4.0;
/// How much one rate step changes the analysis playback rate.
pub const ANALYSIS_RATE_STEP: f64 = 0.25;
/// How far one analysis seek moves the virtual clock.
pub const ANALYSIS_SEEK_STEP_US: i64 = 2_000_000;

/// How far apart on the song clock the run's gauge is sampled: the reference's half-second
/// history (`BMSPlayer.java:628-633`), which [`GAUGE_SAMPLE_INTERVAL_US`] is a whole multiple of.
const GAUGE_SAMPLE_STEP_US: i64 = GAUGE_LOG_INTERVAL_MS * US_PER_MS;

const _: () = assert!(GAUGE_SAMPLE_INTERVAL_US % GAUGE_SAMPLE_STEP_US == 0, "the built-in series is sampled on the gauge log's own instants");

/// Worst judgement an input may earn and still move a judge offset that adjusts itself: GOOD
/// (`JudgeManager.java:700`, `judge <= 2`).
const AUTO_ADJUST_WORST_JUDGE: usize = 2;
/// Widest timing error an input may have and still move the offset (`JudgeManager.java:700`).
const AUTO_ADJUST_MAX_ABS_DELTA_US: i64 = 150_000;
/// The timing error one millisecond of offset answers (`JudgeManager.java:701` divides by 30000).
const AUTO_ADJUST_US_PER_STEP: i64 = 30_000;
/// What the error is pushed away from zero by before that division, which rounds the quotient to
/// the nearest whole millisecond (`mfast + 15000`, `mfast - 15000`).
const AUTO_ADJUST_ROUNDING_US: i64 = 15_000;
/// Lowest judge offset a run may adjust itself to, in milliseconds. The JUDGE OFFSET row's own
/// floor, pinned against it by the player rather than depending on the configuration crate here.
pub const AUTO_ADJUST_OFFSET_MIN_MS: i32 = -200;
/// Highest judge offset a run may adjust itself to, in milliseconds.
pub const AUTO_ADJUST_OFFSET_MAX_MS: i32 = 200;
/// Microseconds in one millisecond, the unit a judge offset is set, shown and recorded in.
const US_PER_MS: i64 = 1_000;

/// How far one judged input moves a judge offset that adjusts itself, in milliseconds: nothing for
/// a judgement worse than GOOD or more than 150 ms out, and otherwise a thirtieth of the error
/// rounded to the nearest whole millisecond and taken off the offset, so an input within 15 ms of
/// its note moves nothing. `delta_us` is how far ahead of the note the input was judged, early
/// positive.
///
/// This is the reference's own step, sign and all (`JudgeManager.java:697-703`:
/// `judgetiming - (mfast +- 15000) / 30000`). The offset is how far ahead of the song the notes
/// are drawn, so a player who presses early has it lowered: the notes are drawn later, and a
/// player who follows them presses later.
pub fn auto_adjust_step_ms(judge: Judge, delta_us: i64) -> i32 {
    if judge as usize > AUTO_ADJUST_WORST_JUDGE || !(-AUTO_ADJUST_MAX_ABS_DELTA_US..=AUTO_ADJUST_MAX_ABS_DELTA_US).contains(&delta_us) {
        return 0;
    }
    let pushed = if delta_us >= 0 { delta_us + AUTO_ADJUST_ROUNDING_US } else { delta_us - AUTO_ADJUST_ROUNDING_US };
    -((pushed / AUTO_ADJUST_US_PER_STEP) as i32)
}

/// The BGA frame shown before the chart's first BGA event.
const BGA_FRAME_NONE: i32 = -1;

/// When a keysound should sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundTime {
    /// As soon as the device can, without waiting for the scheduling lookahead. An input's keysound
    /// is wanted at the instant the key went down, not at the time of the note it hit.
    Immediate,
    /// Booked at this position on the session clock (µs), which the host rebases onto its own axis.
    Scheduled(i64),
}

/// One keysound the session asks the host to play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundRequest {
    /// Keysound id within the chart's own bank.
    pub wav: u32,
    /// Which output bus the sound belongs to.
    pub source: PlaySource,
    pub gain: f32,
    pub pan: f32,
    pub pitch: f32,
    pub at: SoundTime,
}

/// Where a session's keysounds go. The host implements this over its audio engine, mapping
/// [`PlaySource`] onto its own buses and [`SoundTime`] onto its own clock.
pub trait SoundSink {
    /// Play one keysound.
    fn play(&mut self, sound: SoundRequest);
    /// Silence everything this session has started.
    fn stop_all(&mut self);
}

/// A sink that drops every sound: headless tests, offline analysis, and the muted scrubbing of
/// replay analysis all run against it.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSink;

impl SoundSink for NullSink {
    fn play(&mut self, _sound: SoundRequest) {}

    fn stop_all(&mut self) {}
}

/// The two time axes one play frame drives: what the listener is hearing right now, and how far
/// ahead sounds are being booked. Judgement only ever reads the audible axis, so the scheduler's
/// lookahead cannot move a judgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionClock {
    pub audible_us: i64,
    pub scheduled_us: i64,
}

impl SessionClock {
    /// Both axes at one instant, for the virtual clocks (offline analysis, tests, autoplay
    /// simulation) that have no device lead to book against.
    pub fn at(song_us: i64) -> Self {
        SessionClock { audible_us: song_us, scheduled_us: song_us }
    }
}

/// How one run is set up. The note shuffle is applied to the model before the session is built, so
/// an analysis seek rebuilds the judge state against the very same lane layout.
#[derive(Debug, Clone)]
pub struct SessionOptions {
    /// Play the chart from itself instead of from input.
    pub autoplay: bool,
    pub gauge: GaugeKind,
    /// The judge offset the run starts at: how far ahead of the song the notes are drawn, in
    /// microseconds (`PlayerConfig.judgetiming`, which `LaneRenderer.java:300-301` adds to the time
    /// the note field is drawn at and nothing else reads). It moves no judgement and no sound: an
    /// input is judged at the instant it arrived. The session keeps it only because a run that
    /// adjusts it does so as its inputs land ([`auto_adjust_step_ms`]).
    pub judge_offset_us: i64,
    /// JUDGE WIDTH as a percentage of the chart's own windows, applied to every widenable tier of
    /// both key and scratch lanes. The per-tier form lives in [`JudgeSetup`], which
    /// [`PlaySession::set_judge_setup`] takes and which overrides this once given.
    pub judge_rate_percent: i32,
    /// Lanes the chart plays by itself (SCRATCH AUTO), empty when there are none. Their notes are
    /// taken off the chart the run judges and counts, and only drawn and heard
    /// ([`PlaySession::field_timelines`]); input on them is ignored.
    pub auto_lanes: Vec<bool>,
    /// The note-shuffle seed this run was laid out with, reported with the score.
    pub seed: u64,
    /// Show the replay-analysis overlay and accept its playback controls.
    pub analysis: bool,
    /// Let the judge offset adjust itself as inputs land ([`auto_adjust_step_ms`]). A run that plays
    /// itself or reproduces a replay never does, whatever this says.
    pub auto_calibration: bool,
    /// The replay to reproduce instead of taking input.
    pub replay: Option<Replay>,
    pub initial_gauge: Option<(GaugeIndex, f32)>,
    pub initial_combo: u32,
}

impl Default for SessionOptions {
    fn default() -> Self {
        SessionOptions {
            autoplay: false,
            gauge: GaugeKind::Normal,
            judge_offset_us: 0,
            judge_rate_percent: UNMODIFIED_RATE_PERCENT,
            auto_lanes: Vec::new(),
            seed: 0,
            analysis: false,
            auto_calibration: false,
            replay: None,
            initial_gauge: None,
            initial_combo: 0,
        }
    }
}

/// Everything on the JUDGE settings screen that changes how a run is judged, in one value so a
/// caller can hand the whole screen over in a single call.
///
/// [`PlaySession::new`] starts a run on the defaults — the widths the chart's mode states, plain
/// long notes, the gauge table the mode selects and no auto-shift — and
/// [`PlaySession::set_judge_setup`] replaces them before play begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeSetup {
    /// Per-tier `[PGREAT, GREAT, GOOD]` JUDGE WIDTH percentages for key lanes.
    pub judge_rate_key: [i32; JUDGE_WIDTH_TIER_COUNT],
    /// Per-tier JUDGE WIDTH percentages for scratch lanes.
    pub judge_rate_scratch: [i32; JUDGE_WIDTH_TIER_COUNT],
    /// LONGNOTE MARGIN as a percentage of the mode's stock release margin.
    pub longnote_margin_rate: i32,
    /// Which note a press takes when several are in range.
    pub algorithm: JudgeAlgorithm,
    /// The flavour long notes the chart left unstated play as.
    pub ln_mode: LnMode,
    /// Gauge table to play on, or `None` to take the one the chart's mode selects.
    pub gauge_set: Option<GaugeSetId>,
    /// How the selected gauge may move during play.
    pub gauge_auto_shift: GaugeAutoShift,
    /// The floor the per-frame auto-shift may drop the selection to.
    pub bottom_shiftable_gauge: GaugeKind,
}

impl Default for JudgeSetup {
    fn default() -> Self {
        JudgeSetup {
            judge_rate_key: UNMODIFIED_JUDGE_RATES,
            judge_rate_scratch: UNMODIFIED_JUDGE_RATES,
            longnote_margin_rate: UNMODIFIED_RATE_PERCENT,
            algorithm: JudgeAlgorithm::default(),
            ln_mode: LnMode::default(),
            gauge_set: None,
            gauge_auto_shift: GaugeAutoShift::default(),
            bottom_shiftable_gauge: GaugeKind::AssistEasy,
        }
    }
}

impl JudgeSetup {
    /// The setup a single JUDGE WIDTH slider describes: one percentage on every widenable tier of
    /// both key and scratch lanes, everything else left at its default.
    fn uniform_rate(rate_percent: i32) -> Self {
        JudgeSetup {
            judge_rate_key: [rate_percent; JUDGE_WIDTH_TIER_COUNT],
            judge_rate_scratch: [rate_percent; JUDGE_WIDTH_TIER_COUNT],
            ..JudgeSetup::default()
        }
    }
}

/// The part of [`SessionOptions`] an analysis seek has to rebuild the judge state from. Everything
/// that decides a judgement lives here, so a seek reproduces the run it is scrubbing rather than a
/// default-configured one.
#[derive(Debug, Clone)]
struct Setup {
    autoplay: bool,
    gauge: GaugeKind,
    judge: JudgeSetup,
    auto_lanes: Vec<bool>,
    seed: u64,
    initial_gauge: Option<(GaugeIndex, f32)>,
    initial_combo: u32,
    /// The rate the chart is played at, in percent ([`PlaySession::set_playback_rate`]).
    playback_percent: i32,
}

/// The replay being reproduced and how far into it the run has got.
#[derive(Debug)]
struct ReplayTrack {
    events: Vec<ReplayEvent>,
    cursor: usize,
    /// How far from the instant it was recorded at every input of the run was judged, in
    /// microseconds: the replay's own `offset_ms`. Nothing for a run recorded since the judge
    /// offset stopped moving the judgement, and what the run was played at for an older one.
    judged_off_by_us: i64,
    /// The same for a run that moved through several, in the order it did.
    offset_steps: Vec<ReplayOffsetStep>,
}

impl ReplayTrack {
    /// The next recorded input whose judged instant has been reached, with that instant, consuming
    /// it. The instant is the recorded one moved by the offset the replay says it was judged at
    /// ([`recorded_offset_us`]), which is nothing for every run recorded since the offset stopped
    /// moving the judgement; it is what decides when the input is due, so the engine is never
    /// carried past the clock that brought it there.
    fn next_due(&mut self, song_us: i64) -> Option<(ReplayEvent, i64)> {
        let event = *self.events.get(self.cursor)?;
        let judge_us = event.t + recorded_offset_us(&self.offset_steps, self.cursor, self.judged_off_by_us);
        if judge_us > song_us {
            return None;
        }
        self.cursor += 1;
        Some((event, judge_us))
    }
}

/// How far from the instant it was recorded at the recorded input at `index` was judged, in
/// microseconds: what the last step at or before it brought in, and `fallback_us` for a run that
/// recorded no step that early, which is every run judged at one offset throughout.
fn recorded_offset_us(steps: &[ReplayOffsetStep], index: usize, fallback_us: i64) -> i64 {
    ReplayOffsetStep::in_force_at(steps, index).map_or(fallback_us, |offset_ms| i64::from(offset_ms) * US_PER_MS)
}

/// Replay-analysis playback: a virtual clock the player can pause, slow down and scrub with.
#[derive(Debug, Clone, Copy)]
struct AnalysisState {
    enabled: bool,
    manual: bool,
    paused: bool,
    rate: f64,
    position_us: i64,
}

impl AnalysisState {
    fn new(enabled: bool) -> Self {
        AnalysisState { enabled, manual: false, paused: false, rate: 1.0, position_us: 0 }
    }
}

/// One judged input's timing error, newest last, for the analysis overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingMark {
    pub lane: usize,
    pub delta_us: i64,
    pub judge: u8,
}

/// A bounded ring of the most recent judged inputs.
#[derive(Debug, Default)]
struct TimingMarks {
    marks: Vec<TimingMark>,
}

impl TimingMarks {
    fn push(&mut self, mark: TimingMark) {
        if self.marks.len() == TIMING_MARK_CAPACITY {
            self.marks.remove(0);
        }
        self.marks.push(mark);
    }

    fn clear(&mut self) {
        self.marks.clear();
    }

    fn recent(&self, count: usize) -> &[TimingMark] {
        &self.marks[self.marks.len().saturating_sub(count)..]
    }
}

/// How many buckets the timing histogram spreads its window over.
pub const TIMING_HIST_BINS: usize = 81;

/// How wide one timing histogram bucket is.
pub const TIMING_HIST_BIN_US: i64 = 5_000;

/// How far either side of the note the timing histogram reaches. Anything further out is counted in
/// the outermost bucket rather than dropped, so the totals still add up to the inputs that landed.
pub const TIMING_HIST_RANGE_US: i64 = TIMING_HIST_BIN_US * (TIMING_HIST_BINS as i64 / 2);

/// How much song time passes between two gauge samples.
pub const GAUGE_SAMPLE_INTERVAL_US: i64 = 1_000_000;

/// How many gauge samples are kept. At one a second this is long enough for any chart a player will
/// meet and bounded for the ones they will not, so a very long run cannot grow the series without
/// end.
pub const GAUGE_SERIES_CAPACITY: usize = 1_800;

/// How many judgements a judged input can be given, which is the width of the distribution.
pub const JUDGE_KIND_COUNT: usize = 6;

/// How many kinds of lane an input can come from: the keys and the turntable.
pub const LANE_KIND_COUNT: usize = 2;

/// Where the keys are counted in a lane-kind split.
pub const KEY_LANE_KIND: usize = 0;

/// Where the turntable is counted in a lane-kind split.
pub const SCRATCH_LANE_KIND: usize = 1;

/// Judgements from this one on carry no timing: a miss and an empty poor have no input to be early
/// or late against, which is where the judge engine stops counting them too
/// (`JudgeManager.java:652` records only `judge < 4`).
pub const TIMED_JUDGE_LIMIT: usize = 4;

/// What the run looked like over time, for the graphs the result screen draws.
///
/// The gauge series is sampled on the song clock rather than on frames, so it does not change shape
/// with the frame rate. The two tallies count *judgments of the player's inputs*, each once: the
/// judge engine's own counts include the notes a miss sweep and an auto-played lane resolve, which
/// never pass through here, so the two are related but not equal — and the histogram needs a timing
/// error, which only a judged input has. A plain long note is one judgment, at its end, however
/// many inputs took it.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayInstrumentation {
    gauge_series: Vec<f32>,
    next_gauge_sample_us: i64,
    timing_hist: [u32; TIMING_HIST_BINS],
    judge_dist: [u32; JUDGE_KIND_COUNT],
    fast: [u32; LANE_KIND_COUNT],
    slow: [u32; LANE_KIND_COUNT],
}

impl Default for PlayInstrumentation {
    fn default() -> Self {
        PlayInstrumentation {
            gauge_series: Vec::new(),
            next_gauge_sample_us: 0,
            timing_hist: [0; TIMING_HIST_BINS],
            judge_dist: [0; JUDGE_KIND_COUNT],
            fast: [0; LANE_KIND_COUNT],
            slow: [0; LANE_KIND_COUNT],
        }
    }
}

impl PlayInstrumentation {
    /// The gauge value at each sample, oldest first.
    pub fn gauge_series(&self) -> &[f32] {
        &self.gauge_series
    }

    /// How many judged inputs landed in each timing bucket, earliest bucket first. The middle
    /// bucket straddles the note itself.
    pub fn timing_hist(&self) -> &[u32; TIMING_HIST_BINS] {
        &self.timing_hist
    }

    /// How many judged inputs took each judgement, best first.
    pub fn judge_dist(&self) -> &[u32; JUDGE_KIND_COUNT] {
        &self.judge_dist
    }

    /// Early hits, split into the keys and the turntable.
    ///
    /// Counted here rather than by the screen because a run's inputs do not all come through one:
    /// a replay reproduces its own, and an auto-played lane is judged inside the session. A tally
    /// kept outside would read zero for every run the player is not physically playing.
    pub fn fast(&self) -> [u32; LANE_KIND_COUNT] {
        self.fast
    }

    /// Late hits, split the same way as [`PlayInstrumentation::fast`].
    pub fn slow(&self) -> [u32; LANE_KIND_COUNT] {
        self.slow
    }

    /// Which bucket a timing error falls in, clamped to the ends of the window.
    pub fn bin_of(delta_us: i64) -> usize {
        let middle = TIMING_HIST_BINS / 2;
        let clamped = delta_us.clamp(-TIMING_HIST_RANGE_US, TIMING_HIST_RANGE_US);
        let offset = clamped.div_euclid(TIMING_HIST_BIN_US);
        (middle as i64 + offset).clamp(0, TIMING_HIST_BINS as i64 - 1) as usize
    }

    fn record(&mut self, judge: Judge, delta_us: i64, scratch: bool) {
        self.timing_hist[PlayInstrumentation::bin_of(delta_us)] += 1;
        if let Some(slot) = self.judge_dist.get_mut(judge as usize) {
            *slot += 1;
        }
        if (judge as usize) >= TIMED_JUDGE_LIMIT {
            return;
        }
        let kind = if scratch { SCRATCH_LANE_KIND } else { KEY_LANE_KIND };
        let column = if delta_us >= 0 { &mut self.fast } else { &mut self.slow };
        column[kind] += 1;
    }

    /// Take a gauge sample if the song has reached the next sample point. Called every frame; the
    /// interval is on the song clock, so a paused or scrubbed analysis run does not fill the series
    /// with duplicates.
    fn sample_gauge(&mut self, song_us: i64, value: f32) {
        if song_us < self.next_gauge_sample_us || self.gauge_series.len() == GAUGE_SERIES_CAPACITY {
            return;
        }
        self.gauge_series.push(value);
        self.next_gauge_sample_us = song_us - song_us.rem_euclid(GAUGE_SAMPLE_INTERVAL_US) + GAUGE_SAMPLE_INTERVAL_US;
    }

    fn clear(&mut self) {
        self.gauge_series.clear();
        self.next_gauge_sample_us = 0;
        self.timing_hist = [0; TIMING_HIST_BINS];
        self.judge_dist = [0; JUDGE_KIND_COUNT];
        self.fast = [0; LANE_KIND_COUNT];
        self.slow = [0; LANE_KIND_COUNT];
    }
}

/// The chart's BGA changes and the frame currently showing.
#[derive(Debug, Default)]
struct BgaTimeline {
    events: Vec<(i64, i32)>,
    cursor: usize,
    current: i32,
}

impl BgaTimeline {
    fn from_model(model: &Model) -> Self {
        let mut events: Vec<(i64, i32)> = model.timelines.iter().filter(|tl| tl.bga >= 0).map(|tl| (tl.time_us, tl.bga)).collect();
        events.sort_by_key(|e| e.0);
        BgaTimeline { events, cursor: 0, current: BGA_FRAME_NONE }
    }

    fn advance(&mut self, song_us: i64) {
        while self.cursor < self.events.len() && self.events[self.cursor].0 <= song_us {
            self.current = self.events[self.cursor].1;
            self.cursor += 1;
        }
    }
}

/// Everything the result screen reads off the judge engine, as plain values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaySummary {
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub max_ex_score: u32,
    /// The longest combo within this chart (`ScoreData.combo`). A course stage counts no streak
    /// carried in from the stage before ([`PlaySession::standing_max_combo`] does).
    pub max_combo: u32,
    pub total_notes: u32,
    /// Notes resolved so far; empty POOR consumes none, so this can trail `total_notes`.
    pub total_judged: u32,
    pub fast: u32,
    pub slow: u32,
    /// Per-judge early/late split, indexed the same way as `counts`.
    pub early: [u32; 6],
    pub late: [u32; 6],
    /// Mean signed timing error over the run's actual hits: the lean, early positive. Not the
    /// reference's `avgjudge` ([`crate::PlayRecord::avg_duration_us`] is).
    pub avg_judge_us: i64,
    pub empty_poor: u32,
    pub gauge_value: f32,
    pub clear_lamp: ClearType,
    /// Bad + poor + miss so far. The notes a run never reached are not in it; what is recorded of
    /// a run is [`PlaySummary::min_bp_with_unreached`].
    pub min_bp: u32,
    /// Whether the gauge emptied under a shift mode that does not rescue the play.
    pub failed: bool,
    /// The gauge that decided the clear, which auto-shift may have moved away from the one the
    /// player chose (`BMSPlayer.java:639-650`). `None` for a course gauge, which no setting selects.
    pub finished_gauge: Option<GaugeKind>,
    /// Whether auto-shift moved the selection (`GrooveGauge.java:99-101`). The reference reports the
    /// gauge of such a run as `-1` rather than as the one that was chosen (`BMSPlayer.java:884`).
    pub gauge_shifted: bool,
}

impl PlaySummary {
    /// `ScoreData.minbp` as the reference records a run: bad, poor and empty poor, plus every note
    /// the run never reached (`BMSPlayer.java:909`). A run the gauge ended early is charged for the
    /// notes it did not get to, so dying at the start is never a better MISS COUNT than playing on.
    /// A summary already [`left_unfinished`](Self::left_unfinished) has no note left unreached and is
    /// charged nothing more.
    pub fn min_bp_with_unreached(&self) -> u32 {
        self.min_bp + self.total_notes.saturating_sub(self.total_judged)
    }

    /// The summary of a run left before its end: every note it never reached counts as a late POOR
    /// the player let pass, and the run is FAILED.
    ///
    /// Only what was left is added, so a long note judged at its head and still to be judged at its
    /// end counts its end alone, and the POOR tier is the one a note let pass lands in. The score,
    /// the combo, the gauge and the timings stay what the run reached. The reference adds what was
    /// left to the bad-poor count alone (`BMSPlayer.java:909`); here the notes are counted too, so
    /// the judgements that took a note add up to the chart's notes as a finished run's do.
    pub fn left_unfinished(&self) -> PlaySummary {
        let unreached = self.total_notes.saturating_sub(self.total_judged);
        let poor = Judge::Poor as usize;
        let mut left = *self;
        left.counts[poor] += unreached;
        left.late[poor] += unreached;
        left.min_bp += unreached;
        left.total_judged = self.total_judged.max(self.total_notes);
        left.clear_lamp = ClearType::Failed;
        left.failed = true;
        left
    }
}

/// One run of a chart: the judge state, the replay being reproduced or recorded, and the
/// analysis clock. Sound is emitted through a [`SoundSink`], so a whole run can be reproduced with
/// no audio device present.
pub struct PlaySession {
    player: Player,
    setup: Setup,
    judge_offset_us: i64,
    auto_calibration: bool,
    replay: Option<ReplayTrack>,
    recording: Vec<ReplayEvent>,
    /// How many inputs moved the judge offset by themselves.
    adjusted_inputs: u32,
    analysis: AnalysisState,
    marks: TimingMarks,
    instrumentation: PlayInstrumentation,
    /// Every gauge's history on the reference's half-second grid, for a result screen a skin draws.
    /// Kept beside `instrumentation` rather than in it: that one is the built-in screen's and
    /// samples the selected gauge once a second.
    gauge_log: GaugeLogRecorder,
    /// The next instant on the song clock the gauge is sampled at, a whole number of
    /// [`GAUGE_SAMPLE_STEP_US`] from the start of the song.
    next_sample_us: i64,
    bga: BgaTimeline,
}

impl PlaySession {
    /// Start a run over an already laid-out `model` (the note shuffle is the caller's, so a seek
    /// can rebuild against the same lanes).
    pub fn new(model: Model, options: SessionOptions) -> Self {
        let bga = BgaTimeline::from_model(&model);
        let gauge_log = GaugeLogRecorder::new(play_time_ms(&model, options.autoplay));
        let setup = Setup {
            autoplay: options.autoplay,
            gauge: options.gauge,
            judge: JudgeSetup::uniform_rate(options.judge_rate_percent),
            auto_lanes: options.auto_lanes,
            seed: options.seed,
            initial_gauge: options.initial_gauge,
            initial_combo: options.initial_combo,
            playback_percent: UNMODIFIED_PLAYBACK_PERCENT,
        };
        let player = build_player(model, &setup);
        let recorded_inputs = match options.autoplay || options.replay.is_some() {
            true => 0,
            false => RECORDED_INPUTS_PER_NOTE * player.judge().total_notes() as usize,
        };
        PlaySession {
            player,
            setup,
            judge_offset_us: options.judge_offset_us,
            auto_calibration: options.auto_calibration,
            replay: options.replay.map(|replay| ReplayTrack {
                events: replay.events,
                cursor: 0,
                judged_off_by_us: i64::from(replay.offset_ms) * US_PER_MS,
                offset_steps: replay.judge.offset_steps,
            }),
            recording: Vec::with_capacity(recorded_inputs),
            adjusted_inputs: 0,
            analysis: AnalysisState::new(options.analysis),
            marks: TimingMarks::default(),
            instrumentation: PlayInstrumentation::default(),
            gauge_log,
            next_sample_us: 0,
            bga,
        }
    }

    /// Advance one frame: reproduce any replay input the audible axis has reached, book the sounds
    /// that fall inside the schedule axis, judge up to the audible axis, then move the BGA on.
    ///
    /// Nothing the run is judged by depends on where the frames fall. Every input is judged at its
    /// own instant with the clock settled up to it first, and the clock is settled one instant
    /// after another ([`rbms_judge::JudgeEngine::update`]), so a replay comes out as the run it
    /// recorded at any frame rate. The gauge history is sampled at its own instants on the song
    /// clock for the same reason, rather than on the first frame past each.
    pub fn tick(&mut self, clock: SessionClock, sink: &mut dyn SoundSink) {
        self.feed_replay(clock.audible_us, sink);
        self.player.update_schedule(clock.scheduled_us, |event| sink.play(scheduled_sound(event)));
        self.settle_until(clock.audible_us);
        self.bga.advance(clock.audible_us);
    }

    /// Settle the run up to `until_us`, stopping at every gauge sample instant on the way to take
    /// the sample there.
    fn settle_until(&mut self, until_us: i64) {
        while self.next_sample_us <= until_us {
            let at_us = self.next_sample_us;
            self.settle_at(at_us);
            let gauge = &self.player.judge().gauge;
            self.instrumentation.sample_gauge(at_us, gauge.value());
            self.gauge_log.frame(at_us, gauge, false);
            self.next_sample_us += GAUGE_SAMPLE_STEP_US;
        }
        self.settle_at(until_us);
    }

    /// Settle the judge up to `at_us`, tally what the clock settled on the way, and close the
    /// gauge history at the instant the run failed, if it has.
    fn settle_at(&mut self, at_us: i64) {
        let judged_before = self.player.judge().judged().len();
        self.player.update_judge(at_us);
        self.note_settled_judgments(judged_before);
        if let Some(failed_at_us) = self.player.failed_at_us() {
            self.gauge_log.frame(failed_at_us, &self.player.judge().gauge, true);
        }
    }

    /// The instant a live input is judged and recorded at: the instant it arrived, or the instant
    /// the run has already been settled up to when it arrived before that. The second only happens
    /// to an input the host read late, and recording the instant it was judged at keeps the replay
    /// reproducing it exactly.
    fn input_instant(&self, raw_us: i64) -> i64 {
        raw_us.max(self.player.judge().settled_us())
    }

    /// Take one lane press, recording it for the replay and judging it at the instant it arrived:
    /// the judge offset moves where the notes are drawn, never when an input counts. The keysound
    /// sounds now, not at the note's time.
    pub fn press(&mut self, lane: usize, raw_us: i64, sink: &mut dyn SoundSink) -> Option<JudgeResult> {
        self.press_dir(lane, ScratchDir::Forward, raw_us, sink)
    }

    /// Take one lane press from one physical direction. A scratch lane has a key for each direction
    /// and the direction is recorded with the input, so a back-spin reproduces as one.
    pub fn press_dir(&mut self, lane: usize, dir: ScratchDir, raw_us: i64, sink: &mut dyn SoundSink) -> Option<JudgeResult> {
        let at_us = self.input_instant(raw_us);
        self.settle_until(at_us);
        self.recording.push(ReplayEvent { t: at_us, lane, press: true, backward: is_backward(dir) });
        let result = self.player.press_dir(lane, dir, at_us, |event| sink.play(immediate_sound(event)));
        if let Some(result) = result.filter(|result| result.counted) {
            self.note_judgment(result.lane, result.judge, result.delta_us);
        }
        result
    }

    /// Take one lane release at the instant it arrived, recording it for the replay.
    pub fn release(&mut self, lane: usize, raw_us: i64) -> Option<JudgeResult> {
        self.release_dir(lane, ScratchDir::Forward, raw_us)
    }

    /// Take one lane release from one physical direction.
    pub fn release_dir(&mut self, lane: usize, dir: ScratchDir, raw_us: i64) -> Option<JudgeResult> {
        let at_us = self.input_instant(raw_us);
        self.settle_until(at_us);
        self.recording.push(ReplayEvent { t: at_us, lane, press: false, backward: is_backward(dir) });
        let result = self.player.release_dir(lane, dir, at_us);
        if let Some(result) = result.filter(|result| result.counted) {
            self.note_judgment(result.lane, result.judge, result.delta_us);
        }
        result
    }

    /// Tally one judgment the engine counted: into the result screen's distributions, and into the
    /// offset a run that adjusts itself moves ([`auto_adjust`](Self::auto_adjust)).
    fn note_judgment(&mut self, lane: usize, judge: Judge, delta_us: i64) {
        let scratch = self.is_scratch(lane);
        self.instrumentation.record(judge, delta_us, scratch);
        self.auto_adjust(judge, delta_us);
    }

    /// Tally the timed judgments the clock settled by itself since the engine had counted
    /// `judged_before`: a plain long note held through its end, or one whose early release the
    /// margin confirmed. The engine counts them only now, so no input of the player's carried them
    /// into the tallies when it landed. A run that plays itself makes no input, and a miss has no
    /// timing to tally.
    fn note_settled_judgments(&mut self, judged_before: usize) {
        if self.setup.autoplay || self.player.judge().judged().len() == judged_before {
            return;
        }
        let settled: Vec<_> = self.player.judge().judged()[judged_before..]
            .iter()
            .filter(|note| (note.judge as usize) < TIMED_JUDGE_LIMIT)
            .map(|note| (note.lane, note.judge, note.delta_us))
            .collect();
        for (lane, judge, delta_us) in settled {
            self.note_judgment(lane, judge, delta_us);
        }
    }

    /// Move the judge offset by what one judged input of the player's asks for, while the offset
    /// adjusts itself (`JudgeManager.java:697-703`, which runs for every judgement of a run in the
    /// reference's PLAY and PRACTICE modes and for none of an autoplay or a replay).
    ///
    /// The move is to where the notes are drawn from the next frame on, so the judgement of no
    /// input depends on it and the replay has nothing of it to record. The offset stops at the
    /// ends of the range the JUDGE OFFSET row can hold.
    fn auto_adjust(&mut self, judge: Judge, delta_us: i64) {
        if !self.auto_calibration || self.setup.autoplay || self.replay.is_some() {
            return;
        }
        let step = auto_adjust_step_ms(judge, delta_us);
        let next_ms = self.judge_offset_ms().saturating_add(step).clamp(AUTO_ADJUST_OFFSET_MIN_MS, AUTO_ADJUST_OFFSET_MAX_MS);
        if next_ms != self.judge_offset_ms() {
            self.judge_offset_us = i64::from(next_ms) * US_PER_MS;
            self.adjusted_inputs += 1;
        }
    }

    /// Rebuild the judge state at an arbitrary song time by re-running the recorded inputs from the
    /// start, so scrubbing backwards is exactly as correct as scrubbing forwards. Only a replay run
    /// can be scrubbed.
    ///
    /// The rebuild is the run itself played again in one go, silently: each input judged at its own
    /// instant with the clock settled up to it, then the clock settled to the target. Since nothing
    /// in that depends on where frames fall, the rebuilt run is the one being scrubbed, gauge
    /// history and tallies included.
    pub fn seek(&mut self, target_us: i64) {
        if self.replay.is_none() {
            return;
        }
        let target_us = target_us.max(0);
        self.player = build_player(self.player.field_model().clone(), &self.setup);
        if let Some(track) = self.replay.as_mut() {
            track.cursor = 0;
        }
        self.marks.clear();
        self.instrumentation.clear();
        self.gauge_log.clear();
        self.next_sample_us = 0;
        self.feed_replay(target_us, &mut NullSink);
        self.settle_until(target_us);
        self.analysis.manual = true;
        self.analysis.position_us = target_us;
    }

    /// Play the rest of the run out in one step, silently: every replay input still to come judged
    /// at its own instant, then the clock settled past the last note and the tail after it. Since
    /// nothing a run is judged by depends on where frames fall ([`PlaySession::tick`]), a replay
    /// played out this way judges every note as it did when it was recorded, which is how a saved
    /// run is judged again with no screen and no sound.
    pub fn play_out(&mut self) {
        let end_us = self.last_time_us() + PLAY_TAIL_US;
        self.tick(SessionClock::at(end_us), &mut NullSink);
    }

    /// Whether the run is over and the result is due: every note has gone past and the tail silence
    /// has elapsed. An analysis run never ends on its own — the player is scrubbing it.
    pub fn is_finished(&self, song_us: i64) -> bool {
        !self.analysis.enabled && self.player.judge().total_notes() > 0 && song_us > self.player.last_time_us() + PLAY_TAIL_US
    }

    /// Whether there is nothing left to hit, so leaving the field should show the result rather
    /// than discard the run. A chart with no note to judge (a background-only chart, or one whose
    /// every note SCRATCH AUTO plays) has nothing left from the start, as the reference's
    /// `getPastNotes() == getNotes()` reads it (`BMSPlayer.java:953-955`).
    pub fn all_notes_resolved(&self) -> bool {
        let judge = self.player.judge();
        judge.total_judged() >= judge.total_notes()
    }

    /// Everything the result screen needs from this run, as plain values.
    pub fn summary(&self) -> PlaySummary {
        let judge = self.player.judge();
        let counts = judge.counts;
        PlaySummary {
            counts,
            ex_score: judge.ex_score,
            max_ex_score: judge.total_notes() * 2,
            max_combo: judge.stage_max_combo,
            total_notes: judge.total_notes(),
            total_judged: judge.total_judged(),
            fast: judge.fast,
            slow: judge.slow,
            early: judge.early,
            late: judge.late,
            avg_judge_us: judge.avg_judge_us(),
            empty_poor: judge.empty_poor,
            gauge_value: judge.gauge.value(),
            clear_lamp: judge.clear_lamp(),
            min_bp: counts[3] + counts[4] + counts[5],
            failed: self.player.failed(),
            finished_gauge: judge.gauge.selected_index().kind(),
            gauge_shifted: judge.gauge.is_type_changed(),
        }
    }

    /// What a skin's result screen asks about this run, each to the reference's definition: every
    /// gauge's history, the timing of every note, the per-second judge tables and the bounds the
    /// gauges move between. Everything but the gauge history is read off the judge engine here, so
    /// a replay that reproduces the judgements reproduces it.
    pub fn record(&self) -> PlayRecord {
        PlayRecord::of(self.player.model(), self.player.judge(), self.summary(), &self.gauge_log)
    }

    /// The score against the notes that have gone by so far, which is how the reference rates a run
    /// that is still going (`BMSPlayer.java:1031`).
    pub fn score_progress(&self) -> ScoreProgress {
        let judge = self.player.judge();
        ScoreProgress { ex_score: judge.ex_score, pass_notes: judge.total_judged(), total_notes: judge.total_notes() }
    }

    /// How long the reference keeps this chart in its playing state, in milliseconds on the song
    /// clock (`BMSPlayer.playtime`). The gauge history stops there.
    pub fn play_time_ms(&self) -> i64 {
        self.gauge_log.play_time_ms()
    }

    /// Replace everything the JUDGE settings screen controls, before the run starts.
    ///
    /// Resolving the long-note flavour re-counts the chart and every gauge is rebuilt from the new
    /// table, so this discards whatever the gauges hold: call it between [`new`](Self::new) and the
    /// first [`tick`](Self::tick), not mid-run. The setup is remembered, so an analysis
    /// [`seek`](Self::seek) rebuilds against it too.
    pub fn set_judge_setup(&mut self, judge: JudgeSetup) {
        self.setup.judge = judge;
        apply_setup(&mut self.player, &self.setup);
    }

    /// Judge this run as a chart played at `percent` of its own speed, a practice slice's
    /// FREQUENCY: the session is still driven on the chart's clock, and the judge keeps every window,
    /// margin and tick at its length of the player's own time and reports every timing in it, as the
    /// reference does by retiming the chart (`BMSModelUtils.changeFrequency`). Call it before the
    /// first [`tick`](Self::tick); an analysis [`seek`](Self::seek) rebuilds against it too.
    pub fn set_playback_rate(&mut self, percent: i32) {
        self.setup.playback_percent = percent;
        self.player.set_playback_rate(percent);
    }

    /// How this run is judged.
    pub fn judge_setup(&self) -> JudgeSetup {
        self.setup.judge
    }

    /// Live judge state, for the play HUD.
    pub fn judge(&self) -> &JudgeEngine {
        self.player.judge()
    }

    /// The combo a course carries into its next stage: the streak the player sees, which a course
    /// stage started at the one the stage before ended on (`JudgeManager.getCourseCombo`).
    pub fn standing_combo(&self) -> u32 {
        self.player.judge().combo
    }

    /// The lamp a course stands at once this stage is folded in, the stages before it having been
    /// over `notes_before` notes and judged `counts_before` (`MusicResult.java:402-428`).
    ///
    /// The course is a full combo when its longest streak is every note it has been over, which a
    /// stage's own full combo does not make it; MAX and PERFECT are read off the course's counts.
    /// Short of that it is the gauge's lamp, and FAILED once the gauge is out. The chart's own lamp
    /// is [`PlaySummary::clear_lamp`].
    pub fn course_lamp(&self, notes_before: u32, counts_before: [u32; 6]) -> ClearType {
        let judge = self.player.judge();
        let counts: [u32; 6] = std::array::from_fn(|tier| counts_before[tier] + judge.counts[tier]);
        let notes = notes_before + judge.total_notes();
        rbms_judge::clear_lamp(judge.gauge.selected(), &counts, judge.max_combo, notes, notes)
    }

    /// The longest streak of the course so far, carried streak included
    /// (`JudgeManager.getCourseMaxcombo`). The chart's own is [`PlaySummary::max_combo`].
    pub fn standing_max_combo(&self) -> u32 {
        self.player.judge().max_combo
    }

    /// Which note a press takes when several are in range.
    pub fn algorithm(&self) -> JudgeAlgorithm {
        self.player.algorithm()
    }

    /// The judgerank rule the chart's mode is judged under.
    pub fn judge_window_rule(&self) -> JudgeWindowRule {
        self.player.judge_window_rule()
    }

    /// The gauge table the nine gauges are built from.
    pub fn gauge_set(&self) -> GaugeSetId {
        self.player.gauge_set()
    }

    /// How the selected gauge may move during play.
    pub fn gauge_auto_shift(&self) -> GaugeAutoShift {
        self.player.gauge_auto_shift()
    }

    /// The floor the per-frame auto-shift may drop the selection to.
    pub fn bottom_shiftable_gauge(&self) -> GaugeKind {
        self.player.bottom_shiftable_gauge()
    }

    /// The gauge the player chose, which auto-shift may have moved the selection away from.
    pub fn configured_gauge(&self) -> GaugeKind {
        self.player.configured_gauge()
    }

    /// The flavour long notes the chart left unstated play as.
    pub fn ln_mode(&self) -> LnMode {
        self.player.ln_mode()
    }

    /// Whether the gauge emptied under a shift mode that does not rescue the play
    /// (`BMSPlayer.java:653-661`). From that instant the session judges nothing more: it takes no
    /// input and settles no more time, as the reference stops judging in `STATE_FAILED`. Ending the
    /// run is the caller's.
    pub fn is_failed(&self) -> bool {
        self.player.failed()
    }

    /// The chart being played, laid out as this run judges it: without the notes of the lanes the
    /// chart plays by itself, so every count read off it is the run's own.
    pub fn model(&self) -> &Model {
        self.player.model()
    }

    /// The timelines the note field is drawn from: [`PlaySession::model`]'s, with the notes of the
    /// lanes the chart plays by itself put back.
    pub fn field_timelines(&self) -> &[TimeLine] {
        &self.player.field_model().timelines
    }

    /// Which lanes the chart plays by itself, in lane order. Their notes are drawn as the chart's
    /// own rather than the player's.
    pub fn auto_lanes(&self) -> &[bool] {
        self.player.auto_lanes()
    }

    /// Whether any note of the chart was left to a lane that plays by itself, which makes the run an
    /// assisted one.
    pub fn has_auto_played_notes(&self) -> bool {
        self.player.has_auto_played_notes()
    }

    /// Per-lane key-beam press timestamps.
    pub fn beam_on(&self) -> &[i64] {
        self.player.beam_on()
    }

    /// Per-lane key-beam release timestamps.
    pub fn beam_off(&self) -> &[i64] {
        self.player.beam_off()
    }

    /// Per-lane key-bomb state.
    pub fn bomb(&self) -> &[(i64, u8)] {
        self.player.bomb()
    }

    /// Song time of the chart's last timeline row.
    pub fn last_time_us(&self) -> i64 {
        self.player.last_time_us()
    }

    /// The note-shuffle seed this run was laid out with.
    pub fn seed(&self) -> u64 {
        self.setup.seed
    }

    /// Whether this run is reproducing a replay rather than taking input.
    pub fn is_replay(&self) -> bool {
        self.replay.is_some()
    }

    /// The inputs recorded so far, in arrival order.
    pub fn recorded_events(&self) -> &[ReplayEvent] {
        &self.recording
    }

    /// Follow a live change of the judge offset, so a run that adjusts it goes on from where the
    /// player put it. The offset is one the JUDGE OFFSET row can hold, a whole number of
    /// milliseconds.
    pub fn set_judge_offset_us(&mut self, offset_us: i64) {
        self.judge_offset_us = offset_us;
    }

    /// The judge offset as it stands, in whole milliseconds: how far ahead of the song the notes
    /// are drawn ([`SessionOptions::judge_offset_us`]).
    pub fn judge_offset_ms(&self) -> i32 {
        (self.judge_offset_us / US_PER_MS) as i32
    }

    /// Follow a live change of the setting that lets the judge offset adjust itself.
    pub fn set_auto_calibration(&mut self, enabled: bool) {
        self.auto_calibration = enabled;
    }

    /// How many inputs moved the judge offset ([`auto_adjust_step_ms`]).
    pub fn adjusted_inputs(&self) -> u32 {
        self.adjusted_inputs
    }

    /// Whether the replay-analysis overlay and its playback controls are active.
    pub fn analysis_enabled(&self) -> bool {
        self.analysis.enabled
    }

    /// Follow a live change of the REPLAY ANALYSIS setting.
    pub fn set_analysis_enabled(&mut self, enabled: bool) {
        self.analysis.enabled = enabled;
    }

    /// Whether playback has been taken over by the analysis controls, in which case the run is on
    /// the virtual clock and its keysounds are muted.
    pub fn analysis_manual(&self) -> bool {
        self.analysis.enabled && self.analysis.manual
    }

    pub fn analysis_paused(&self) -> bool {
        self.analysis.paused
    }

    pub fn analysis_rate(&self) -> f64 {
        self.analysis.rate
    }

    /// Where the virtual analysis clock currently stands.
    pub fn analysis_position_us(&self) -> i64 {
        self.analysis.position_us
    }

    /// Keep the virtual clock on the live position, so a first manual control resumes from what is
    /// being heard rather than from the start.
    pub fn sync_analysis_position(&mut self, song_us: i64) {
        if self.analysis.enabled {
            self.analysis.position_us = song_us;
        }
    }

    /// Move the virtual clock on by one frame at the current rate, and report where it now stands.
    pub fn advance_analysis(&mut self, frame_us: i64) -> i64 {
        if !self.analysis.paused {
            self.analysis.position_us = (self.analysis.position_us + (frame_us as f64 * self.analysis.rate) as i64).max(0);
        }
        self.analysis.position_us
    }

    /// Pause or resume analysis playback, taking manual control of it.
    pub fn toggle_analysis_pause(&mut self) {
        self.analysis.manual = true;
        self.analysis.paused = !self.analysis.paused;
    }

    /// Step the analysis playback rate, taking manual control of it.
    pub fn adjust_analysis_rate(&mut self, steps: i32) {
        self.analysis.manual = true;
        let moved = self.analysis.rate + steps as f64 * ANALYSIS_RATE_STEP;
        self.analysis.rate = moved.clamp(ANALYSIS_RATE_MIN, ANALYSIS_RATE_MAX);
    }

    /// The most recent judged inputs, oldest first.
    pub fn recent_marks(&self, count: usize) -> &[TimingMark] {
        self.marks.recent(count)
    }

    /// What the run looked like over time: the gauge series and the two tallies the result screen
    /// draws its graphs from.
    pub fn instrumentation(&self) -> &PlayInstrumentation {
        &self.instrumentation
    }

    /// Whether a lane of this chart is the turntable, which is what splits the early/late tallies
    /// into their two columns.
    fn is_scratch(&self, lane: usize) -> bool {
        self.player.model().mode.is_scratch(lane)
    }

    /// The BGA frame currently showing, or a negative id before the chart's first BGA event.
    pub fn bga_frame(&self) -> i32 {
        self.bga.current
    }

    /// Reproduce every recorded input the audible axis has reached, judging each as far from its
    /// recorded time as the replay says it was judged, so the original run comes out identical: at
    /// the time itself for a run recorded since the judge offset stopped moving the judgement, and
    /// the recorded offset away for an older one. The judge offset in force now is the viewer's and
    /// has no part in it. Each input settles the run up to its own instant first, as a live one
    /// does.
    fn feed_replay(&mut self, audible_us: i64, sink: &mut dyn SoundSink) {
        while let Some((event, judge_us)) = self.replay.as_mut().and_then(|track| track.next_due(audible_us)) {
            let dir = scratch_dir(event.backward);
            self.settle_until(judge_us);
            let result = if event.press {
                self.player.press_dir(event.lane, dir, judge_us, |emitted| sink.play(immediate_sound(emitted)))
            } else {
                self.player.release_dir(event.lane, dir, judge_us)
            };
            if let Some(result) = result.filter(|result| result.counted) {
                self.note_judgment(result.lane, result.judge, result.delta_us);
                self.marks.push(TimingMark { lane: result.lane, delta_us: result.delta_us, judge: result.judge as u8 });
            }
        }
    }
}

/// The direction a recorded input spun its lane.
fn scratch_dir(backward: bool) -> ScratchDir {
    if backward { ScratchDir::Backward } else { ScratchDir::Forward }
}

/// How a direction is recorded in a replay.
fn is_backward(dir: ScratchDir) -> bool {
    dir == ScratchDir::Backward
}

/// A judge player set up the way this run was configured, used both to start the run and to rebuild
/// it at a seek target.
fn build_player(model: Model, setup: &Setup) -> Player {
    let mut player = Player::with_auto_lanes(model, setup.autoplay, &setup.auto_lanes);
    apply_setup(&mut player, setup);
    player
}

/// Configure a freshly built player exactly the way this run is set up.
///
/// The order matters: resolving the long-note flavour re-counts the chart because charge notes are
/// judged twice, and both that and the gauge table decide how the nine gauges are built, so the
/// gauge selection has to come after them.
fn apply_setup(player: &mut Player, setup: &Setup) {
    player.set_ln_mode(setup.judge.ln_mode);
    if let Some(set) = setup.judge.gauge_set {
        player.set_gauge_set(set);
    }
    player.set_gauge(setup.gauge);
    player.set_judge_window_rates(setup.judge.judge_rate_key, setup.judge.judge_rate_scratch);
    player.set_longnote_margin_rate(setup.judge.longnote_margin_rate);
    player.set_algorithm(setup.judge.algorithm);
    player.set_playback_rate(setup.playback_percent);
    player.set_gauge_auto_shift(setup.judge.gauge_auto_shift);
    player.set_bottom_shiftable_gauge(setup.judge.bottom_shiftable_gauge);
    if let Some((index, gauge_value)) = setup.initial_gauge {
        player.set_initial_state(index, gauge_value, setup.initial_combo);
    }
}

/// A keysound booked ahead on the song clock: autoplay accompaniment and autoplay note sounds.
fn scheduled_sound(event: PlayEvent) -> SoundRequest {
    SoundRequest {
        wav: event.wav.max(0) as u32,
        source: event.source,
        gain: KEYSOUND_GAIN,
        pan: KEYSOUND_PAN,
        pitch: KEYSOUND_PITCH,
        at: SoundTime::Scheduled(event.at_us),
    }
}

/// A keysound wanted at once: an input's own sound, which must not wait for the scheduling
/// lookahead and must not move with the judge offset.
fn immediate_sound(event: PlayEvent) -> SoundRequest {
    SoundRequest { wav: event.wav.max(0) as u32, source: event.source, gain: KEYSOUND_GAIN, pan: KEYSOUND_PAN, pitch: KEYSOUND_PITCH, at: SoundTime::Immediate }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_chart::to_model;
    use rbms_model::Mode;
    use rbms_parser::parse;

    /// A sink that keeps every request, so a test can assert what a run asked to be played.
    #[derive(Debug, Default)]
    struct RecordingSink {
        sounds: Vec<SoundRequest>,
        stopped: u32,
    }

    impl SoundSink for RecordingSink {
        fn play(&mut self, sound: SoundRequest) {
            self.sounds.push(sound);
        }

        fn stop_all(&mut self) {
            self.stopped += 1;
        }
    }

    fn model(bms: &[u8]) -> Model {
        to_model(&parse(bms), Mode::BEAT_7K)
    }

    fn one_note() -> Model {
        model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n")
    }

    fn replay_of(events: Vec<ReplayEvent>) -> Replay {
        Replay {
            chart_path: String::new(),
            md5: String::new(),
            mode: Mode::BEAT_7K.name.to_string(),
            random: String::new(),
            seed: 0,
            offset_ms: 0,
            scratch_auto: false,
            gauge: String::new(),
            judge: Default::default(),
            events,
        }
    }

    /// A run that judged 40 of its 100 notes, every kind of judgement among them, and some 空POOR.
    fn summary_of_a_run_left_at_forty_of_a_hundred() -> PlaySummary {
        PlaySummary {
            counts: [20, 10, 4, 3, 3, 5],
            ex_score: 50,
            max_ex_score: 200,
            max_combo: 17,
            total_notes: 100,
            total_judged: 40,
            fast: 6,
            slow: 7,
            early: [12, 4, 2, 1, 0, 3],
            late: [8, 6, 2, 2, 3, 2],
            avg_judge_us: -1_200,
            empty_poor: 5,
            gauge_value: 64.0,
            clear_lamp: ClearType::Normal,
            min_bp: 11,
            failed: false,
            finished_gauge: Some(GaugeKind::Normal),
            gauge_shifted: false,
        }
    }

    #[test]
    fn a_run_left_unfinished_counts_every_unreached_note_as_a_late_poor_and_fails() {
        let run = summary_of_a_run_left_at_forty_of_a_hundred();
        let left = run.left_unfinished();
        let poor = Judge::Poor as usize;
        assert_eq!(left.counts[poor], run.counts[poor] + 60);
        assert_eq!(left.late[poor], run.late[poor] + 60);
        assert_eq!(left.early, run.early, "a note let pass is late");
        assert_eq!(left.counts[..=poor].iter().sum::<u32>(), left.total_notes, "the judgements that took a note add up to the chart");
        assert_eq!(left.counts[Judge::Miss as usize], run.counts[Judge::Miss as usize], "空POOR takes no note and gains none");
        assert_eq!((left.min_bp, left.total_judged), (run.min_bp + 60, 100));
        assert_eq!((left.clear_lamp, left.failed), (ClearType::Failed, true));
        assert_eq!(left.left_unfinished(), left, "a run with nothing left is left as it is");
    }

    #[test]
    fn a_run_left_unfinished_keeps_its_score_combo_and_gauge() {
        let run = summary_of_a_run_left_at_forty_of_a_hundred();
        let left = run.left_unfinished();
        assert_eq!((left.ex_score, left.max_ex_score, left.max_combo, left.total_notes), (run.ex_score, run.max_ex_score, run.max_combo, run.total_notes));
        assert_eq!((left.gauge_value, left.finished_gauge, left.gauge_shifted), (run.gauge_value, run.finished_gauge, run.gauge_shifted));
        assert_eq!((left.fast, left.slow, left.avg_judge_us, left.empty_poor), (run.fast, run.slow, run.avg_judge_us, run.empty_poor));
    }

    /// A long note the chart states no flavour for, so LN MODE decides how many judged objects it is.
    fn undefined_long_note() -> Model {
        model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n")
    }

    /// The session builds its player once at [`PlaySession::new`] and again at every
    /// [`PlaySession::seek`], so LN MODE has to survive being applied twice — the run would
    /// otherwise judge a different number of notes after a scrub than before it.
    #[test]
    fn setting_the_long_note_mode_after_the_session_is_built_still_reaches_the_engine() {
        let mut session = PlaySession::new(undefined_long_note(), SessionOptions::default());
        assert_eq!(session.judge().total_notes(), 1, "the shipped LN MODE judges it as one plain long note");
        session.set_judge_setup(JudgeSetup { ln_mode: LnMode::ChargeNote, ..JudgeSetup::default() });
        assert_eq!(session.judge().total_notes(), 2, "a charge note is judged at its head and at its end");
        assert_eq!(session.ln_mode(), LnMode::ChargeNote);
    }

    #[test]
    fn a_scrub_rebuilds_the_run_with_the_same_note_count_it_was_judging() {
        let mut session = PlaySession::new(undefined_long_note(), SessionOptions { replay: Some(replay_of(Vec::new())), ..Default::default() });
        session.set_judge_setup(JudgeSetup { ln_mode: LnMode::ChargeNote, ..JudgeSetup::default() });
        let before = session.judge().total_notes();
        session.seek(1_000_000);
        assert_eq!(session.judge().total_notes(), before, "the live run and the scrubbed one must share an EX denominator");
    }

    /// A charge note on the scratch lane, so the second scratch key has something to end.
    fn scratch_charge_note() -> Model {
        model(b"#BPM 120\r\n#RANK 3\r\n#LNMODE 2\r\n#WAV01 a.wav\r\n#00156:01000001\r\n")
    }

    fn scratch_note_times(m: &Model) -> (i64, i64) {
        let times: Vec<i64> = m.timelines.iter().flat_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).collect();
        (times[0], times[1])
    }

    /// Spinning the other way is what ends a charge note the forward key grabbed
    /// (`JudgeManager.java:358-372`), so the direction has to survive the whole path from the input
    /// down to the judge engine — and be recorded, or the replay would not reproduce the run.
    #[test]
    fn a_back_spin_ends_the_charge_note_the_forward_spin_grabbed() {
        let m = scratch_charge_note();
        let (head_us, end_us) = scratch_note_times(&m);
        let mut session = PlaySession::new(m, SessionOptions::default());
        assert_eq!(session.press_dir(7, ScratchDir::Forward, head_us, &mut NullSink).map(|r| r.judge), Some(rbms_judge::Judge::PerfectGreat));
        let ended = session.press_dir(7, ScratchDir::Backward, end_us, &mut NullSink);
        assert_eq!(ended.map(|r| r.judge), Some(rbms_judge::Judge::PerfectGreat), "the opposite direction takes the charge-note end");
        assert_eq!(session.summary().counts[0], 2, "a charge note is judged at both ends");

        let recorded = session.recorded_events();
        assert!(!recorded[0].backward, "the forward spin is recorded as one");
        assert!(recorded[1].backward, "and the back spin as one, or a replay would re-grab instead of ending");
    }

    /// The same run reproduced from its own recording has to end the charge note the same way, which
    /// it can only do if playback reads the recorded direction back.
    #[test]
    fn a_replayed_back_spin_ends_the_charge_note_again() {
        let m = scratch_charge_note();
        let (head_us, end_us) = scratch_note_times(&m);
        let events = vec![ReplayEvent { t: head_us, lane: 7, press: true, backward: false }, ReplayEvent { t: end_us, lane: 7, press: true, backward: true }];
        let mut session = PlaySession::new(m, SessionOptions { replay: Some(replay_of(events)), ..Default::default() });
        session.tick(SessionClock::at(end_us + PLAY_TAIL_US), &mut NullSink);
        assert_eq!(session.summary().counts[0], 2, "the reproduction took both ends, so the direction came back off the recording");
    }

    /// The result has to name the gauge that decided the clear: GAUGE AUTO SHIFT re-picks every
    /// frame (`BMSPlayer.java:639-650`), and a run recorded under the gauge that was merely chosen
    /// would pair a NORMAL gauge with a HARD lamp.
    #[test]
    fn a_summary_reports_the_gauge_the_run_finished_on() {
        let mut unshifted = PlaySession::new(one_note(), SessionOptions::default());
        unshifted.tick(SessionClock::at(0), &mut NullSink);
        assert!(!unshifted.summary().gauge_shifted, "nothing shifts under the shipped GAUGE AUTO SHIFT");
        assert_eq!(unshifted.summary().finished_gauge, Some(GaugeKind::Normal));

        let mut shifted = PlaySession::new(one_note(), SessionOptions::default());
        shifted.set_judge_setup(JudgeSetup { gauge_auto_shift: GaugeAutoShift::BestClear, ..JudgeSetup::default() });
        shifted.tick(SessionClock::at(0), &mut NullSink);
        let summary = shifted.summary();
        assert!(summary.gauge_shifted, "BEST CLEAR climbs off the chosen gauge on the first frame");
        assert_ne!(summary.finished_gauge, Some(GaugeKind::Normal));
        assert_eq!(summary.finished_gauge, shifted.judge().gauge.selected_index().kind());
    }

    #[test]
    fn a_fresh_session_is_set_up_the_way_the_chart_asks() {
        let session = PlaySession::new(one_note(), SessionOptions::default());
        assert_eq!(session.judge_setup(), JudgeSetup::default());
        assert_eq!(session.algorithm(), JudgeAlgorithm::default());
        assert_eq!(session.judge_window_rule(), JudgeWindowRule::Normal);
        assert_eq!(session.gauge_set(), GaugeSetId::SevenKeys);
        assert_eq!(session.gauge_auto_shift(), GaugeAutoShift::None);
        assert_eq!(session.bottom_shiftable_gauge(), GaugeKind::AssistEasy);
        assert_eq!(session.ln_mode(), LnMode::LongNote);
        assert!(!session.is_failed());
        assert!(!session.summary().failed);
        assert_eq!(session.standing_combo(), 0);
        assert_eq!(session.summary().gauge_value, 20.0);
    }

    #[test]
    fn a_course_session_keeps_its_gauge_and_combo_after_judge_setup() {
        let mut session = PlaySession::new(
            one_note(),
            SessionOptions {
                initial_gauge: Some((GaugeIndex::Class, 72.0)),
                initial_combo: 120,
                replay: Some(replay_of(Vec::new())),
                ..SessionOptions::default()
            },
        );
        session.set_judge_setup(JudgeSetup { gauge_set: Some(GaugeSetId::Lr2), ..JudgeSetup::default() });
        session.seek(1_000_000);
        let summary = session.summary();
        assert_eq!(session.judge().gauge.selected_index(), GaugeIndex::Class);
        assert_eq!(summary.gauge_value, 72.0);
        assert_eq!(session.standing_combo(), 120);
        assert_eq!(session.standing_max_combo(), 120);
        assert_eq!(summary.max_combo, 0, "the chart's own combo carries nothing in from the stage before");
    }

    /// Four notes a beat apart from the second measure on, on the first key lane.
    const FOUR_NOTES: &[u8] = b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01010101\r\n";

    /// Where the four notes of [`FOUR_NOTES`] fall on the song clock, in microseconds.
    const FOUR_NOTE_TIMES_US: [i64; 4] = [2_000_000, 2_500_000, 3_000_000, 3_500_000];

    /// How long a key of the fixtures is held down, in microseconds.
    const TAP_US: i64 = 10_000;

    /// How long after the last note the fixtures press once more, in microseconds: inside the
    /// empty-POOR window, outside every window of the note.
    const LATE_PRESS_AFTER_LAST_NOTE_US: i64 = 100_000;

    /// A course stage keeps the course's streak going for the player and for the next stage, and
    /// records its own: the chart's max combo is its notes, and every note a PGREAT is a MAX
    /// (`JudgeManager.java:668-676`, `BMSPlayer.java:868`).
    #[test]
    fn a_course_stage_reports_its_own_combo() {
        let carried = 100;
        let m = model(FOUR_NOTES);
        let notes = rbms_chart::count_playable_notes(&m) as u32;
        let options = SessionOptions { autoplay: true, initial_gauge: Some((GaugeIndex::Class, 80.0)), initial_combo: carried, ..Default::default() };
        let mut session = PlaySession::new(m, options);
        session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
        let summary = session.summary();
        assert_eq!(summary.counts[0], notes, "autoplay takes every note as a PGREAT");
        assert_eq!(summary.max_combo, notes, "the chart's record holds its own combo");
        assert_eq!(summary.clear_lamp, ClearType::Max);
        assert_eq!(session.standing_combo(), carried + notes, "the course carries the whole streak on");
        assert_eq!(session.standing_max_combo(), carried + notes);
    }

    /// A course is a full combo only while its streak has gone unbroken from its first note: a stage
    /// that is a MAX of its own after a stage that broke the streak leaves the course on its gauge's
    /// lamp, and one that carries an unbroken streak on is a MAX of the course.
    #[test]
    fn a_course_is_a_full_combo_only_while_its_streak_is_unbroken() {
        let notes_before = 10;
        let course = |carried: u32| {
            let options = SessionOptions { autoplay: true, initial_gauge: Some((GaugeIndex::Class, 80.0)), initial_combo: carried, ..Default::default() };
            let mut session = PlaySession::new(model(FOUR_NOTES), options);
            session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
            session
        };
        let unbroken = course(notes_before);
        assert_eq!(unbroken.summary().clear_lamp, ClearType::Max);
        assert_eq!(unbroken.course_lamp(notes_before, [notes_before, 0, 0, 0, 0, 0]), ClearType::Max);
        let broken = course(0);
        assert_eq!(broken.summary().clear_lamp, ClearType::Max, "the stage is a MAX of its own");
        let lamp = broken.course_lamp(notes_before, [notes_before - 1, 0, 0, 0, 1, 0]);
        assert!(lamp.is_cleared() && !matches!(lamp, ClearType::Max | ClearType::Perfect | ClearType::FullCombo), "{lamp:?}");
    }

    /// Play [`FOUR_NOTES`] in `mode`, every note a PGREAT, then press once more after the last note.
    fn every_note_then_a_press_after_the_last(mode: Mode) -> PlaySummary {
        let mut session = PlaySession::new(to_model(&parse(FOUR_NOTES), mode), SessionOptions::default());
        let last = FOUR_NOTE_TIMES_US[FOUR_NOTE_TIMES_US.len() - 1];
        for at in FOUR_NOTE_TIMES_US.into_iter().chain([last + LATE_PRESS_AFTER_LAST_NOTE_US]) {
            session.press(0, at, &mut NullSink);
            session.release(0, at + TAP_US);
        }
        session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
        session.summary()
    }

    /// An empty POOR after the last note breaks the combo on five keys (`JudgeProperty.java:18`), so
    /// the run is no full combo there, as the reference reads it off the combo standing at the end
    /// (`BMSPlayer.java:868`). Seven keys keep the combo through an empty POOR, and the MAX.
    #[test]
    fn five_key_empty_poor_after_the_last_note_breaks_the_full_combo() {
        let five = every_note_then_a_press_after_the_last(Mode::BEAT_5K);
        assert_eq!((five.counts[0], five.empty_poor), (4, 1), "every note a PGREAT and one empty POOR: {:?}", five.counts);
        assert_eq!(five.max_combo, 4, "the streak was as long as the chart");
        assert_ne!(five.clear_lamp, ClearType::Max, "the combo standing at the end is broken");
        assert!(five.clear_lamp.is_cleared() && five.clear_lamp != ClearType::FullCombo && five.clear_lamp != ClearType::Perfect);

        let seven = every_note_then_a_press_after_the_last(Mode::BEAT_7K);
        assert_eq!((seven.counts[0], seven.empty_poor), (4, 1));
        assert_eq!(seven.clear_lamp, ClearType::Max, "seven keys keep the combo through an empty POOR");
    }

    /// A run the gauge ended is charged for every note it did not get to, as the reference records
    /// it (`BMSPlayer.java:909`); the summary's own count is what was judged.
    #[test]
    fn a_failed_run_counts_unreached_notes_in_min_bp() {
        let m =
            model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:0101010101010101\r\n#00211:0101010101010101\r\n#00311:0101010101010101\r\n#00411:0101010101010101\r\n");
        let mut session = PlaySession::new(m, SessionOptions { gauge: GaugeKind::ExHard, ..Default::default() });
        let mut at = 0;
        while !session.is_failed() && at < session.last_time_us() {
            at += PLAY_TAIL_US / 10;
            session.tick(SessionClock::at(at), &mut NullSink);
        }
        let summary = session.summary();
        assert!(session.is_failed(), "the run never hit a note, so EX-HARD ran out");
        assert!(summary.total_judged < summary.total_notes, "the run ended before the chart did: {summary:?}");
        let bad_poor = summary.counts[3] + summary.counts[4] + summary.counts[5];
        assert_eq!(summary.min_bp_with_unreached(), bad_poor + summary.total_notes - summary.total_judged);
        assert_eq!(session.record().min_bp_with_unreached(), summary.min_bp_with_unreached());
        assert_eq!(summary.left_unfinished().min_bp_with_unreached(), summary.min_bp_with_unreached(), "a run left is charged once");
    }

    #[test]
    fn a_practice_session_keeps_its_initial_normal_gauge_after_judge_setup() {
        let mut session = PlaySession::new(one_note(), SessionOptions { initial_gauge: Some((GaugeIndex::Normal, 64.0)), ..SessionOptions::default() });
        session.set_judge_setup(JudgeSetup { gauge_set: Some(GaugeSetId::Lr2), ..JudgeSetup::default() });
        let summary = session.summary();
        assert_eq!(session.judge().gauge.selected_index(), GaugeIndex::Normal);
        assert_eq!(summary.gauge_value, 64.0);
        assert_eq!(session.standing_combo(), 0);
    }

    #[test]
    fn the_single_judge_width_slider_fills_every_tier() {
        let session = PlaySession::new(one_note(), SessionOptions { judge_rate_percent: 150, ..Default::default() });
        assert_eq!(session.judge_setup().judge_rate_key, [150; JUDGE_WIDTH_TIER_COUNT]);
        assert_eq!(session.judge_setup().judge_rate_scratch, [150; JUDGE_WIDTH_TIER_COUNT]);
    }

    #[test]
    fn the_judge_setup_reaches_the_engine() {
        let mut session = PlaySession::new(one_note(), SessionOptions::default());
        let setup = JudgeSetup {
            judge_rate_key: [120, 110, 105],
            judge_rate_scratch: [90, 95, 100],
            longnote_margin_rate: 150,
            algorithm: JudgeAlgorithm::Lowest,
            ln_mode: LnMode::HellChargeNote,
            gauge_set: Some(GaugeSetId::Lr2),
            gauge_auto_shift: GaugeAutoShift::BestClear,
            bottom_shiftable_gauge: GaugeKind::Normal,
        };
        session.set_judge_setup(setup);
        assert_eq!(session.judge_setup(), setup);
        assert_eq!(session.algorithm(), JudgeAlgorithm::Lowest);
        assert_eq!(session.ln_mode(), LnMode::HellChargeNote);
        assert_eq!(session.gauge_set(), GaugeSetId::Lr2);
        assert_eq!(session.gauge_auto_shift(), GaugeAutoShift::BestClear);
        assert_eq!(session.bottom_shiftable_gauge(), GaugeKind::Normal);
    }

    #[test]
    fn an_out_of_range_auto_shift_floor_is_clamped_before_it_reaches_the_engine() {
        let mut session = PlaySession::new(one_note(), SessionOptions::default());
        session.set_judge_setup(JudgeSetup { bottom_shiftable_gauge: GaugeKind::ExHard, ..JudgeSetup::default() });
        assert_eq!(session.bottom_shiftable_gauge(), GaugeKind::Normal);
    }

    #[test]
    fn the_judge_setup_still_selects_the_gauge_the_run_was_started_with() {
        let mut session = PlaySession::new(one_note(), SessionOptions { gauge: GaugeKind::Hard, ..Default::default() });
        session.set_judge_setup(JudgeSetup { ln_mode: LnMode::ChargeNote, ..JudgeSetup::default() });
        assert_eq!(session.configured_gauge(), GaugeKind::Hard, "rebuilding the gauges must not lose the chosen one");
    }

    #[test]
    fn an_emptied_gauge_fails_the_run_and_shows_in_the_summary() {
        let mut session = PlaySession::new(one_note(), SessionOptions { gauge: GaugeKind::Hazard, ..Default::default() });
        session.tick(SessionClock::at(4_000_000), &mut NullSink);
        assert!(session.is_failed());
        assert!(session.summary().failed);
    }

    #[test]
    fn a_seek_rebuilds_the_judge_setup_the_run_was_scrubbed_with() {
        let mut session = PlaySession::new(one_note(), SessionOptions { replay: Some(replay_of(Vec::new())), ..Default::default() });
        let setup = JudgeSetup {
            judge_rate_key: [130, 120, 110],
            algorithm: JudgeAlgorithm::Score,
            ln_mode: LnMode::ChargeNote,
            gauge_set: Some(GaugeSetId::Lr2),
            gauge_auto_shift: GaugeAutoShift::Continue,
            bottom_shiftable_gauge: GaugeKind::Easy,
            ..JudgeSetup::default()
        };
        session.set_judge_setup(setup);
        session.seek(1_000_000);
        assert_eq!(session.judge_setup(), setup);
        assert_eq!(session.algorithm(), JudgeAlgorithm::Score);
        assert_eq!(session.ln_mode(), LnMode::ChargeNote);
        assert_eq!(session.gauge_set(), GaugeSetId::Lr2);
        assert_eq!(session.gauge_auto_shift(), GaugeAutoShift::Continue);
        assert_eq!(session.bottom_shiftable_gauge(), GaugeKind::Easy);
    }

    #[test]
    fn a_widened_judge_width_reaches_a_press_through_the_session() {
        let late_us = 2_000_000 + 25_000;
        let mut stock = PlaySession::new(one_note(), SessionOptions::default());
        stock.press(0, late_us, &mut NullSink);
        assert_eq!(stock.summary().counts[0], 0);

        let mut widened = PlaySession::new(one_note(), SessionOptions::default());
        widened.set_judge_setup(JudgeSetup { judge_rate_key: [200, 100, 100], ..JudgeSetup::default() });
        widened.press(0, late_us, &mut NullSink);
        assert_eq!(widened.summary().counts[0], 1);
    }

    #[test]
    fn a_press_sounds_its_keysound_at_once_whatever_the_offset() {
        for offset_ms in [-200_i64, -30, 0, 30, 200] {
            let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: offset_ms * 1000, ..Default::default() });
            let mut sink = RecordingSink::default();
            session.press(0, 1_000_000, &mut sink);
            assert_eq!(sink.sounds.len(), 1, "offset {offset_ms}ms still sounds the keysound once");
            assert_eq!(sink.sounds[0].at, SoundTime::Immediate, "offset {offset_ms}ms must not move the sound");
            assert_eq!(sink.sounds[0].source, PlaySource::Key);
        }
    }

    #[test]
    fn the_offset_moves_the_judgement_of_no_input() {
        let hit = |offset_us: i64, pressed_us: i64| {
            let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: offset_us, ..Default::default() });
            session.press(0, pressed_us, &mut NullSink).map(|result| result.delta_us)
        };
        for offset_us in [0, 30_000, -45_000, 200_000, -200_000] {
            assert_eq!(hit(offset_us, 2_000_000), Some(0), "a press on the note has no timing error at {offset_us} us");
            assert_eq!(hit(offset_us, 2_000_000 - 12_000), Some(12_000), "a press 12 ms ahead of the note is 12 ms early at {offset_us} us");
        }
    }

    /// Far enough past a recorded input that it is due whatever offset its replay judged it at.
    const RECORDED_OFFSETS_REACHED_US: i64 = 100_000;

    #[test]
    fn a_replay_is_judged_as_far_from_its_inputs_as_it_says_whatever_the_offset_is_now() {
        let judged = |recorded_ms: i32, steps: Vec<ReplayOffsetStep>, now_us: i64| {
            let replay = Replay {
                chart_path: String::new(),
                md5: String::new(),
                mode: String::new(),
                random: String::new(),
                seed: 0,
                offset_ms: recorded_ms,
                scratch_auto: false,
                gauge: String::new(),
                judge: rbms_store::ReplayJudge { offset_steps: steps, ..Default::default() },
                events: vec![ReplayEvent { t: 2_000_000, lane: 0, press: true, backward: false }],
            };
            let mut session = PlaySession::new(one_note(), SessionOptions { replay: Some(replay), judge_offset_us: now_us, ..Default::default() });
            session.tick(SessionClock::at(2_000_000 + RECORDED_OFFSETS_REACHED_US), &mut NullSink);
            session.recent_marks(1).first().map(|mark| mark.delta_us)
        };
        for now_us in [0, 90_000, -90_000] {
            assert_eq!(judged(0, Vec::new(), now_us), Some(0), "a run recorded at no offset is judged where its inputs are");
            assert_eq!(judged(30, Vec::new(), now_us), Some(-30_000), "a run recorded 30 ms off is judged 30 ms off");
            assert_eq!(
                judged(30, vec![ReplayOffsetStep { from_event: 0, offset_ms: -8 }], now_us),
                Some(8_000),
                "a step it recorded is what its input is judged at"
            );
        }
    }

    #[test]
    fn an_input_is_recorded_at_the_instant_it_arrived() {
        let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: 30_000, ..Default::default() });
        session.press(0, 2_000_000, &mut NullSink);
        session.release(0, 2_050_000);
        assert_eq!(session.recorded_events().len(), 2);
        assert_eq!(session.recorded_events()[0].t, 2_000_000, "the raw input time is what a replay reproduces");
        assert!(session.recorded_events()[0].press);
        assert_eq!(session.recorded_events()[1].t, 2_050_000);
        assert!(!session.recorded_events()[1].press);
    }

    #[test]
    fn autoplay_books_its_sounds_on_the_schedule_axis_only() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        session.tick(SessionClock { audible_us: 0, scheduled_us: 3_000_000 }, &mut sink);
        assert!(!sink.sounds.is_empty(), "the whole booking horizon is scheduled at once");
        assert!(
            sink.sounds.iter().all(|sound| matches!(sound.at, SoundTime::Scheduled(_))),
            "an autoplay sound is booked ahead, never collapsed onto the current callback"
        );
        assert_eq!(session.judge().total_judged(), 0, "nothing inside the booking horizon may be judged yet");
    }

    #[test]
    fn judging_follows_the_audible_axis_however_far_the_scheduler_has_run_ahead() {
        let mut ahead = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut level = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        for step in 0..40 {
            let audible_us = step * 100_000;
            ahead.tick(SessionClock { audible_us, scheduled_us: audible_us + 500_000 }, &mut NullSink);
            level.tick(SessionClock::at(audible_us), &mut NullSink);
        }
        assert_eq!(ahead.summary(), level.summary(), "the scheduler's lookahead must not shift a judgement");
    }

    #[test]
    fn accompaniment_and_note_sounds_are_tagged_with_their_own_source() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01\r\n");
        let mut session = PlaySession::new(m, SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        session.tick(SessionClock::at(3_000_000), &mut sink);
        assert!(sink.sounds.iter().any(|sound| sound.source == PlaySource::Bgm), "the BGM channel is tagged as accompaniment");
        assert!(sink.sounds.iter().any(|sound| sound.source == PlaySource::Key), "an autoplayed note is tagged as a keysound");
    }

    #[test]
    fn every_emitted_sound_carries_the_neutral_voice_parameters() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        session.tick(SessionClock::at(3_000_000), &mut sink);
        session.press(0, 3_000_000, &mut sink);
        assert!(!sink.sounds.is_empty());
        for sound in &sink.sounds {
            assert_eq!(sound.gain, KEYSOUND_GAIN);
            assert_eq!(sound.pan, KEYSOUND_PAN);
            assert_eq!(sound.pitch, KEYSOUND_PITCH);
        }
    }

    #[test]
    fn a_null_sink_changes_nothing_about_the_judgement() {
        let mut heard = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut silent = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        heard.tick(SessionClock::at(3_000_000), &mut sink);
        silent.tick(SessionClock::at(3_000_000), &mut NullSink);
        assert!(!sink.sounds.is_empty(), "the recording sink did hear the run");
        assert_eq!(heard.summary(), silent.summary());
    }

    #[test]
    fn the_offset_step_is_the_references_rounded_thirtieth_of_the_error_taken_off() {
        use rbms_judge::Judge;
        let rows = [
            (0, 0),
            (14_999, 0),
            (15_000, -1),
            (44_999, -1),
            (45_000, -2),
            (74_999, -2),
            (75_000, -3),
            (135_000, -5),
            (150_000, -5),
            (-14_999, 0),
            (-15_000, 1),
            (-45_000, 2),
            (-150_000, 5),
        ];
        for (delta_us, step) in rows {
            for judge in [Judge::PerfectGreat, Judge::Great, Judge::Good] {
                assert_eq!(auto_adjust_step_ms(judge, delta_us), step, "{judge:?} at {delta_us} us (JudgeManager.java:700-701)");
            }
        }
    }

    #[test]
    fn a_judgement_worse_than_good_or_too_far_out_moves_nothing() {
        use rbms_judge::Judge;
        for judge in [Judge::Bad, Judge::Poor, Judge::Miss] {
            assert_eq!(auto_adjust_step_ms(judge, 60_000), 0, "{judge:?}");
            assert_eq!(auto_adjust_step_ms(judge, -60_000), 0, "{judge:?}");
        }
        assert_eq!(auto_adjust_step_ms(Judge::Good, AUTO_ADJUST_MAX_ABS_DELTA_US + 1), 0);
        assert_eq!(auto_adjust_step_ms(Judge::Good, -AUTO_ADJUST_MAX_ABS_DELTA_US - 1), 0);
        assert_eq!(auto_adjust_step_ms(Judge::PerfectGreat, i64::MAX), 0);
        assert_eq!(auto_adjust_step_ms(Judge::PerfectGreat, i64::MIN), 0);
    }

    #[test]
    fn an_early_input_has_the_notes_drawn_later_from_then_on() {
        let mut session = PlaySession::new(one_note(), SessionOptions { auto_calibration: true, ..Default::default() });
        let hit = session.press(0, 2_000_000 - 20_000, &mut NullSink).expect("the note is hit");
        assert_eq!(hit.delta_us, 20_000, "the input is judged at the instant it arrived");
        assert_eq!(session.judge_offset_ms(), -1, "20 ms early is the notes one millisecond later (JudgeManager.java:701)");
        assert_eq!(session.adjusted_inputs(), 1);
    }

    #[test]
    fn a_late_input_has_the_notes_drawn_earlier_from_then_on() {
        let mut session = PlaySession::new(one_note(), SessionOptions { auto_calibration: true, ..Default::default() });
        let hit = session.press(0, 2_000_000 + 20_000, &mut NullSink).expect("the note is hit");
        assert_eq!(hit.delta_us, -20_000);
        assert_eq!(session.judge_offset_ms(), 1);
    }

    #[test]
    fn the_offset_stays_put_while_it_is_not_asked_to_adjust_itself() {
        let mut session = PlaySession::new(one_note(), SessionOptions { auto_calibration: false, ..Default::default() });
        session.press(0, 2_000_000 - 20_000, &mut NullSink);
        assert_eq!(session.judge_offset_ms(), 0);
        assert_eq!(session.adjusted_inputs(), 0);
    }

    #[test]
    fn an_offset_changed_from_outside_is_the_one_the_run_adjusts_from() {
        let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: 3_000, auto_calibration: true, ..Default::default() });
        session.set_judge_offset_us(-7_000);
        assert_eq!(session.judge_offset_ms(), -7);
        assert_eq!(session.adjusted_inputs(), 0, "the run moved nothing by itself");
        session.press(0, 2_000_000 - 20_000, &mut NullSink);
        assert_eq!(session.judge_offset_ms(), -8);
    }

    #[test]
    fn a_recorded_input_is_judged_at_the_last_offset_stepped_in_before_it() {
        let steps = [
            ReplayOffsetStep { from_event: 0, offset_ms: 4 },
            ReplayOffsetStep { from_event: 2, offset_ms: 5 },
            ReplayOffsetStep { from_event: 2, offset_ms: 6 },
            ReplayOffsetStep { from_event: 9, offset_ms: -3 },
        ];
        let offsets: Vec<i64> = [0, 1, 2, 8, 9, 400].iter().map(|index| recorded_offset_us(&steps, *index, 99_000)).collect();
        assert_eq!(offsets, [4_000, 4_000, 6_000, 6_000, -3_000, -3_000]);
        assert_eq!(recorded_offset_us(&[], 5, 99_000), 99_000, "a replay with no steps is judged at its one offset");
        assert_eq!(recorded_offset_us(&steps[1..], 0, 99_000), 99_000, "an input before the first step keeps the replay's own");
    }

    #[test]
    fn the_analysis_rate_steps_between_its_limits() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        assert_eq!(session.analysis_rate(), 1.0);
        for _ in 0..40 {
            session.adjust_analysis_rate(1);
        }
        assert_eq!(session.analysis_rate(), ANALYSIS_RATE_MAX);
        for _ in 0..80 {
            session.adjust_analysis_rate(-1);
        }
        assert_eq!(session.analysis_rate(), ANALYSIS_RATE_MIN);
        assert!(session.analysis_manual(), "touching a playback control takes manual control");
    }

    #[test]
    fn a_paused_analysis_clock_stands_still() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        session.sync_analysis_position(1_000_000);
        assert_eq!(session.advance_analysis(16_000), 1_016_000);
        session.toggle_analysis_pause();
        assert_eq!(session.advance_analysis(16_000), 1_016_000, "a paused clock ignores the frame");
        session.toggle_analysis_pause();
        assert_eq!(session.advance_analysis(16_000), 1_032_000);
    }

    #[test]
    fn the_analysis_clock_scales_with_the_playback_rate_and_never_goes_negative() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        session.adjust_analysis_rate(-2);
        assert_eq!(session.analysis_rate(), 0.5);
        assert_eq!(session.advance_analysis(100_000), 50_000);
        assert_eq!(session.advance_analysis(-1_000_000), 0, "scrubbing back past the start stops at zero");
    }

    #[test]
    fn the_virtual_clock_only_follows_the_live_one_while_analysis_is_on() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: false, ..Default::default() });
        session.sync_analysis_position(5_000_000);
        assert_eq!(session.analysis_position_us(), 0);
        session.set_analysis_enabled(true);
        session.sync_analysis_position(5_000_000);
        assert_eq!(session.analysis_position_us(), 5_000_000);
    }

    #[test]
    fn the_overlay_keeps_only_the_most_recent_judged_inputs() {
        let mut marks = TimingMarks::default();
        for i in 0..(TIMING_MARK_CAPACITY as i64 + 5) {
            marks.push(TimingMark { lane: 0, delta_us: i, judge: 0 });
        }
        assert_eq!(marks.marks.len(), TIMING_MARK_CAPACITY);
        let recent = marks.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent.last().map(|mark| mark.delta_us), Some(TIMING_MARK_CAPACITY as i64 + 4), "newest last");
        assert_eq!(marks.recent(1_000).len(), TIMING_MARK_CAPACITY, "asking for more than there are is not an error");
    }

    #[test]
    fn the_bga_frame_holds_until_the_next_event_is_reached() {
        let m = model(b"#BPM 120\r\n#BMP01 a.png\r\n#BMP02 b.png\r\n#00104:01\r\n#00204:02\r\n");
        let mut session = PlaySession::new(m, SessionOptions { autoplay: true, ..Default::default() });
        assert_eq!(session.bga_frame(), BGA_FRAME_NONE, "nothing is shown before the first event");
        session.tick(SessionClock::at(1_999_999), &mut NullSink);
        assert_eq!(session.bga_frame(), BGA_FRAME_NONE, "the first event has not been reached yet");
        session.tick(SessionClock::at(2_000_000), &mut NullSink);
        assert_eq!(session.bga_frame(), 1, "the first frame is shown from the moment it is due");
        session.tick(SessionClock::at(3_000_000), &mut NullSink);
        assert_eq!(session.bga_frame(), 1, "it holds until the next event");
        session.tick(SessionClock::at(5_000_000), &mut NullSink);
        assert_eq!(session.bga_frame(), 2);
    }

    #[test]
    fn a_run_finishes_only_after_the_tail_silence() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let last = session.last_time_us();
        session.tick(SessionClock::at(last + PLAY_TAIL_US), &mut NullSink);
        assert!(!session.is_finished(last + PLAY_TAIL_US), "the tail is not over yet");
        assert!(session.is_finished(last + PLAY_TAIL_US + 1));
    }

    #[test]
    fn an_analysis_run_never_finishes_on_its_own() {
        let events = vec![ReplayEvent { t: 2_000_000, lane: 0, press: true, ..Default::default() }];
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, replay: Some(replay_of(events)), ..Default::default() });
        let past_the_end = session.last_time_us() + PLAY_TAIL_US * 4;
        session.tick(SessionClock::at(past_the_end), &mut NullSink);
        assert!(!session.is_finished(past_the_end), "the player is scrubbing, so the result screen must wait");
    }

    #[test]
    fn a_resolved_chart_reports_that_nothing_is_left_to_hit() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        assert!(!session.all_notes_resolved(), "nothing has been judged yet");
        session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
        assert!(session.all_notes_resolved());
    }

    #[test]
    fn a_chart_with_no_note_to_judge_has_nothing_left_to_hit_from_the_start() {
        let session = PlaySession::new(model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00101:01\r\n"), SessionOptions::default());
        assert_eq!(session.judge().total_notes(), 0, "the chart is background sound alone");
        assert!(session.all_notes_resolved(), "nothing is left to hit, so leaving it is ending it");
    }

    #[test]
    fn only_a_replay_run_can_be_scrubbed() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        session.seek(1_000_000);
        assert_eq!(session.analysis_position_us(), 0, "a live run has no recorded input to rebuild from");
        assert!(!session.analysis_manual());
    }

    #[test]
    fn a_session_reports_whether_it_is_reproducing_a_replay() {
        let live = PlaySession::new(one_note(), SessionOptions::default());
        assert!(!live.is_replay());
        let replayed = PlaySession::new(one_note(), SessionOptions { replay: Some(replay_of(Vec::new())), ..Default::default() });
        assert!(replayed.is_replay());
    }

    #[test]
    fn the_summary_reports_the_run_as_the_result_screen_reads_it() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n");
        let notes = rbms_chart::count_playable_notes(&m);
        let mut session = PlaySession::new(m, SessionOptions { autoplay: true, ..Default::default() });
        session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
        let summary = session.summary();
        assert_eq!(summary.total_notes, notes as u32);
        assert_eq!(summary.counts[0], notes as u32, "autoplay hits everything perfectly");
        assert_eq!(summary.ex_score, summary.max_ex_score);
        assert_eq!(summary.max_ex_score, summary.total_notes * 2);
        assert_eq!(summary.max_combo, summary.total_notes);
        assert_eq!(summary.min_bp, 0);
        assert_eq!(summary.total_judged, summary.total_notes);
        for judge in 0..summary.counts.len() {
            assert_eq!(summary.early[judge] + summary.late[judge], summary.counts[judge], "the early/late split covers every judgement");
        }
    }

    #[test]
    fn the_seed_the_run_was_laid_out_with_is_reported_back() {
        let session = PlaySession::new(one_note(), SessionOptions { seed: 0x5EED, ..Default::default() });
        assert_eq!(session.seed(), 0x5EED);
    }

    #[test]
    fn a_timing_error_lands_in_the_bucket_that_straddles_it() {
        let middle = TIMING_HIST_BINS / 2;
        assert_eq!(PlayInstrumentation::bin_of(0), middle, "a note hit exactly on time is the middle bucket");
        assert_eq!(PlayInstrumentation::bin_of(TIMING_HIST_BIN_US), middle + 1);
        assert_eq!(PlayInstrumentation::bin_of(-TIMING_HIST_BIN_US), middle - 1);
        assert_eq!(PlayInstrumentation::bin_of(TIMING_HIST_BIN_US - 1), middle, "a bucket holds the whole span up to the next one");
        assert_eq!(PlayInstrumentation::bin_of(-1), middle - 1, "and the one below it holds everything under the note");
    }

    /// An input further out than the window is counted at the end rather than dropped, so the
    /// histogram still totals the inputs that were judged.
    #[test]
    fn a_timing_error_past_the_window_is_counted_at_the_end_of_it() {
        assert_eq!(PlayInstrumentation::bin_of(TIMING_HIST_RANGE_US * 9), TIMING_HIST_BINS - 1);
        assert_eq!(PlayInstrumentation::bin_of(-TIMING_HIST_RANGE_US * 9), 0);
        assert_eq!(PlayInstrumentation::bin_of(i64::MAX), TIMING_HIST_BINS - 1);
        assert_eq!(PlayInstrumentation::bin_of(i64::MIN), 0);
    }

    #[test]
    fn a_fresh_run_has_measured_nothing() {
        let session = PlaySession::new(one_note(), SessionOptions::default());
        let inst = session.instrumentation();
        assert!(inst.gauge_series().is_empty());
        assert_eq!(inst.timing_hist().iter().sum::<u32>(), 0);
        assert_eq!(inst.judge_dist().iter().sum::<u32>(), 0);
        assert_eq!(inst.fast(), [0; LANE_KIND_COUNT]);
        assert_eq!(inst.slow(), [0; LANE_KIND_COUNT]);
    }

    /// A key and the turntable are counted apart: a turntable is thrown rather than pressed and
    /// drifts its own way, so a run's early/late reading is only useful split.
    #[test]
    fn early_and_late_hits_are_counted_apart_for_the_keys_and_the_turntable() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00116:01\r\n");
        let key_at = m.timelines.iter().flat_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).next().expect("a key note");
        let scratch_at = m.timelines.iter().flat_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).next().expect("a turntable note");
        let mut session = PlaySession::new(m, SessionOptions::default());
        session.press(0, key_at - 4_000, &mut NullSink);
        session.press(7, scratch_at + 4_000, &mut NullSink);
        let inst = session.instrumentation();
        assert_eq!(inst.fast()[KEY_LANE_KIND], 1, "the early key hit is in the key column");
        assert_eq!(inst.slow()[SCRATCH_LANE_KIND], 1, "the late turntable hit is in the turntable column");
        assert_eq!(inst.fast()[SCRATCH_LANE_KIND] + inst.slow()[KEY_LANE_KIND], 0, "a hit reached the wrong column");
    }

    /// A judgement with no timing — the sweep's miss, an empty poor — has no input to be early or
    /// late against, which is where the judge engine stops counting too.
    #[test]
    fn a_judgement_with_no_timing_reaches_neither_column() {
        let mut inst = PlayInstrumentation::default();
        for judge in [rbms_judge::Judge::Bad, rbms_judge::Judge::Poor, rbms_judge::Judge::Miss] {
            inst.record(judge, 1_000, false);
        }
        assert_eq!(inst.judge_dist().iter().sum::<u32>(), 3, "every judgement still reaches the distribution");
        assert_eq!(inst.fast()[KEY_LANE_KIND], 1, "the one judgement inside the timed range is counted");
        assert_eq!(inst.fast().iter().chain(inst.slow().iter()).sum::<u32>(), 1);
    }

    /// An auto-played lane is judged inside the engine and never produces an input, so the split —
    /// which counts what the *player* did — leaves it out where the engine's own totals include it.
    #[test]
    fn an_auto_played_lane_is_not_counted_as_something_the_player_hit() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        for step in 0..=40 {
            session.tick(SessionClock::at(step * 100_000), &mut NullSink);
        }
        let judged = session.judge().counts.iter().sum::<u32>();
        assert!(judged > 0, "the fixture did not autoplay anything");
        assert_eq!(session.instrumentation().fast(), [0; LANE_KIND_COUNT], "an auto-played note was counted as a player's hit");
        assert_eq!(session.instrumentation().slow(), [0; LANE_KIND_COUNT]);
    }

    /// A replayed run is judged inside the session rather than through the screen's key handler, so
    /// its split has to be counted here or every playback would report nothing.
    #[test]
    fn a_replayed_run_counts_its_early_and_late_hits_too() {
        let m = one_note();
        let at = m.timelines.iter().flat_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).next().expect("a note");
        let events = vec![ReplayEvent { t: at - 4_000, lane: 0, press: true, backward: false }];
        let mut session = PlaySession::new(m, SessionOptions { replay: Some(replay_of(events)), ..Default::default() });
        for step in 0..=(at / 100_000 + 10) {
            session.tick(SessionClock::at(step * 100_000), &mut NullSink);
        }
        let inst = session.instrumentation();
        assert_eq!(inst.fast()[KEY_LANE_KIND], 1, "the replayed hit was not counted");
        assert_eq!(inst.fast().iter().chain(inst.slow().iter()).sum::<u32>(), session.judge().fast + session.judge().slow);
    }

    fn plain_long_note() -> Model {
        model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n")
    }

    fn long_note_times(m: &Model) -> (i64, i64) {
        let times: Vec<i64> = m.timelines.iter().flat_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).collect();
        (times[0], times[1])
    }

    /// How late the head is taken, and how early the end is let go, in the runs below. The head is
    /// the worse of the two, which is the timing the note is judged with.
    const LATE_HEAD_US: i64 = 40_000;
    const EARLY_END_US: i64 = 30_000;

    /// A plain long note is one judgment, at its end. The press that takes its head used to be
    /// tallied as a judgment too, so the distribution, the FAST/SLOW split and the offset an
    /// adjusting run moves all counted every long note twice.
    #[test]
    fn plain_long_notes_are_counted_once_by_instrumentation_and_auto_adjust() {
        let m = plain_long_note();
        let (head_us, end_us) = long_note_times(&m);
        let mut session = PlaySession::new(m, SessionOptions { auto_calibration: true, ..Default::default() });
        let head = session.press(0, head_us + LATE_HEAD_US, &mut NullSink).expect("the head is taken");
        assert!(!head.counted, "the head only starts the hold");
        assert_eq!(session.instrumentation().judge_dist().iter().sum::<u32>(), 0, "the head alone is no judgment");
        assert_eq!((session.adjusted_inputs(), session.judge_offset_ms()), (0, 0), "and moves no offset");

        let end = session.release(0, end_us - EARLY_END_US).expect("the release judges the note");
        assert!(end.counted);
        let inst = session.instrumentation();
        assert_eq!(inst.judge_dist().iter().sum::<u32>(), 1, "one judgment in the distribution");
        assert_eq!(inst.judge_dist()[end.judge as usize], 1);
        assert_eq!(inst.timing_hist().iter().sum::<u32>(), 1);
        assert_eq!(inst.fast().iter().chain(inst.slow().iter()).sum::<u32>(), session.judge().fast + session.judge().slow, "FAST/SLOW agree with the engine");
        assert_eq!(session.adjusted_inputs(), 1, "the offset moved once, for the one judgment");
        assert_eq!(session.judge_offset_ms(), auto_adjust_step_ms(end.judge, end.delta_us));
    }

    /// A long note held through its end is judged by the clock, with the timing of its head, and
    /// that judgment is counted once as well: no input of the player's brought it back.
    #[test]
    fn a_plain_long_note_held_through_its_end_is_counted_once() {
        let m = plain_long_note();
        let (head_us, end_us) = long_note_times(&m);
        let mut session = PlaySession::new(m, SessionOptions { auto_calibration: true, ..Default::default() });
        session.press(0, head_us + LATE_HEAD_US, &mut NullSink);
        for step in 0..=((end_us + 200_000) / 50_000) {
            session.tick(SessionClock::at(step * 50_000), &mut NullSink);
        }
        let judge = session.judge();
        assert_eq!(judge.counts.iter().sum::<u32>(), 1, "the engine counted the note once");
        let inst = session.instrumentation();
        assert_eq!(inst.judge_dist().iter().sum::<u32>(), 1);
        assert_eq!(inst.fast().iter().chain(inst.slow().iter()).sum::<u32>(), judge.fast + judge.slow);
        assert_eq!(session.adjusted_inputs(), 1);
    }

    /// A miss swept up by the clock has no timing and is nobody's input, so it is left out of the
    /// tallies as before.
    #[test]
    fn a_long_note_swept_as_a_miss_reaches_no_tally() {
        let m = plain_long_note();
        let (_, end_us) = long_note_times(&m);
        let mut session = PlaySession::new(m, SessionOptions { auto_calibration: true, ..Default::default() });
        for step in 0..=((end_us + 1_000_000) / 100_000) {
            session.tick(SessionClock::at(step * 100_000), &mut NullSink);
        }
        assert_eq!(session.judge().counts[Judge::Poor as usize], 1);
        assert_eq!(session.instrumentation().judge_dist().iter().sum::<u32>(), 0);
        assert_eq!(session.adjusted_inputs(), 0);
    }

    /// A run that plays itself makes no input, so none of its judgments are tallied or move the
    /// offset, long notes included.
    #[test]
    fn an_autoplayed_long_note_is_not_tallied_as_an_input() {
        let mut session = PlaySession::new(plain_long_note(), SessionOptions { autoplay: true, auto_calibration: true, ..Default::default() });
        for step in 0..=60 {
            session.tick(SessionClock::at(step * 100_000), &mut NullSink);
        }
        assert_eq!(session.judge().counts[0], 1, "the autoplay took the note");
        assert_eq!(session.instrumentation().judge_dist().iter().sum::<u32>(), 0);
        assert_eq!(session.adjusted_inputs(), 0);
    }

    #[test]
    fn the_gauge_is_sampled_once_per_second_of_song_time() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        for step in 0..=6 {
            session.tick(SessionClock::at(step * GAUGE_SAMPLE_INTERVAL_US / 2), &mut NullSink);
        }
        assert_eq!(session.instrumentation().gauge_series().len(), 4, "half-second frames still leave one sample a second");
        for value in session.instrumentation().gauge_series() {
            assert!((0.0..=100.0).contains(value), "a gauge sample is a percentage: {value}");
        }
    }

    /// A very long run must not grow the series without end, which is what a plain push per second
    /// would do.
    #[test]
    fn the_gauge_series_stops_growing_at_its_capacity() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        for step in 0..(GAUGE_SERIES_CAPACITY as i64 + 50) {
            session.tick(SessionClock::at(step * GAUGE_SAMPLE_INTERVAL_US), &mut NullSink);
        }
        assert_eq!(session.instrumentation().gauge_series().len(), GAUGE_SERIES_CAPACITY);
    }

    #[test]
    fn a_judged_press_is_counted_in_both_tallies() {
        let mut session = PlaySession::new(one_note(), SessionOptions::default());
        let at = session.last_time_us();
        let judged = session.press(0, at, &mut NullSink).expect("a press on the note is judged");
        let inst = session.instrumentation();
        assert_eq!(inst.judge_dist()[judged.judge as usize], 1, "the judgement it took is tallied");
        assert_eq!(inst.judge_dist().iter().sum::<u32>(), 1, "and nothing else is");
        assert_eq!(inst.timing_hist()[PlayInstrumentation::bin_of(judged.delta_us)], 1);
        assert_eq!(inst.timing_hist().iter().sum::<u32>(), 1);
    }

    /// Scrubbing rebuilds the run from the recorded inputs, so what was measured before the seek
    /// describes a run that no longer exists, and what is measured after it is the rebuilt run's up
    /// to where the scrub landed.
    #[test]
    fn scrubbing_a_replay_throws_away_what_was_measured_before_it() {
        let at = PlaySession::new(one_note(), SessionOptions::default()).last_time_us();
        let events = vec![ReplayEvent { t: at, lane: 0, press: true, backward: false }];
        let options = SessionOptions { analysis: true, replay: Some(replay_of(events)), ..Default::default() };
        let mut session = PlaySession::new(one_note(), options);
        session.tick(SessionClock::at(at + 1), &mut NullSink);
        let measured = session.instrumentation().judge_dist().iter().sum::<u32>();
        assert!(measured > 0, "the replayed input was measured");
        session.seek(0);
        let inst = session.instrumentation();
        assert_eq!(inst.gauge_series().len(), 1, "the one sample at the start of the song");
        assert_eq!(inst.timing_hist().iter().sum::<u32>(), 0);
        assert_eq!(inst.judge_dist().iter().sum::<u32>(), 0);
        session.seek(at + 1);
        assert_eq!(session.instrumentation().judge_dist().iter().sum::<u32>(), measured, "scrubbing past the input measures it again, once");
    }
    /// A run played by hand with a 空POOR beside every note it hits records four inputs a note, and
    /// the room made when the run started covers them: the recording is never grown, and so never
    /// moved, in the middle of play.
    #[test]
    fn a_run_played_by_hand_never_grows_its_recording() {
        const MEASURES: usize = 200;
        const SLOTS: usize = 8;
        const KEY_CHANNELS: [u8; 7] = [11, 12, 13, 14, 15, 18, 19];
        const RELEASE_AFTER_US: i64 = 10_000;
        const EMPTY_POOR_AFTER_US: i64 = 20_000;
        let mut bms = String::from("#BPM 120\r\n#WAV01 a.wav\r\n");
        for measure in 0..MEASURES {
            for slot in 0..SLOTS {
                let channel = KEY_CHANNELS[(measure * SLOTS + slot) % KEY_CHANNELS.len()];
                let objects: String = (0..SLOTS).map(|at| if at == slot { "01" } else { "00" }).collect();
                bms.push_str(&format!("#{:03}{channel}:{objects}\r\n", measure + 1));
            }
        }
        let chart = model(bms.as_bytes());
        let mut hits: Vec<(i64, usize)> = chart
            .timelines
            .iter()
            .flat_map(|tl| tl.notes.iter().enumerate().filter_map(move |(lane, note)| note.as_ref().map(|_| (tl.time_us, lane))))
            .collect();
        hits.sort();
        let mut session = PlaySession::new(chart, SessionOptions::default());
        let (recording, capacity) = (session.recorded_events().as_ptr(), session.recording.capacity());
        for &(at, lane) in &hits {
            session.press(lane, at, &mut NullSink);
            session.release(lane, at + RELEASE_AFTER_US);
            session.press(lane, at + EMPTY_POOR_AFTER_US, &mut NullSink);
            session.release(lane, at + EMPTY_POOR_AFTER_US + RELEASE_AFTER_US);
        }
        assert_eq!(hits.len(), MEASURES * SLOTS);
        assert_eq!(session.judge().empty_poor as usize, hits.len(), "every second press reached only the 空POOR band");
        assert_eq!(session.recorded_events().len(), RECORDED_INPUTS_PER_NOTE * hits.len());
        assert_eq!(session.recording.capacity(), capacity, "the recording was grown during the run");
        assert_eq!(session.recorded_events().as_ptr(), recording, "the recording was moved during the run");
    }
}
