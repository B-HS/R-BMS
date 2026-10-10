//! What a finished run leaves for the result screen of a skin to read.
//!
//! A [`ResultSnapshot`] is made once, when the result screen is entered, from values that already
//! exist by then: the run's own record, the best the player had before it, the target it was paced
//! against and the settings it was played with. It owns everything it holds, so the screen keeps it
//! for as long as it is up and every frame lends it to the host with what the screen holds itself
//! ([`FinishedRun`]).
//!
//! Making one is a pure function of a [`ResultInput`]: no screen, no player state and no clock.
//! What changes while the screen is up -- the gauge the graph is switched to, the replay slots, how
//! far the ranking is scrolled -- is not in it. The screen owns that and says it in a
//! [`ResultScene`] each frame.

use rbms_judge::ClearType;
use rbms_judge::gauge::GaugeIndex;
use rbms_model::Mode;
use rbms_play::{GaugeLog, PlayRecord, TIMING_DISTRIBUTION_RANGE_MS};
use rbms_render::result::TargetView;
use rbms_store::{ScoreBook, ScoreRecord};

use crate::skin_host::ResultScene;
use crate::skin_host::options::PlayedOptions;
use crate::skin_host::score::standing::{PointFamily, ScoreSheet, TargetPace};
use crate::skin_host::score::{GaugeReading, RunScore};

/// The first of the judgements that count as bad-poor, in the order a score keeps its counts: BAD,
/// then POOR and MISS after it (`ScoreData.minbp` is the three added up).
const FIRST_BAD_POOR_JUDGEMENT: usize = 3;

/// How many replay slots a result screen offers (`AbstractResult.REPLAY_SIZE`).
pub const REPLAY_SLOT_COUNT: usize = 4;

/// What a replay slot holds (`AbstractResult.ReplayStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplaySlot {
    /// Nothing is stored there (`NOT_EXIST`).
    #[default]
    Missing,
    /// A replay of an earlier run is stored there (`EXIST`).
    Exists,
    /// This run's replay was stored there just now (`SAVED`).
    Saved,
}

/// The best the player had on a chart before the run that has just ended (`oldscore`).
///
/// The reference keeps one `ScoreData` per chart that is the best of each figure on its own, and a
/// chart never played reads as a fresh one: nothing scored, no combo, the worst possible bad-poor
/// count and the lamp of a chart that was never played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreviousScore {
    pub ex_score: u32,
    pub max_combo: u32,
    /// The fewest bad, poor and miss judgements of any run, or `None` for the reference's
    /// `Integer.MAX_VALUE` of a chart that was never played.
    pub min_bp: Option<u32>,
    /// The best lamp, as the reference numbers lamps.
    pub clear: u8,
}

impl PreviousScore {
    /// The best of the runs `book` holds of one chart, played under one LN MODE.
    ///
    /// An assisted run is history only: it can raise the lamp and nothing else
    /// (`ScoreData.update(newscore, updateScore)`). The bad-poor count is the judgements alone, as
    /// the book keeps no more than that of a run.
    ///
    /// This is read before the run that has just ended is added to the book.
    pub fn of_book(book: &ScoreBook, md5: &str, ln_mode: &str) -> PreviousScore {
        let scoring: Vec<&ScoreRecord> = book.for_md5(md5).into_iter().filter(|record| !record.assisted && record.ln_mode == ln_mode).collect();
        PreviousScore {
            ex_score: book.best_ex_for_md5_in_ln_mode(md5, ln_mode).unwrap_or_default(),
            max_combo: scoring.iter().map(|record| record.max_combo).max().unwrap_or_default(),
            min_bp: scoring.iter().map(|record| record.counts[FIRST_BAD_POOR_JUDGEMENT..].iter().sum()).min(),
            clear: book.best_clear_for_md5_in_ln_mode(md5, ln_mode).unwrap_or_default(),
        }
    }
}

/// The last value of each gauge's history.
///
/// The gauge numbers on a result screen read the end of the history of the gauge the graph is
/// switched to, not the gauge the run was played on
/// (`IntegerPropertyFactory.createGrooveGaugeProperty`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GaugeEnds {
    /// Where each gauge ended, in the order of [`GaugeIndex::index`], or `None` for one with no
    /// history.
    pub ends: [Option<f32>; GaugeIndex::COUNT],
}

impl GaugeEnds {
    /// The ends of a run's gauge histories.
    pub fn of_log(log: &GaugeLog) -> GaugeEnds {
        GaugeEnds { ends: GaugeIndex::ALL.map(|index| log.last(index)) }
    }

    /// Where the gauge numbered `gauge_type` ended, or `None` for one with no history or no such
    /// gauge.
    pub fn value(&self, gauge_type: usize) -> Option<f32> {
        self.ends.get(gauge_type).copied().flatten()
    }
}

/// How the run's notes were timed, as the timing digits and the average duration read them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimingFigures {
    /// The mean timing of the judged notes in milliseconds, positive when early, or the
    /// reference's `Float.MAX_VALUE` when none landed in range.
    pub average_ms: f32,
    /// The standard deviation of the same, or `-1` when none landed in range.
    pub std_dev_ms: f32,
    /// How far either side of a note the distribution reaches, in milliseconds
    /// (`TimingDistribution.getArrayCenter`).
    pub center_ms: u32,
    /// `ScoreData.avgjudge`: the mean distance of each note from its input, in microseconds.
    pub avg_duration_us: i64,
}

impl Default for TimingFigures {
    fn default() -> TimingFigures {
        TimingFigures { average_ms: 0.0, std_dev_ms: 0.0, center_ms: TIMING_DISTRIBUTION_RANGE_MS as u32, avg_duration_us: 0 }
    }
}

impl TimingFigures {
    /// The figures of a finished run.
    pub fn of_record(record: &PlayRecord) -> TimingFigures {
        TimingFigures {
            average_ms: record.timing.average(),
            std_dev_ms: record.timing.std_dev(),
            center_ms: record.timing.center() as u32,
            avg_duration_us: record.avg_duration_us,
        }
    }
}

/// Everything [`ResultSnapshot::of`] makes a snapshot from.
///
/// [`ResultInput::from_record`] fills in what the run itself says and leaves the rest as a run
/// with no history, no target and no extras would have it; the caller sets the fields it knows by
/// struct update.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultInput {
    pub sheet: ScoreSheet,
    pub gauge: GaugeEnds,
    pub timing: TimingFigures,
    pub previous: PreviousScore,
    /// What the run was paced against, when it was paced against anything.
    pub target: Option<TargetView>,
    pub options: PlayedOptions,
    /// Whether the song and the chart are starred, when that is known.
    pub favorite_song: Option<bool>,
    pub favorite_chart: Option<bool>,
    /// The titles of the songs of the course the run belongs to, in order. Empty for a single
    /// chart.
    pub course_titles: Vec<String>,
    /// The lamp of the course so far, for a run that belongs to one (`resource.getCourseScoreData`).
    pub course_clear: Option<u8>,
    /// Whether the run counts towards the stored bests (`PlayerResource.isUpdateScore`).
    pub updates_score: bool,
}

impl Default for ResultInput {
    /// A run of nothing: no notes, no history, no target, and one that counts towards the bests.
    fn default() -> ResultInput {
        ResultInput {
            sheet: ScoreSheet::default(),
            gauge: GaugeEnds::default(),
            timing: TimingFigures::default(),
            previous: PreviousScore::default(),
            target: None,
            options: PlayedOptions::default(),
            favorite_song: None,
            favorite_chart: None,
            course_titles: Vec::new(),
            course_clear: None,
            updates_score: true,
        }
    }
}

impl ResultInput {
    /// The input of a finished run played in `mode`, reported under the lamp `clear`.
    pub fn from_record(record: &PlayRecord, mode: Mode, clear: ClearType) -> ResultInput {
        ResultInput {
            sheet: ScoreSheet::of_record(record, PointFamily::of_mode(mode), clear),
            gauge: GaugeEnds::of_log(&record.gauge_log),
            timing: TimingFigures::of_record(record),
            ..ResultInput::default()
        }
    }
}

/// A finished run, as the result screen of a skin reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultSnapshot {
    /// The run's score, held against the best and the target (`ScoreDataProperty`).
    pub score: RunScore,
    pub previous: PreviousScore,
    pub gauge: GaugeEnds,
    pub timing: TimingFigures,
    pub options: PlayedOptions,
    /// Whether the song and the chart are starred, which the screen writes as the player stars one.
    pub favorite_song: Option<bool>,
    pub favorite_chart: Option<bool>,
    /// The name of the target the run was paced against, empty when there was none.
    pub target_name: String,
    pub course_titles: Vec<String>,
    pub course_clear: Option<u8>,
    pub updates_score: bool,
}

impl ResultSnapshot {
    /// The snapshot of one finished run: `setTargetScore(best, target)` and `update(score)`
    /// (`MusicResult.updateScoreDatabase`).
    pub fn of(input: ResultInput) -> ResultSnapshot {
        let pace = TargetPace {
            best_score: input.previous.ex_score,
            rival_score: input.target.as_ref().map_or(0, |target| target.ex),
            total_notes: input.sheet.notes,
        };
        ResultSnapshot {
            score: RunScore::finished(input.sheet, pace),
            previous: input.previous,
            gauge: input.gauge,
            timing: input.timing,
            options: input.options,
            favorite_song: input.favorite_song,
            favorite_chart: input.favorite_chart,
            target_name: input.target.map(|target| target.name).unwrap_or_default(),
            course_titles: input.course_titles,
            course_clear: input.course_clear,
            updates_score: input.updates_score,
        }
    }

    /// Whether the run counts as a clear: neither the run nor the course it belongs to has failed
    /// (`BooleanPropertyFactory.result_clear`).
    pub fn cleared(&self) -> bool {
        let failed = rbms_judge::clear_type_id(ClearType::Failed);
        self.score.sheet.clear != failed && self.course_clear != Some(failed)
    }

    /// Where the gauge numbered `gauge_type` ended, as the gauge numbers read it.
    pub fn gauge_reading(&self, gauge_type: usize) -> Option<GaugeReading> {
        self.gauge.value(gauge_type).map(|value| GaugeReading { value, max: GaugeReading::FULL, kind: gauge_type as i32, live: false })
    }
}

/// A finished run on the screen that is up: what it left, and what the screen holds beside it.
///
/// This is what the clusters that report a result borrow for a frame. The snapshot never changes
/// while the screen is up; the scene is the screen's own state, which the player's keys change.
#[derive(Debug, Clone, Copy)]
pub struct FinishedRun<'a> {
    pub snapshot: &'a ResultSnapshot,
    pub scene: ResultScene,
}

impl<'a> FinishedRun<'a> {
    pub fn new(snapshot: &'a ResultSnapshot, scene: ResultScene) -> FinishedRun<'a> {
        FinishedRun { snapshot, scene }
    }

    /// Where the gauge the graph is on ended, as the gauge numbers read it.
    pub fn gauge_reading(&self) -> Option<GaugeReading> {
        self.snapshot.gauge_reading(self.scene.gauge_type)
    }
}

#[cfg(test)]
mod tests;
