//! What the browser keeps for its skin beside the list: the records of the chart under the cursor,
//! the replay slot a replay would be played from, and the course under the cursor.
//!
//! A skin's frame asks about all three on every frame, and none of them changes until the cursor or
//! what it is read from does. So each is read when that happens and kept
//! ([`SelectState::refresh_lent`]), and the frame is lent what was kept
//! ([`crate::skin_host::select::BrowserLent`]).
//!
//! The replay slot is the reference's (`MusicSelector.selectedreplay`): whenever the bar under the
//! cursor changes it goes back to the lowest slot that holds a replay, or to none
//! (`MusicSelectCommand.RESET_REPLAY`), and the key for the next replay moves it round the slots
//! that hold one (`NEXT_REPLAY`). The slots are the replays saved of the chart, the newest first.
//!
//! None of this is read for a browser no skin draws.

use rbms_store::scoredb::BestScore;

use super::SelectState;
use super::skinned::FocusedBar;
use crate::course_ui::library_index;
use crate::skin_host::select::{CourseShown, REPLAY_SLOTS};
use crate::{AppShared, SKIN_TYPE_MUSIC_SELECT, SelectTab, SystemSound};

/// What [`FocusedRecords`] was read for: the list's generation, the chart's place in the library
/// (none for a bar that is no chart) and how many records the score book held.
type RecordsKey = (u64, Option<usize>, usize);

/// What the player's records hold of the chart under the cursor.
#[derive(Debug, Default)]
pub(super) struct FocusedRecords {
    key: Option<RecordsKey>,
    best: Option<BestScore>,
    replays_stored: usize,
}

impl FocusedRecords {
    /// The best score the player holds on the chart.
    pub(super) fn best(&self) -> Option<&BestScore> {
        self.best.as_ref()
    }

    /// How many replays are saved of it.
    pub(super) fn replays_stored(&self) -> usize {
        self.replays_stored
    }
}

impl SelectState {
    /// The place in the library of the chart under the cursor, when the bar there is a chart.
    pub(super) fn chart_index_under_cursor(&self, shared: &AppShared) -> Option<usize> {
        if self.tab != SelectTab::Songs {
            return None;
        }
        shared.focused_song_index()
    }

    /// Bring what the skin is lent up to date with the cursor, for a browser a skin draws.
    pub(super) fn refresh_lent(&mut self, shared: &AppShared) {
        if !shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT) {
            return;
        }
        self.refresh_focused_records(shared);
        self.refresh_course_shown(shared);
    }

    /// Read the records of the chart under the cursor when the cursor has come to another bar, the
    /// list was built again or a run was recorded, and put the replay slot back on the lowest one
    /// that holds a replay: all of which the reference does when the bar under the cursor moves
    /// (`BarManager.updateBar`, `MusicSelector.selectedBarMoved`).
    fn refresh_focused_records(&mut self, shared: &AppShared) {
        let index = self.chart_index_under_cursor(shared);
        let key = (shared.select_gen, index, shared.scores.records().len());
        if self.focused_records.key == Some(key) {
            return;
        }
        let (best, replays_stored) = index.map_or((None, 0), |index| shared.chart_records_shown(index));
        self.focused_records = FocusedRecords { key: Some(key), best, replays_stored };
        self.selected_replay = (replays_stored > 0).then_some(0);
    }

    /// Whether the replay slot `slot` of the bar under the cursor holds a replay
    /// (`SelectableBar.existsReplay`).
    fn replay_exists(&self, slot: usize) -> bool {
        slot < self.focused_records.replays_stored.min(REPLAY_SLOTS)
    }

    /// `NEXT_REPLAY`: move the replay slot on to the next one that holds a replay, round the four,
    /// with the sound of a changed option. Nothing moves on a bar that opens, or with no other slot
    /// to move to. With no slot selected the search starts from the first, as the reference's does
    /// from its -1.
    pub(super) fn next_replay(&mut self, shared: &mut AppShared) {
        if self.focused_bar(shared) != FocusedBar::Playable {
            return;
        }
        let from = self.selected_replay.unwrap_or(REPLAY_SLOTS - 1);
        let next = (1..REPLAY_SLOTS).map(|step| (from + step) % REPLAY_SLOTS).find(|slot| self.replay_exists(*slot));
        if let Some(slot) = next {
            self.selected_replay = Some(slot);
            shared.play_system_sound(SystemSound::OptionChange);
        }
    }

    /// Make the course under the cursor as a skin reads it when the cursor has come to another
    /// course or the course list was read again: what it plays under, and the title of each stage,
    /// which is the library's own for a chart the library holds and the course's for one it does not
    /// (`GradeBar.getSongDatas`, `StringType.createCoursetitle`).
    fn refresh_course_shown(&mut self, shared: &AppShared) {
        let at = (self.tab == SelectTab::Courses).then(|| (self.courses_generation, self.courses.cursor()));
        if self.course_shown.as_ref().map(|(generation, cursor, _)| (*generation, *cursor)) == at {
            return;
        }
        self.course_shown = at.and_then(|(generation, cursor)| {
            let course = &self.courses.focused()?.course;
            let stages = course.charts.iter().map(|chart| match library_index(&shared.library, chart).and_then(|index| shared.library.songs().get(index)) {
                Some(held) => (held.title.as_str(), true),
                None => (chart.title.as_str(), false),
            });
            Some((generation, cursor, CourseShown::new(&course.constraints, stages)))
        });
    }
}
