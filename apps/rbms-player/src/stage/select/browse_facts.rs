//! What the browser works out for its skin beyond the bar under the cursor: the other charts of the
//! song the cursor is on, how the charts of the open folder are cleared, and the player's own plays
//! of the chart ([`BrowseFacts`]).
//!
//! A skin asks about all of it on every frame and none of it changes until the cursor, the list or
//! the records do. So each is worked out when that happens and kept ([`SelectState::refresh_browse`]),
//! and the frame is lent what was kept ([`crate::skin_host::select::BrowserLent::browse`]). None of
//! it is worked out for a browser no skin draws.
//!
//! # The other charts of a song
//!
//! The library has no song to group charts by, so the charts of a song are the ones of the list on
//! show that share the cursor chart's folder, title and play mode. A chart is in the slot of its
//! `#DIFFICULTY`, 1 to 5; the chart under the cursor has its slot to itself, and where two other
//! charts claim a slot the first of the list has it. A chart the list does not hold is not in a
//! slot, because the event that moves the cursor to a slot ([`SelectState::move_to_sibling_chart`])
//! could not take it there. A folder that holds several songs under one title can put charts of
//! another song in a slot.
//!
//! # The open folder
//!
//! The charts of the open folder are the chart bars of the list on show whose file is on disk,
//! counted by the lamp the player holds on each. A chart a table lists and the library lacks is not
//! counted, nor is a folder, a course or any other bar.
//!
//! # The player's own plays
//!
//! The first row is the best of the chart as the score database holds it, the ones after it the
//! latest plays, newest first. The best has the date of the first play that scored it, and no row
//! at all while no play that may set a best has been made. The latest plays are read from the score
//! database's log of plays.

use rbms_library::SongEntry;
use rbms_render::skin_render::frame::{BarKind, LAMP_KINDS};
use rbms_store::scoredb::{BestScore, PlayLog, UNSET_MINIMUM, charged_minbp, chart_key, mode_id};
use rbms_store::sets_best;

use super::SelectState;
use super::ranking_scroll::RankingScroll;
use crate::skin_host::chart::ChartMeta;
use crate::skin_host::score::standing::{PointFamily, ScoreSheet};
use crate::skin_host::select::BarRecord;
use crate::skin_host::select::browse::{BrowseFacts, RECORD_ROWS, RecordRow, SIBLING_SLOTS, SiblingChart};
use crate::{AppShared, SKIN_TYPE_MUSIC_SELECT, SelectBar, SelectItem, SelectTab};

/// How many plays of a chart are read from the log to find the best and the latest of them. A chart
/// played more often than this has the date of its best looked for among the plays that many back
/// only.
const PLAY_LOG_READ_LIMIT: usize = 1000;

/// The row of the player's own plays that holds the best.
const BEST_ROW: usize = 0;

/// What the other charts of the song under the cursor were worked out for: the list's generation,
/// the row of the cursor and the tab on show.
type SiblingsKey = (u64, usize, SelectTab);

/// What the lamps of the open folder were counted for: the list's generation, how many records the
/// score book held, the tab on show, how often the course list and the chart facts have been read,
/// and how many bars the list had.
type LampsKey = (u64, usize, SelectTab, u64, u64, usize);

/// What the player's own plays were read for: the list's generation, the chart's place in the
/// library (none for a bar that is no chart) and how many records the score book held.
type RecordsKey = (u64, Option<usize>, usize);

/// What the browser keeps for its skin beside the list itself, and what it was kept for.
#[derive(Debug, Default)]
pub(super) struct BrowseState {
    pub(super) facts: BrowseFacts,
    siblings_for: Option<SiblingsKey>,
    lamps_for: Option<LampsKey>,
    records_for: Option<RecordsKey>,
    /// How far down the ranking of the chart under the cursor the skin's list of scores is scrolled.
    pub(super) ranking: RankingScroll,
}

/// The rows of `items` that hold the charts of the song of the chart at `row`, by difficulty slot.
/// Nothing is held where `row` is not a chart.
pub(super) fn sibling_rows(items: &[SelectItem], songs: &[SongEntry], row: usize) -> [Option<usize>; SIBLING_SLOTS] {
    let mut slots = [None; SIBLING_SLOTS];
    let Some(SelectItem::Song(focus)) = items.get(row) else {
        return slots;
    };
    let Some(focus) = songs.get(*focus) else {
        return slots;
    };
    let slot_of = |entry: &SongEntry| usize::try_from(entry.difficulty.checked_sub(1)?).ok().filter(|slot| *slot < SIBLING_SLOTS);
    for (at, item) in items.iter().enumerate() {
        let SelectItem::Song(index) = item else {
            continue;
        };
        let Some(entry) = songs.get(*index) else {
            continue;
        };
        let Some(slot) = slot_of(entry) else {
            continue;
        };
        if entry.mode == focus.mode && entry.title == focus.title && entry.path.parent() == focus.path.parent() {
            slots[slot] = slots[slot].or(Some(at));
        }
    }
    if let Some(slot) = slot_of(focus) {
        slots[slot] = Some(row);
    }
    slots
}

/// How many of the chart bars on disk among `bars` sit on each lamp, no play first.
pub(super) fn lamp_counts(bars: &[SelectBar]) -> [u32; LAMP_KINDS] {
    let mut counts = [0; LAMP_KINDS];
    for bar in bars.iter().filter(|bar| matches!(bar.kind, BarKind::Song { exists: true })) {
        if let Some(count) = counts.get_mut(usize::from(bar.lamp.unwrap_or_default())) {
            *count += 1;
        }
    }
    counts
}

/// The figures of a play the way a skin reads the figures of a score.
fn sheet_of_play(play: &PlayLog) -> ScoreSheet {
    ScoreSheet {
        early: play.judge.early,
        late: play.judge.late,
        notes: play.notes,
        max_combo: play.combo,
        min_bp: u32::try_from(charged_minbp(play)).unwrap_or_default(),
        clear: play.clear,
        family: PointFamily::default(),
    }
}

/// The rows of the player's own plays of a chart: the best as the score database holds it, then the
/// latest of `plays`, which are newest first.
///
/// A chart holds no best until a play that may set one was made: the bad-poor count of a best that
/// was never set stays at its unset value.
pub(super) fn record_rows(plays: &[PlayLog], best: Option<&BestScore>) -> [Option<RecordRow>; RECORD_ROWS] {
    let best_row = best.filter(|best| best.minbp != UNSET_MINIMUM).map(|best| {
        let scored = best.ex_score();
        let first_to_score = plays.iter().rev().find(|play| sets_best(play.assist, play.rule_version) && play.judge.ex_score() == scored);
        RecordRow::of(&BarRecord::of_best(best, PointFamily::default()).sheet, first_to_score.map(|play| play.date))
    });
    let mut latest = plays.iter().take(RECORD_ROWS - 1).map(|play| RecordRow::of(&sheet_of_play(play), Some(play.date)));
    std::array::from_fn(|row| if row == BEST_ROW { best_row.clone() } else { latest.next() })
}

impl SelectState {
    /// Bring what the skin is lent beside the list up to date with the cursor, the list and the
    /// records, for a browser a skin draws. It is asked once the list's bars are up to date, which is
    /// when a frame is drawn.
    pub(super) fn refresh_browse(&mut self, shared: &AppShared) {
        if !shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT) {
            return;
        }
        self.refresh_siblings(shared);
        self.refresh_folder_lamps(shared);
        self.refresh_own_records(shared);
        self.browse.ranking.settle(self.ranked_chart(shared));
    }

    /// The md5 of the chart whose ranking the skin's list shows, when the cursor is on a chart.
    pub(super) fn ranked_chart<'a>(&self, shared: &'a AppShared) -> Option<&'a str> {
        let index = self.chart_index_under_cursor(shared)?;
        Some(shared.library.songs().get(index)?.md5.as_str())
    }

    fn refresh_siblings(&mut self, shared: &AppShared) {
        let key = (shared.select_gen, shared.sel, self.tab);
        if self.browse.siblings_for == Some(key) {
            return;
        }
        self.browse.siblings_for = Some(key);
        let rows = match self.tab {
            SelectTab::Songs => sibling_rows(&shared.select_items, shared.library.songs(), shared.sel),
            SelectTab::Courses => [None; SIBLING_SLOTS],
        };
        self.browse.facts.siblings = rows.map(|row| {
            let SelectItem::Song(index) = shared.select_items.get(row?)? else {
                return None;
            };
            Some(SiblingChart { level: ChartMeta::of_entry(shared.library.songs().get(*index)?).level })
        });
    }

    fn refresh_folder_lamps(&mut self, shared: &AppShared) {
        let bars = self.bars.bars();
        let key = (shared.select_gen, shared.scores.records().len(), self.tab, self.courses_generation, self.facts_generation, bars.len());
        if self.browse.lamps_for == Some(key) {
            return;
        }
        self.browse.lamps_for = Some(key);
        self.browse.facts.folder_lamps = lamp_counts(bars);
    }

    fn refresh_own_records(&mut self, shared: &AppShared) {
        let index = self.chart_index_under_cursor(shared);
        let key = (shared.select_gen, index, shared.scores.records().len());
        if self.browse.records_for == Some(key) {
            return;
        }
        self.browse.records_for = Some(key);
        let entry = index.and_then(|index| shared.library.songs().get(index));
        self.browse.facts.records = match (entry, shared.scoredb.as_ref()) {
            (Some(entry), Some(database)) => {
                let mode = mode_id(entry.mode.name);
                let log = database.history(&chart_key(&entry.md5, ""), PLAY_LOG_READ_LIMIT).unwrap_or_default();
                let plays: Vec<PlayLog> = log.into_iter().filter(|play| play.mode == mode).collect();
                record_rows(&plays, self.focused_records.best())
            }
            _ => Default::default(),
        };
    }

    /// `siblingchart`: move the cursor to the chart of the song under it in difficulty slot `slot`
    /// (0 for BEGINNER), with the sound of a move. A slot with no chart in the list, and a chart
    /// that is the one under the cursor already, do nothing.
    pub(super) fn move_to_sibling_chart(&mut self, shared: &mut AppShared, slot: usize) {
        if self.tab != SelectTab::Songs {
            return;
        }
        let Some(row) = sibling_rows(&shared.select_items, shared.library.songs(), shared.sel).get(slot).copied().flatten() else {
            return;
        };
        if row == shared.sel {
            return;
        }
        shared.sel = row;
        self.record_modal = None;
        shared.print_selection();
    }
}

#[cfg(test)]
mod tests;
