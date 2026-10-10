//! The song browser's frame, gathered from what the application keeps and what the browser lends.
//!
//! [`SelectShown::of`] is a pure function of what it is handed; this is the one place that hands it
//! the application's own state: the list the wheel draws, the settings, and the ranking the panel
//! has fetched.
//!
//! What only the browser itself knows it lends for the frame ([`BrowserLent`]): which panel is open,
//! the mode the list is held to, the course under the cursor, the replay slot selected, and what the
//! player's records hold of the chart under the cursor. The last is read from the score database,
//! which is not something to do on every frame, so the browser reads it when the cursor comes to
//! rest on a chart and keeps it until the cursor or the records move.

use rbms_model::Mode;
use rbms_render::SongBars;
use rbms_render::skin_render::frame::BarKind;
use rbms_store::scoredb::{BestScore, chart_key, mode_id};

use super::shown::{ChartInputs, CourseShown, SelectInputs, SelectShown};
use crate::skin_host::ir::IrBrowser;
use crate::{AppShared, SelectView};

/// What the reference writes for the folder that holds every chart, and for a level of a table.
const ALL_SONGS_TITLE: &str = "ALL SONGS";
const TABLE_LEVEL_PREFIX: &str = "LV";

/// What the browser lends one frame of its skin beside the list itself.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BrowserLent<'a> {
    /// The title of the list when it is the course list, whose folders are not the library's.
    pub(crate) courses: &'a str,
    /// Which option panel is up, 1 to 3, or 0 for none.
    pub(crate) panel: u8,
    /// The play mode the list is held to, or `None` for every mode.
    pub(crate) mode_filter: Option<Mode>,
    /// The course under the cursor, when the bar there is one.
    pub(crate) course: Option<&'a CourseShown>,
    /// The replay slot a replay would be played from, or `None` when there is none to play.
    pub(crate) selected_replay: Option<usize>,
    /// The best score the player holds on the chart under the cursor, and how many replays are
    /// saved of it. Both are nothing for a bar that is no chart.
    pub(crate) best: Option<&'a BestScore>,
    pub(crate) replays_stored: usize,
}

impl AppShared {
    /// The best score the score database holds on the chart at `index` of the library, and how many
    /// replays are saved of it: what [`BrowserLent::best`] and [`BrowserLent::replays_stored`] are
    /// read from. One query of the database and one walk of the chart's records, so the browser
    /// asks when the chart under the cursor changes rather than on every frame.
    pub(crate) fn chart_records_shown(&self, index: usize) -> (Option<BestScore>, usize) {
        let Some(entry) = self.library.songs().get(index) else {
            return (None, 0);
        };
        let best = self.scoredb.as_ref().and_then(|db| db.best(&chart_key(&entry.md5, ""), mode_id(entry.mode.name)).ok().flatten());
        let replays_stored = self.scores.for_md5(&entry.md5).iter().filter(|past| past.replay_file.is_some()).count();
        (best, replays_stored)
    }

    /// The browser's frame as the property host reads it, for the list `wheel` draws and what the
    /// browser lends beside it.
    pub(crate) fn select_shown(&self, wheel: Option<&SongBars<'_>>, lent: &BrowserLent<'_>) -> SelectShown {
        let bars = wheel.map_or(&[][..], |wheel| wheel.bars);
        let cursor = wheel.map_or(0, |wheel| wheel.selected);
        let kind = bars.get(cursor).map(|bar| bar.kind);
        let entry = self.focused_song_index().and_then(|index| self.library.songs().get(index)).filter(|_| matches!(kind, Some(BarKind::Song { .. })));
        let chart = entry.map(|entry| ChartInputs {
            mode: entry.mode,
            best: lent.best,
            favorite: self.favorites.contains(&entry.md5),
            replays_stored: lent.replays_stored,
        });
        let directory =
            if matches!(kind, Some(BarKind::Course { .. } | BarKind::RandomCourse { .. })) { vec![lent.courses.to_string()] } else { self.open_folders() };
        SelectShown::of(SelectInputs {
            bars,
            cursor,
            chart,
            course: lent.course.cloned(),
            directory,
            mode_filter: lent.mode_filter,
            sort: self.config.library.sort,
            panel: Some(lent.panel),
            selected_replay: lent.selected_replay,
            config: &self.config,
            ir: IrBrowser::of_ranking(self.has_primary_ir_server(), entry.and_then(|entry| self.ranking_cache.peek(&entry.md5))),
        })
    }

    /// The titles of the folders the browser has opened, outermost first (`BarManager.dir`).
    fn open_folders(&self) -> Vec<String> {
        let table = |index: usize| self.table_names.get(index).cloned().unwrap_or_default();
        match self.select_view {
            SelectView::Root => Vec::new(),
            SelectView::AllSongs => vec![ALL_SONGS_TITLE.to_string()],
            SelectView::TableLevels(index) => vec![table(index)],
            SelectView::TableLevel(index, level) => {
                let level = self
                    .table_levels
                    .get(index)
                    .and_then(|levels| levels.get(level))
                    .map(|(level, _)| format!("{TABLE_LEVEL_PREFIX} {level}"))
                    .unwrap_or_default();
                vec![table(index), level]
            }
        }
    }
}
