//! What a skin reads off the song browser beyond the bar under the cursor: the other charts of the
//! song the cursor is on, how the charts of the open folder are cleared, and the player's own plays
//! of the chart. They are this player's own ids ([`crate::skin_host::extension`]); the reference has
//! nothing under them.
//!
//! [`BrowseFacts`] is plain data. The browser works it out when the cursor, the list or the records
//! move (`stage/select/browse_facts.rs`) and lends it for the frames in between, and the browser
//! cluster answers the ids from it. Every id of the runs below is answered while the browser lends a
//! frame, with the absent value of its kind where there is nothing to say: no chart in a slot, no
//! play in a row. A folder with no chart in it holds none on any lamp, and every share of it is zero.
//!
//! - booleans [`RBMS_OPTION_SIBLING_CHART_FIRST`] on: slot `s` of the song holds a chart;
//! - numbers [`RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST`] on: the level of the chart in slot `s`;
//! - numbers [`RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST`] on, and [`RBMS_NUMBER_FOLDER_CHART_COUNT`]: how
//!   many charts of the open folder sit on each lamp, and how many it holds;
//! - rates [`RBMS_RATE_FOLDER_LAMP_FIRST`] on: each lamp's share of those charts;
//! - numbers [`RBMS_NUMBER_MY_RECORD_EX_FIRST`], [`RBMS_NUMBER_MY_RECORD_LAMP_FIRST`],
//!   [`RBMS_NUMBER_MY_RECORD_MISS_COUNT_FIRST`] and [`RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST`] on, and
//!   texts [`RBMS_TEXT_MY_RECORD_DATE_FIRST`] on: the figures and the date of each record row.

use std::borrow::Cow;

use rbms_render::dj_rank;
use rbms_render::skin_render::frame::LAMP_KINDS;
use rbms_skin::lua::{LocalTime, local_time};
use rbms_skin::property::{INTEGER_ABSENT, TEXT_ABSENT};

use crate::skin_host::extension::{
    MY_RECORD_ROWS, RBMS_NUMBER_FOLDER_CHART_COUNT, RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST, RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST, RBMS_NUMBER_MY_RECORD_EX_FIRST,
    RBMS_NUMBER_MY_RECORD_LAMP_FIRST, RBMS_NUMBER_MY_RECORD_MISS_COUNT_FIRST, RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST, RBMS_OPTION_SIBLING_CHART_FIRST,
    RBMS_RATE_FOLDER_LAMP_FIRST, RBMS_TEXT_MY_RECORD_DATE_FIRST, SIBLING_CHART_SLOTS,
};
use crate::skin_host::score::standing::ScoreSheet;
use crate::skin_host::score::whole;

/// How many other charts of a song a skin can name by slot, the BMS difficulties BEGINNER to INSANE.
pub const SIBLING_SLOTS: usize = SIBLING_CHART_SLOTS as usize;

/// How many rows of the player's own plays of a chart a skin can read: the best, then the latest.
pub const RECORD_ROWS: usize = MY_RECORD_ROWS as usize;

/// A chart of the song under the cursor, as far as a skin reads one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiblingChart {
    /// The chart's level (`#PLAYLEVEL`), zero when the header states none a number can be read from.
    pub level: i32,
}

/// One row of the player's own plays of the chart under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRow {
    /// The EX score of the play.
    pub ex_score: i32,
    /// The lamp the play earned, as the reference numbers its `ClearType`s.
    pub lamp: i32,
    /// The bad, poor and miss count of the play.
    pub miss_count: i32,
    /// The DJ LEVEL of the play: 0 for F up to 7 for AAA (`rbms_render::dj_rank`).
    pub dj_level: i32,
    /// When the play was made in the machine's local time, or nothing where the play carries no date.
    pub date: String,
}

impl RecordRow {
    /// The row of a play whose figures are `sheet`, made at `date` seconds after the epoch.
    pub fn of(sheet: &ScoreSheet, date: Option<i64>) -> RecordRow {
        let (ex_score, max_ex_score) = (sheet.ex_score(), sheet.max_ex_score());
        RecordRow {
            ex_score: whole(ex_score),
            lamp: i32::from(sheet.clear),
            miss_count: whole(sheet.min_bp),
            dj_level: i32::try_from(dj_rank(ex_score, max_ex_score)).unwrap_or_default(),
            date: date.filter(|date| *date > 0).and_then(local_time).map(clock_text).unwrap_or_default(),
        }
    }
}

/// A moment the way the browser writes the moment of a record: the date, then the time to the
/// minute.
pub fn clock_text(time: LocalTime) -> String {
    format!("{:04}-{:02}-{:02} {:02}:{:02}", time.year, time.month, time.day, time.hour, time.minute)
}

/// What the browser works out about the chart under the cursor and the folder it is in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrowseFacts {
    /// The charts of the song under the cursor by difficulty slot, BEGINNER first.
    pub siblings: [Option<SiblingChart>; SIBLING_SLOTS],
    /// How many charts of the open folder are on each lamp, no play first.
    pub folder_lamps: [u32; LAMP_KINDS],
    /// The best play of the chart and the latest ones, newest first.
    pub records: [Option<RecordRow>; RECORD_ROWS],
}

/// The place of `id` in the run of `count` ids that starts at `first`.
fn place_in(id: i32, first: i32, count: usize) -> Option<usize> {
    usize::try_from(id.checked_sub(first)?).ok().filter(|place| *place < count)
}

/// The first id of each run of record figures, in the order of [`RecordRow`]: the EX score, the lamp,
/// the miss count and the DJ LEVEL.
const RECORD_FIGURES: [i32; 4] =
    [RBMS_NUMBER_MY_RECORD_EX_FIRST, RBMS_NUMBER_MY_RECORD_LAMP_FIRST, RBMS_NUMBER_MY_RECORD_MISS_COUNT_FIRST, RBMS_NUMBER_MY_RECORD_DJ_LEVEL_FIRST];

impl BrowseFacts {
    /// How many charts the open folder holds: the lamps added up.
    pub fn folder_chart_count(&self) -> u32 {
        self.folder_lamps.iter().sum()
    }

    /// The options of the slots: on while the song holds a chart there.
    pub fn boolean(&self, id: i32) -> Option<bool> {
        let slot = place_in(id, RBMS_OPTION_SIBLING_CHART_FIRST, SIBLING_SLOTS)?;
        Some(self.siblings[slot].is_some())
    }

    /// The levels of the slots, the charts of the folder by lamp and altogether, and the figures of
    /// the record rows.
    pub fn integer(&self, id: i32) -> Option<i32> {
        if let Some(slot) = place_in(id, RBMS_NUMBER_SIBLING_CHART_LEVEL_FIRST, SIBLING_SLOTS) {
            return Some(self.siblings[slot].map_or(INTEGER_ABSENT, |chart| chart.level));
        }
        if let Some(lamp) = place_in(id, RBMS_NUMBER_FOLDER_LAMP_COUNT_FIRST, LAMP_KINDS) {
            return Some(whole(self.folder_lamps[lamp]));
        }
        if id == RBMS_NUMBER_FOLDER_CHART_COUNT {
            return Some(whole(self.folder_chart_count()));
        }
        let (figure, row) = RECORD_FIGURES.iter().enumerate().find_map(|(figure, first)| Some((figure, place_in(id, *first, RECORD_ROWS)?)))?;
        let Some(record) = &self.records[row] else {
            return Some(INTEGER_ABSENT);
        };
        Some(match figure {
            0 => record.ex_score,
            1 => record.lamp,
            2 => record.miss_count,
            _ => record.dj_level,
        })
    }

    /// The share of the open folder's charts each lamp holds, nothing for a folder with no chart.
    pub fn rate(&self, id: i32) -> Option<f32> {
        let lamp = place_in(id, RBMS_RATE_FOLDER_LAMP_FIRST, LAMP_KINDS)?;
        let total = self.folder_chart_count();
        Some(if total == 0 { 0.0 } else { self.folder_lamps[lamp] as f32 / total as f32 })
    }

    /// The dates of the record rows.
    pub fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        let row = place_in(id, RBMS_TEXT_MY_RECORD_DATE_FIRST, RECORD_ROWS)?;
        Some(Cow::Borrowed(self.records[row].as_ref().map_or(TEXT_ABSENT, |record| record.date.as_str())))
    }
}

#[cfg(test)]
mod tests;
