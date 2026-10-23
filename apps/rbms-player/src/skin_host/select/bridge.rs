//! The song browser's frame, gathered from what the application keeps and what the browser lends.
//!
//! [`SelectShown::of`] is a pure function of what it is handed; this is the one place that hands it
//! the application's own state: the list the wheel draws, the settings, and the ranking the panel
//! has fetched.
//!
//! What only the browser itself knows it lends for the frame ([`BrowserLent`]): which panel is open,
//! the mode the list is held to, the course under the cursor, the replay slot selected, how far the
//! ranking list is scrolled, and what the player's records hold of the chart under the cursor. The
//! last is read from the score database, which is not something to do on every frame, so the browser
//! reads it when the cursor comes to rest on a chart and keeps it until the cursor or the records
//! move. The other charts of the song, the open folder's lamps and the player's latest plays of the
//! chart are kept the same way ([`super::browse::BrowseFacts`]).

use rbms_config::ScoreTarget;
use rbms_model::Mode;
use rbms_render::SongBars;
use rbms_render::skin_render::frame::BarKind;
use rbms_store::scoredb::{BestScore, chart_key, mode_id};

use super::browse::BrowseFacts;
use super::shown::{ChartInputs, CourseShown, SelectInputs, SelectShown, unreachable_keys_of};
use crate::AppShared;
use crate::app_result::{local_best_before_reading, target_context};
use crate::skin_host::ir::IrBrowser;
use crate::target;

/// What the reference writes round the word of the folder it files a search under
/// (`SearchWordBar`: `"Search : '" + text + "'"`).
const SEARCH_TITLE_OPENS: &str = "Search : '";
const SEARCH_TITLE_CLOSES: &str = "'";

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
    /// Whether the ranking mode is open over the browser.
    pub(crate) ranking_mode: bool,
    /// How many scores down the ranking of the chart under the cursor the list a skin draws is
    /// scrolled (`MusicSelector.getRankingOffset`).
    pub(crate) ranking_offset: usize,
    /// The other charts of the song under the cursor, the open folder's lamps and the player's own
    /// plays of the chart, when the browser has worked them out.
    pub(crate) browse: Option<&'a BrowseFacts>,
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
        let stand_in = entry.and_then(|entry| self.target_stand_in_on(&entry.md5));
        let shown = SelectShown::of(SelectInputs {
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
            ir: IrBrowser::of_ranking(self.has_primary_ir_server(), entry.and_then(|entry| self.ranking_cache.peek(&entry.md5)), lent.ranking_offset),
        });
        let shown = SelectShown { ranking_mode: lent.ranking_mode, browse: lent.browse.cloned().unwrap_or_default(), ..shown };
        SelectShown { settings: shown.settings.with_target_stand_in(stand_in), unreachable_keys: unreachable_keys_of(self.mode), ..shown }
    }

    /// How many scores the ranking list a skin draws is scaled by for the chart under the cursor: the
    /// scores received so far, and one while there are none (`MusicSelector.getRankingPosition`).
    /// The slider that scrolls the list reads and writes its position over this count.
    pub(crate) fn ranking_list_extent(&self) -> usize {
        let ranking = self.focused_song_index().and_then(|index| self.library.songs().get(index)).and_then(|entry| self.ranking_cache.peek(&entry.md5));
        let received = IrBrowser::of_ranking(self.has_primary_ir_server(), ranking, 0).board.map_or(0, |board| board.total);
        usize::try_from(received).unwrap_or_default().max(1)
    }

    /// The target a run of the chart `md5` would be paced against in place of the one the TARGET
    /// row names, or `None` when the chart holds the score that target names (or the target needs
    /// none).
    ///
    /// The local record is the one the run itself would count ([`local_best_before_reading`]), so
    /// the browser never says a chart holds a record the play screen then passes over.
    fn target_stand_in_on(&self, md5: &str) -> Option<ScoreTarget> {
        let context = target_context(self, md5, 0, local_best_before_reading(self, md5));
        let asked = self.config.judge.target;
        let (used, _) = target::settle(asked, self.config.judge.target_fallback, &context);
        (used != asked).then_some(used)
    }

    /// The titles of the folders the browser has opened, outermost first (`BarManager.dir`).
    ///
    /// A search is a folder of its own to the reference, titled with the word (`SearchWordBar`), and
    /// a list a search narrows is that folder: its title is what a skin that draws this path has to
    /// say the list on show is what a word found, and which word.
    fn open_folders(&self) -> Vec<String> {
        let word = self.search.trim();
        if self.is_in_the_flat_list() && !word.is_empty() {
            return vec![format!("{SEARCH_TITLE_OPENS}{word}{SEARCH_TITLE_CLOSES}")];
        }
        self.open_titles()
    }
}
