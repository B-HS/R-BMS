//! What a finished run leaves behind for a result screen a skin draws.
//!
//! The built-in result screen reads [`PlayInstrumentation`](crate::PlayInstrumentation), which counts
//! the inputs the player made. A skin written for the reference implementation asks different
//! questions — every gauge's history rather than the selected one's, the timing of every note
//! rather than of every input, rates measured against the notes that have gone by — and each of
//! them has a definition in the reference that the existing tallies do not share. They are kept
//! here, apart from those tallies, so nothing the built-in screen draws changes meaning.
//!
//! Everything in a [`PlayRecord`] except the gauge history is read back off the judge engine when
//! the record is taken, so a replay that reproduces the judgements reproduces the record. The gauge
//! history is sampled while the run is going and follows the frames it was sampled on.

use rbms_judge::gauge::{GaugeIndex, GrooveGauge};
use rbms_judge::{JudgeEngine, NoteMark};
use rbms_model::Model;

use crate::session::PlaySummary;

/// Song time between two gauge history samples, in milliseconds (`BMSPlayer.java:632`).
pub const GAUGE_LOG_INTERVAL_MS: i64 = 500;

/// How long past its last note the reference keeps a chart in its playing state, in milliseconds
/// (`BMSPlayer.TIME_MARGIN`, `BMSPlayer.java:65`).
pub const PLAY_TIME_MARGIN_MS: i64 = 5_000;

/// How far either side of a note the timing distribution reaches, in milliseconds
/// (`AbstractResult.distRange`, `AbstractResult.java:52`). A note judged further out is dropped.
pub const TIMING_DISTRIBUTION_RANGE_MS: i64 = 150;

/// How many one-millisecond buckets the timing distribution holds: the range either side of the
/// note plus the note itself (`AbstractResult.java:285`).
pub const TIMING_DISTRIBUTION_BINS: usize = (TIMING_DISTRIBUTION_RANGE_MS * 2 + 1) as usize;

/// The timing error an unjudged or missed note is charged in the average duration, in
/// microseconds (`BMSPlayer.java:923`).
pub const UNJUDGED_DURATION_US: i64 = 1_000_000;

/// The average duration of a chart with no judged object in it: the reference's own initial value
/// of `ScoreData.avgjudge` (`ScoreData.java:87`), which nothing would have replaced.
pub const AVG_DURATION_ABSENT_US: i64 = i64::MAX;

/// The mean of an empty timing distribution, the value `TimingDistribution.init` leaves behind
/// (`AbstractResult.java:308`).
pub const TIMING_AVERAGE_ABSENT: f32 = f32::MAX;

/// The standard deviation of an empty timing distribution (`AbstractResult.java:309`).
pub const TIMING_STD_DEV_ABSENT: f32 = -1.0;

/// How many kinds a judge graph of type 1 sorts a note into: unjudged, then PGREAT to POOR
/// (`SkinNoteDistributionGraph.java:66`).
pub const JUDGE_SECOND_KINDS: usize = 6;

/// How many kinds a judge graph of type 2 sorts a note into: unjudged, PGREAT, then GREAT to POOR
/// early and GREAT to POOR late (`SkinNoteDistributionGraph.java:66`).
pub const TIMING_SECOND_KINDS: usize = 10;

/// How many rank steps the reference divides a score rate into: AAA is step 24, AA 21, A 18 and so
/// on down in threes (`ScoreDataProperty.java:35`).
pub const RANK_STEP_COUNT: usize = 27;

/// EX score a PGREAT is worth, which is what makes twice the note count the most a run can hold.
const EX_PER_NOTE: u32 = 2;

/// The first judge code a FAST/SLOW total counts. PGREAT is left out (`IntegerPropertyFactory.java:449`).
const FIRST_TIMED_TOTAL_JUDGE: usize = 1;

/// The highest `Note.getState()` whose own timing counts towards the average duration: PGREAT to
/// BAD. A 見逃し POOR is charged the flat miss duration instead (`BMSPlayer.java:923`).
const LAST_TIMED_STATE: u8 = 4;

/// The highest `Note.getState()` a judge graph of type 2 keeps in one bucket whatever its timing:
/// unjudged and PGREAT (`SkinNoteDistributionGraph.java:422`).
const LAST_UNSPLIT_STATE: u8 = 1;

/// How far a late judgement's bucket sits past the early one of the same judge in a judge graph of
/// type 2 (`SkinNoteDistributionGraph.java:425`).
const LATE_BUCKET_OFFSET: usize = 4;

/// What a line's BGA and layer fields hold when the line changes neither.
const NO_BGA_EVENT: i32 = -1;

const MICROS_PER_MILLI: i64 = 1_000;
const MILLIS_PER_SECOND: i64 = 1_000;

/// Chart time in whole milliseconds, the reference's `(int) (microtime / 1000)`.
fn millis(time_us: i64) -> i64 {
    time_us / MICROS_PER_MILLI
}

/// `BMSModel.getLastNoteTime()`: the time of the last line that holds a note in any lane, in
/// milliseconds. Mines and long-note ends are notes here; BGM and BGA events are not.
pub fn last_note_time_ms(model: &Model) -> i64 {
    model.timelines.iter().rev().find(|line| line.notes.iter().any(Option::is_some)).map_or(0, |line| millis(line.time_us))
}

/// `BMSModel.getLastTime()`: the time of the last line that holds anything at all — a note, a
/// hidden note, a BGM sound or a BGA change — in milliseconds.
pub fn last_event_time_ms(model: &Model) -> i64 {
    model
        .timelines
        .iter()
        .rev()
        .find(|line| {
            line.notes.iter().any(Option::is_some)
                || line.hidden.iter().any(Option::is_some)
                || !line.bgnotes.is_empty()
                || line.bga != NO_BGA_EVENT
                || line.layer != NO_BGA_EVENT
        })
        .map_or(0, |line| millis(line.time_us))
}

/// The reference's `playtime`: how long the chart stays in its playing state, in milliseconds on
/// the song clock (`BMSPlayer.java:198`). An autoplay run waits for the last sound; a played one
/// only for the last note.
pub fn play_time_ms(model: &Model, autoplay: bool) -> i64 {
    let last = if autoplay { last_event_time_ms(model) } else { last_note_time_ms(model) };
    last + PLAY_TIME_MARGIN_MS
}

/// The history of all nine gauges through a run, one value per [`GAUGE_LOG_INTERVAL_MS`] of song
/// time, oldest first (`BMSPlayer.gaugelog`, handed to the result as `resource.getGauge()`).
///
/// Every gauge has the same number of samples. A run that failed is padded with zeroes out to the
/// end of the chart, so a gauge graph drawn from it falls to the floor where the run stopped
/// rather than stretching what was played across the whole width.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GaugeLog {
    samples: [Vec<f32>; GaugeIndex::COUNT],
}

impl GaugeLog {
    /// One gauge's history, oldest first.
    pub fn of(&self, index: GaugeIndex) -> &[f32] {
        &self.samples[index.index()]
    }

    /// How many samples each gauge holds.
    pub fn len(&self) -> usize {
        self.samples[0].len()
    }

    /// Whether nothing was sampled.
    pub fn is_empty(&self) -> bool {
        self.samples[0].is_empty()
    }

    /// The value one gauge ended on, which is what the result screen reports as the gauge value
    /// once the player has switched the graph to that gauge (`FloatPropertyFactory.java:449-458`).
    pub fn last(&self, index: GaugeIndex) -> Option<f32> {
        self.of(index).last().copied()
    }
}

/// The gauge history while it is still being written.
///
/// It follows the reference's playing state: a sample is due whenever the series is no longer than
/// the song time divided by the interval, at most one a frame, from the moment the song clock
/// starts until the chart's play time has gone by or the gauge has failed the run
/// (`BMSPlayer.java:625-635, 651-662, 679-687`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GaugeLogRecorder {
    log: GaugeLog,
    play_time_ms: i64,
    failed_at_ms: Option<i64>,
    finished: bool,
}

impl GaugeLogRecorder {
    pub(crate) fn new(play_time_ms: i64) -> Self {
        GaugeLogRecorder { log: GaugeLog::default(), play_time_ms, failed_at_ms: None, finished: false }
    }

    /// One frame of the playing state at `song_us`, after the frame's judgements have reached the
    /// gauges. `failed` is whether the run has just been ended by an empty gauge.
    pub(crate) fn frame(&mut self, song_us: i64, gauge: &GrooveGauge, failed: bool) {
        if self.finished || self.failed_at_ms.is_some() || song_us < 0 {
            return;
        }
        let play_ms = millis(song_us);
        if self.log.len() as i64 <= play_ms / GAUGE_LOG_INTERVAL_MS {
            for index in GaugeIndex::ALL {
                self.log.samples[index.index()].push(gauge.value_at(index));
            }
        }
        if self.play_time_ms < play_ms {
            self.finished = true;
        } else if failed {
            self.failed_at_ms = Some(play_ms);
        }
    }

    /// Song time at which an empty gauge ended the run, in milliseconds.
    pub(crate) fn failed_at_ms(&self) -> Option<i64> {
        self.failed_at_ms
    }

    pub(crate) fn play_time_ms(&self) -> i64 {
        self.play_time_ms
    }

    /// The history as the result screen receives it: what was sampled, and for a failed run a zero
    /// for every interval from the failure to half a second past the play time
    /// (`BMSPlayer.java:723-729`).
    pub(crate) fn closed(&self) -> GaugeLog {
        let mut log = self.log.clone();
        if let Some(failed_at_ms) = self.failed_at_ms {
            let mut at_ms = failed_at_ms;
            while at_ms < self.play_time_ms + GAUGE_LOG_INTERVAL_MS {
                for samples in log.samples.iter_mut() {
                    samples.push(0.0);
                }
                at_ms += GAUGE_LOG_INTERVAL_MS;
            }
        }
        log
    }

    pub(crate) fn clear(&mut self) {
        *self = GaugeLogRecorder::new(self.play_time_ms);
    }
}

/// The bounds one gauge moves between, which a gauge graph needs to place a history on its axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeBounds {
    pub min: f32,
    pub max: f32,
    /// The value the gauge has to hold at the end to clear.
    pub border: f32,
}

/// How a run's notes were timed, in one-millisecond buckets either side of the note
/// (`AbstractResult.TimingDistribution`).
///
/// It is built from the notes, not from the inputs: each judged note contributes the timing of the
/// last judgement it took, in whole milliseconds, and one judged further out than
/// [`TIMING_DISTRIBUTION_RANGE_MS`] contributes nothing. Positive is early.
#[derive(Debug, Clone, PartialEq)]
pub struct TimingDistribution {
    dist: Vec<u32>,
    average: f32,
    std_dev: f32,
}

impl TimingDistribution {
    fn of(play_times_ms: impl Iterator<Item = i64>) -> Self {
        let mut dist = vec![0u32; TIMING_DISTRIBUTION_BINS];
        for timing_ms in play_times_ms {
            if (-TIMING_DISTRIBUTION_RANGE_MS..=TIMING_DISTRIBUTION_RANGE_MS).contains(&timing_ms) {
                dist[(timing_ms + TIMING_DISTRIBUTION_RANGE_MS) as usize] += 1;
            }
        }
        let mut count = 0i32;
        let mut sum = 0i32;
        for (bin, hits) in dist.iter().enumerate() {
            count += *hits as i32;
            sum += *hits as i32 * (bin as i32 - TIMING_DISTRIBUTION_RANGE_MS as i32);
        }
        if count == 0 {
            return TimingDistribution { dist, average: TIMING_AVERAGE_ABSENT, std_dev: TIMING_STD_DEV_ABSENT };
        }
        let average = sum as f32 / count as f32;
        let mut squares = 0.0f32;
        for (bin, hits) in dist.iter().enumerate() {
            let offset = (bin as i32 - TIMING_DISTRIBUTION_RANGE_MS as i32) as f32 - average;
            squares += *hits as f32 * offset * offset;
        }
        let std_dev = f64::from(squares / count as f32).sqrt() as f32;
        TimingDistribution { dist, average, std_dev }
    }

    /// How many notes landed in each bucket, latest first: bucket 0 is
    /// [`TIMING_DISTRIBUTION_RANGE_MS`] late, the middle one is on the note.
    pub fn buckets(&self) -> &[u32] {
        &self.dist
    }

    /// The bucket a note hit exactly on time lands in (`TimingDistribution.getArrayCenter`).
    pub fn center(&self) -> usize {
        TIMING_DISTRIBUTION_RANGE_MS as usize
    }

    /// Mean timing in milliseconds, positive when early; [`TIMING_AVERAGE_ABSENT`] when no note
    /// landed inside the range.
    pub fn average(&self) -> f32 {
        self.average
    }

    /// Standard deviation of the timing in milliseconds; [`TIMING_STD_DEV_ABSENT`] when no note
    /// landed inside the range.
    pub fn std_dev(&self) -> f32 {
        self.std_dev
    }
}

/// How the notes of each second of the chart were judged, for the two judge graphs that plot the
/// run rather than the chart (`SkinNoteDistributionGraph` type 1 and type 2).
///
/// A note is counted in the second its own chart time falls in, not the second it was hit in. Both
/// tables have one row per second up to the chart's last event.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JudgeSeconds {
    /// Type 1: `[unjudged, PGREAT, GREAT, GOOD, BAD, POOR]` per second.
    pub by_judge: Vec<[u32; JUDGE_SECOND_KINDS]>,
    /// Type 2: `[unjudged, PGREAT, GREAT early, GOOD early, BAD early, POOR early, GREAT late,
    /// GOOD late, BAD late, POOR late]` per second. A judgement inside one millisecond of the note
    /// counts as early.
    pub by_timing: Vec<[u32; TIMING_SECOND_KINDS]>,
}

impl JudgeSeconds {
    fn of(marks: &[NoteMark], last_event_ms: i64) -> Self {
        let seconds = (last_event_ms / MILLIS_PER_SECOND + 1) as usize;
        let mut by_judge = vec![[0u32; JUDGE_SECOND_KINDS]; seconds];
        let mut by_timing = vec![[0u32; TIMING_SECOND_KINDS]; seconds];
        for mark in marks {
            let second = (millis(mark.time_us) / MILLIS_PER_SECOND) as usize;
            if second >= seconds {
                continue;
            }
            let state = mark.state as usize;
            if let Some(slot) = by_judge[second].get_mut(state) {
                *slot += 1;
            }
            let early = millis(mark.play_time_us) >= 0;
            let bucket = if mark.state <= LAST_UNSPLIT_STATE || early { state } else { state + LATE_BUCKET_OFFSET };
            if let Some(slot) = by_timing[second].get_mut(bucket) {
                *slot += 1;
            }
        }
        JudgeSeconds { by_judge, by_timing }
    }
}

/// A score measured against the notes that have gone by, which is how the reference rates a run
/// that is still going (`ScoreDataProperty.update(score, notes)`).
///
/// On the result screen every note has gone by, so the two rates agree. During play they do not:
/// a run that has hit every note so far stands at 100% and MAX, not at the fraction of the chart it
/// has reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreProgress {
    pub ex_score: u32,
    /// Notes resolved so far, the reference's `passnotes`. An empty POOR resolves none.
    pub pass_notes: u32,
    pub total_notes: u32,
}

impl ScoreProgress {
    /// The most EX the notes gone by could have given.
    pub fn pass_max_ex(&self) -> u32 {
        self.pass_notes * EX_PER_NOTE
    }

    /// The most EX the whole chart can give.
    pub fn max_ex(&self) -> u32 {
        self.total_notes * EX_PER_NOTE
    }

    /// EX over the notes gone by, 1 before any has (`ScoreDataProperty.java:97`).
    pub fn now_rate(&self) -> f32 {
        rate_of(self.ex_score, self.pass_notes)
    }

    /// EX over the whole chart, 1 for a chart with no notes (`ScoreDataProperty.java:94`).
    pub fn rate(&self) -> f32 {
        rate_of(self.ex_score, self.total_notes)
    }

    /// Whether the run currently stands at rank step `step` or better, of [`RANK_STEP_COUNT`]
    /// (`ScoreDataProperty.java:110-112`).
    pub fn qualifies_now_rank(&self, step: usize) -> bool {
        self.total_notes != 0 && self.now_rate() >= rank_floor(step)
    }

    /// Whether the run has already secured rank step `step` over the whole chart
    /// (`ScoreDataProperty.java:101-102`).
    pub fn qualifies_rank(&self, step: usize) -> bool {
        self.total_notes != 0 && self.rate() >= rank_floor(step)
    }

    /// Where a standing score of `ex` over the whole chart would be by now, had it been earned
    /// evenly: the pace the run is compared against when no note-by-note record of that score is
    /// kept (`ScoreDataProperty.java:119`).
    pub fn paced(&self, ex: u32) -> u32 {
        if self.total_notes == 0 {
            return 0;
        }
        (u64::from(ex) * u64::from(self.pass_notes) / u64::from(self.total_notes)) as u32
    }
}

fn rate_of(ex_score: u32, notes: u32) -> f32 {
    if notes == 0 { 1.0 } else { ex_score as f32 / (notes * EX_PER_NOTE) as f32 }
}

fn rank_floor(step: usize) -> f32 {
    step as f32 / RANK_STEP_COUNT as f32
}

/// Everything a skin's result screen asks about a finished run that the built-in screen's own
/// tallies do not answer, each to the reference's definition.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayRecord {
    /// The judge engine's own tally. Its `early`/`late` are the reference's per-judge split
    /// (`ScoreData.epg` to `lms`): every counted judgement, sweeps and auto-played lanes included,
    /// early when the input came at or before the note.
    pub summary: PlaySummary,
    pub gauge_log: GaugeLog,
    /// The bounds of each of the nine gauges, indexed by [`GaugeIndex::index`].
    pub gauge_bounds: [GaugeBounds; GaugeIndex::COUNT],
    /// The gauge the run ended on, course gauges included. It is the gauge a result screen opens
    /// its graph on (`MusicResult.java:70`).
    pub finished_gauge: GaugeIndex,
    /// Song time at which an empty gauge ended the run, in milliseconds.
    pub failed_at_ms: Option<i64>,
    /// How long the chart stays in its playing state, in milliseconds (`BMSPlayer.playtime`).
    pub play_time_ms: i64,
    pub timing: TimingDistribution,
    /// `ScoreData.avgjudge`: the mean distance of each note from its input in microseconds, a note
    /// that was missed or never reached counting as [`UNJUDGED_DURATION_US`]. Not the engine's
    /// `avg_judge_us`, which is a signed mean over the hits alone.
    pub avg_duration_us: i64,
    /// `ScoreData.totalDuration`: the sum the average is taken from.
    pub total_duration_us: i64,
    pub seconds: JudgeSeconds,
}

impl PlayRecord {
    pub(crate) fn of(model: &Model, judge: &JudgeEngine, summary: PlaySummary, gauge_log: &GaugeLogRecorder) -> Self {
        let marks: Vec<NoteMark> = judge.note_marks().filter(|mark| mark.charge || !mark.long_end).collect();
        let timing = TimingDistribution::of(marks.iter().filter(|mark| mark.state > 0).map(|mark| millis(mark.play_time_us)));
        let total_duration_us: i64 =
            marks.iter().map(|mark| if (1..=LAST_TIMED_STATE).contains(&mark.state) { mark.play_time_us.abs() } else { UNJUDGED_DURATION_US }).sum();
        let avg_duration_us = if marks.is_empty() { AVG_DURATION_ABSENT_US } else { total_duration_us / marks.len() as i64 };
        let gauge_bounds = GaugeIndex::ALL.map(|index| {
            let gauge = judge.gauge.gauge_at(index);
            GaugeBounds { min: gauge.min(), max: gauge.max(), border: gauge.border() }
        });
        PlayRecord {
            summary,
            gauge_log: gauge_log.closed(),
            gauge_bounds,
            finished_gauge: judge.gauge.selected_index(),
            failed_at_ms: gauge_log.failed_at_ms(),
            play_time_ms: gauge_log.play_time_ms(),
            timing,
            avg_duration_us,
            total_duration_us,
            seconds: JudgeSeconds::of(&marks, last_event_time_ms(model)),
        }
    }

    /// Judgements that came early, GREAT to empty POOR. PGREAT is left out, and a 見逃し POOR and an
    /// empty POOR are counted, which is neither the engine's `fast` nor the built-in screen's
    /// per-lane FAST (`IntegerPropertyFactory.java:447-453`).
    pub fn total_early(&self) -> u32 {
        self.summary.early[FIRST_TIMED_TOTAL_JUDGE..].iter().sum()
    }

    /// Judgements that came late, GREAT to empty POOR (`IntegerPropertyFactory.java:454-460`).
    pub fn total_late(&self) -> u32 {
        self.summary.late[FIRST_TIMED_TOTAL_JUDGE..].iter().sum()
    }

    /// The run's score against the notes that went by. Once every note has, this is the score
    /// against the whole chart.
    pub fn progress(&self) -> ScoreProgress {
        ScoreProgress { ex_score: self.summary.ex_score, pass_notes: self.summary.total_judged, total_notes: self.summary.total_notes }
    }

    /// `ScoreData.minbp` as the reference records it: bad, poor and empty poor, plus every note the
    /// run never reached (`BMSPlayer.java:909`). The summary's own `min_bp` leaves the unreached
    /// notes out.
    pub fn min_bp_with_unreached(&self) -> u32 {
        self.summary.min_bp + self.summary.total_notes.saturating_sub(self.summary.total_judged)
    }
}

#[cfg(test)]
mod tests;
