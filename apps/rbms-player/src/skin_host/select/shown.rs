//! What the song browser hands the property host: one frame's view of the list, the bar under the
//! cursor and the settings and ranking that bar is read against.
//!
//! [`SelectShown`] is plain data. It is made by [`SelectShown::of`] from the bars the wheel draws and
//! what the application keeps about the chart under the cursor, and reads nothing that is not
//! passed to it, so what a skin reads off the browser can be built and checked without a window, a
//! library or a database. It is rebuilt every frame; the cluster
//! ([`SelectState`](super::SelectState)) borrows it. `bridge.rs` is the one place that gathers the
//! inputs from the application.
//!
//! The reference reads the same things from several places: the bar manager for the bar and its
//! place, `Bar.getScore` for the best score on it, `ScoreDataProperty` for what is made of that
//! score, `PlayerConfig` and `PlayConfig` for the settings and `RankingData` for the ranking. Here
//! they are one value each, and the score of the bar under the cursor is the one thing the
//! reference derives other numbers from.

use rbms_config::{Config, SortMode};
use rbms_course::CourseConstraint;
use rbms_model::Mode;
use rbms_render::skin_render::frame::{BarDistribution, BarKind, SongBar};
use rbms_skin::lua::{LocalTime, local_time};
use rbms_store::scoredb::BestScore;

use crate::skin_host::ir::IrBrowser;
use crate::skin_host::options::SettingsView;
use crate::skin_host::score::standing::{PointFamily, ScoreSheet, ScoreStanding, TargetPace};

/// How many replays a chart can keep, which the reference's replay options number
/// (`MusicSelector.REPLAY`). This player keeps every replay a run saves by itself, and the slots
/// are the newest of them: the first slot the newest, as the browser plays them
/// (`stage/select/events.rs`).
pub const REPLAY_SLOTS: usize = 4;

/// The largest value a count of the reference's `int` holds, which is where the unset bad-poor
/// count of a score is kept.
const COUNT_CEILING: i64 = i32::MAX as i64;

/// What the reference writes in front of a stage's title when the chart is not in the library
/// (`StringType.createCoursetitle`).
const MISSING_STAGE_PREFIX: &str = "(no song) ";

/// What a bar is separated from the next by in the path of the open folders
/// (`BarManager.updateBar`).
const DIRECTORY_SEPARATOR: &str = " > ";

/// The best score on a chart as `Bar.getScore` holds it (`ScoreData`).
#[derive(Debug, Clone, PartialEq)]
pub struct BarRecord {
    /// The judgements, the combo, the bad-poor count and the lamp.
    pub sheet: ScoreSheet,
    /// How many times the chart was played and how many of those were cleared.
    pub play_count: u32,
    pub clear_count: u32,
    /// The moment of the last play, in seconds, and the clock it falls on here.
    pub last_played: i64,
    pub last_played_clock: Option<LocalTime>,
}

impl BarRecord {
    /// The record the score database keeps for a chart, whose kind of play decides how its points
    /// are weighed.
    pub fn of_best(best: &BestScore, family: PointFamily) -> BarRecord {
        BarRecord {
            sheet: ScoreSheet {
                early: best.judge.early,
                late: best.judge.late,
                notes: best.notes,
                max_combo: best.combo,
                min_bp: u32::try_from(best.minbp.clamp(0, COUNT_CEILING)).unwrap_or_default(),
                clear: best.clear,
                family,
            },
            play_count: best.playcount,
            clear_count: best.clearcount,
            last_played: best.date,
            last_played_clock: Some(best.date).filter(|date| *date > 0).and_then(local_time),
        }
    }
}

/// What a course is made of, as the reference's course bar holds it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CourseShown {
    /// The restrictions the course plays under.
    pub constraints: Vec<CourseConstraint>,
    /// The title of each stage as the reference writes it on the browser.
    pub stages: Vec<String>,
}

impl CourseShown {
    /// A course from its constraints and its stages, each a title and whether the library holds
    /// the chart.
    pub fn new<'t>(constraints: &[CourseConstraint], stages: impl IntoIterator<Item = (&'t str, bool)>) -> CourseShown {
        CourseShown { constraints: constraints.to_vec(), stages: stages.into_iter().map(|(title, in_library)| stage_title(title, in_library)).collect() }
    }
}

/// A stage's title the way `StringType.createCoursetitle` writes it: marked when the chart is not
/// in the library.
pub fn stage_title(title: &str, in_library: bool) -> String {
    if in_library { title.to_string() } else { format!("{MISSING_STAGE_PREFIX}{title}") }
}

/// The bar under the cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct BarShown {
    /// Which of the reference's bar classes it stands for.
    pub kind: BarKind,
    /// What the bar is called.
    pub title: String,
    /// The best score on it (`Bar.getScore`). A folder, a table and a course have none here.
    pub record: Option<BarRecord>,
    /// How the charts under a bar that opens are cleared, for the bars that carry it.
    pub distribution: Option<BarDistribution>,
    /// Which replay slots hold a replay (`SelectableBar.existsReplay`), or `None` for a bar that
    /// cannot have any.
    pub replays: Option<[bool; REPLAY_SLOTS]>,
    /// Whether the chart is starred, for a bar that is a chart.
    pub favorite: Option<bool>,
    /// The course, for a bar that is one.
    pub course: Option<CourseShown>,
}

/// One frame of the browser as the property host reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectShown {
    /// The bar under the cursor, or `None` when the list is empty.
    pub bar: Option<BarShown>,
    /// Where the cursor stands in the list and how long the list is (`BarManager.getSelectedPosition`).
    pub index: usize,
    pub total: usize,
    /// Which panel is open, 1 to 3, or 0 for none. `None` while the browser does not say, in which
    /// case the panel options are left to whatever else can answer them.
    pub panel: Option<u8>,
    /// The mode the list is held to, or `None` for every mode.
    pub mode_filter: Option<Mode>,
    /// How the list is ordered.
    pub sort: SortMode,
    /// The titles of the folders that are open, outermost first.
    pub directory: Vec<String>,
    /// The replay slot a replay would be played from, or `None` when the bar under the cursor has
    /// no replay to play (`MusicSelector.getSelectedReplay`, which is -1 then).
    pub selected_replay: Option<usize>,
    /// What the score on the bar makes of itself (`ScoreDataProperty`), which reads as it does for
    /// a bar with no score while the bar has none.
    pub standing: ScoreStanding,
    /// The settings the bar is read against.
    pub settings: SettingsView,
    /// The ranking of the chart under the cursor.
    pub ir: IrBrowser,
}

/// What the library and the player's records say about the chart under the cursor.
pub struct ChartInputs<'a> {
    /// The kind of play the chart is, which decides how its points are weighed.
    pub mode: Mode,
    /// The best score the player holds on it.
    pub best: Option<&'a BestScore>,
    /// Whether the player has starred it.
    pub favorite: bool,
    /// How many replays are saved of it.
    pub replays_stored: usize,
}

/// Everything [`SelectShown::of`] reads.
pub struct SelectInputs<'a> {
    /// The whole list as the wheel draws it, and the place of the cursor in it.
    pub bars: &'a [SongBar],
    pub cursor: usize,
    /// The chart under the cursor, when the bar there is a chart.
    pub chart: Option<ChartInputs<'a>>,
    /// The course under the cursor, when the bar there is one.
    pub course: Option<CourseShown>,
    pub directory: Vec<String>,
    pub mode_filter: Option<Mode>,
    pub sort: SortMode,
    pub panel: Option<u8>,
    pub selected_replay: Option<usize>,
    pub config: &'a Config,
    /// The ranking of the chart under the cursor, as the panel holds it.
    pub ir: IrBrowser,
}

impl SelectShown {
    /// The browser's frame as the property host reads it.
    pub fn of(inputs: SelectInputs<'_>) -> SelectShown {
        let selected = inputs.bars.get(inputs.cursor);
        let kind = selected.map(|bar| bar.kind);
        let selectable = matches!(kind, Some(BarKind::Song { .. } | BarKind::Course { .. } | BarKind::RandomCourse { .. } | BarKind::Executable));
        let is_song = matches!(kind, Some(BarKind::Song { .. }));
        let chart = inputs.chart.filter(|_| is_song);
        let record = chart.as_ref().and_then(|chart| chart.best.map(|best| BarRecord::of_best(best, PointFamily::of_mode(chart.mode))));
        let sheet = record.as_ref().map(|record| record.sheet).unwrap_or_default();
        let has_ranking = matches!(kind, Some(BarKind::Song { exists: true } | BarKind::Course { complete: true }));
        let has_chart_settings = matches!(kind, Some(BarKind::Song { .. } | BarKind::Course { complete: true }));
        let settings = SettingsView::of_config(inputs.config);
        let bar = selected.map(|bar| BarShown {
            kind: bar.kind,
            title: bar.title.clone(),
            record,
            distribution: bar.distribution.as_deref().copied(),
            replays: selectable.then(|| std::array::from_fn(|slot| chart.as_ref().is_some_and(|chart| slot < chart.replays_stored))),
            favorite: chart.as_ref().map(|chart| chart.favorite),
            course: inputs.course.filter(|_| matches!(bar.kind, BarKind::Course { .. })),
        });
        SelectShown {
            bar,
            index: inputs.cursor,
            total: inputs.bars.len(),
            panel: inputs.panel,
            mode_filter: inputs.mode_filter,
            sort: inputs.sort,
            directory: inputs.directory,
            selected_replay: inputs.selected_replay,
            standing: ScoreStanding::of(&sheet, sheet.notes, TargetPace { total_notes: sheet.notes, ..TargetPace::default() }),
            settings: if has_chart_settings { settings } else { settings.without_chart() },
            ir: IrBrowser { board: inputs.ir.board.filter(|_| has_ranking), ..inputs.ir },
        }
    }

    /// The path of the open folders the way the reference writes it: every title followed by a
    /// separator.
    pub fn directory_path(&self) -> String {
        self.directory.iter().map(|title| format!("{title}{DIRECTORY_SEPARATOR}")).collect()
    }
}
