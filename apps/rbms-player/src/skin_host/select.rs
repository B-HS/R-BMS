//! Property cluster F: the song browser.
//!
//! Where the values come from: the bar under the cursor and the panels over the browser: what kind
//! of bar it is, its lamp, its play counts, its folder's lamp totals, its replays and where the
//! cursor stands in the list. They are lent by the browser for one frame as a [`SelectShown`]
//! ([`shown`]).
//!
//! What this cluster answers:
//!
//! - options 1-3 and 5 (which kind of bar is under the cursor), 21-23 (the open panel), 100-105 and
//!   1100-1104 (the lamp on it), 196-198, 1196-1204 and 1205-1208 (its replays and the slot
//!   selected), 624-625 (whether a rival is compared), 1002-1007 and 1010-1017 (what a course
//!   forbids), 352-354 (who wins against the rival's score of zero), 1030-1031 (the random bars), and 200-207, 220-227, 300-307, 320-327 and 340-347
//!   (the rank of its score);
//! - the numbers of the score on the bar: 71-72, 75-89, 100-103, 105, 108, 110-116, 121-123, 128,
//!   135-136, 150-158, 170-178, 183-184, 243-249, 271 and 410-427, and the figures of a folder: 300
//!   and 320-330. The score cluster owns most of those ids and has no run on the browser, so they
//!   fall through to this one;
//! - rates 1 (where the cursor stands in the list), 110-115, 140-145 and 147; floats 85-89, 122,
//!   135, 155, 157, 183, 1102 and 1115;
//! - image indices 10-12 (the difficulty filter, the mode filter and the order), 89-90 (the star)
//!   and 370-371 (the lamp);
//! - strings 10 and 12 (the name of a folder), 30 (the search word), 60-62 (the filters and the
//!   order), 150-159 (the stages of a course) and 1000 (the open folders).
//!
//! A value is read as `IntegerPropertyFactory`, `BooleanPropertyFactory`, `FloatPropertyFactory`
//! and `StringPropertyFactory` read it for `MusicSelector`, quirks included. The score of the
//! browser is the one `BarManager` hands `ScoreDataProperty` whenever the cursor moves, with no
//! target and no rival set: so the best score and the target read zero, the rate over a bar that
//! has no score reads as a perfect one, the rival's rate reads a hundred percent, and the replay
//! options that ask whether a replay was saved read as the ones that ask whether there is none. A
//! number that needs a score the bar does not have reads absent, and so does one a bar of its kind
//! can never have.
//!
//! The search word reads empty, as the reference's reader returns (`StringType.searchword`). What is
//! typed is shown by the text input and the search is made when it is confirmed.
//!
//! Until the browser lends a frame this cluster answers nothing, so the ids fall through to whatever
//! else can.

use std::borrow::Cow;

use rbms_config::SortMode;
use rbms_course::CourseConstraint;
use rbms_judge::{ClearType, clear_type_id};
use rbms_model::Mode;
use rbms_render::skin_render::frame::{BarKind, LAMP_KINDS};
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};

use super::score::standing::{CUMULATIVE_RANK_STEPS, RANK_COUNT};
use super::score::{BAD, GREAT, JUDGE_KINDS, MISS, POOR, SIDES_PER_JUDGEMENT, whole};
use super::{ClusterState, INDEX_CLEAR, INDEX_DIFFICULTY_FILTER, INDEX_TARGET_CLEAR, STRING_DIFFICULTY_FILTER, STRING_MODE_FILTER, STRING_SORT};

mod bridge;
pub mod shown;

pub(crate) use bridge::BrowserLent;

pub use shown::{BarRecord, BarShown, ChartInputs, CourseShown, REPLAY_SLOTS, SelectInputs, SelectShown, stage_title};

/// The browser's difficulty filter and what it reads as when nothing is filtered
/// (`DifficultyFilter.ALL`). This player has no difficulty filter, so it always reads as that.
const DIFFICULTY_FILTER_ALL: i32 = 0;
const DIFFICULTY_FILTER_ALL_NAME: &str = "ALL";

/// The names and numbers of the mode filters (`ModeFilter`), by the mode they hold the list to.
const MODE_FILTERS: [(Mode, &str, i32); 6] = [
    (Mode::BEAT_5K, "5KEY", 1),
    (Mode::BEAT_7K, "7KEY", 2),
    (Mode::BEAT_10K, "10KEY", 3),
    (Mode::BEAT_14K, "14KEY", 4),
    (Mode::POPN_9K, "9KEY", 5),
    (Mode::KEYBOARD_24K, "24KEY", 6),
];

/// The mode filter that holds the list to no mode (`ModeFilter.ALL`).
const MODE_FILTER_ALL: (&str, i32) = ("ALL", 0);

/// The orderings the reference numbers (`BarSorter.defaultSorter`, which `Config.getSort` indexes)
/// and the name of each (`BarSorter.name`), then the ones past that list that it names without
/// numbering.
const SORT_NAMES: [(SortMode, &str); 12] = [
    (SortMode::Title, "TITLE"),
    (SortMode::Artist, "ARTIST"),
    (SortMode::Bpm, "BPM"),
    (SortMode::Length, "LENGTH"),
    (SortMode::Level, "LEVEL"),
    (SortMode::Clear, "CLEAR"),
    (SortMode::Score, "SCORE"),
    (SortMode::MissCount, "MISSCOUNT"),
    (SortMode::Duration, "DURATION"),
    (SortMode::LastUpdate, "LASTUPDATE"),
    (SortMode::RivalClear, "RIVALCOMPARE_CLEAR"),
    (SortMode::RivalScore, "RIVALCOMPARE_SCORE"),
];

/// How many of those orderings the reference's order switch cycles through and numbers.
const SORTS_NUMBERED: usize = 8;

/// What the order switch reads as under an ordering it does not number. The reference keeps the
/// number apart from the ordering in force (`Config.sort` beside `PlayerConfig.sortid`): the event
/// that steps through every ordering moves only the second, so the switch goes on showing the
/// number it had, which is this one until the switch itself is pressed. The ordering this player
/// starts on has no number either and reads the same, so the switch is always there to press. The
/// name of the ordering in force is what the string beside it says.
const SORT_UNNUMBERED: i32 = 0;

/// The clear lamps a bar's options name (`SelectedBarClearDrawCondition`), by the id each lamp has.
const CLEAR_OPTIONS: [(i32, ClearType); 10] = [
    (OPTION_SELECT_BAR_FAILED, ClearType::Failed),
    (OPTION_SELECT_BAR_ASSIST_EASY_CLEARED, ClearType::AssistEasy),
    (OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED, ClearType::LightAssistEasy),
    (OPTION_SELECT_BAR_EASY_CLEARED, ClearType::Easy),
    (OPTION_SELECT_BAR_NORMAL_CLEARED, ClearType::Normal),
    (OPTION_SELECT_BAR_HARD_CLEARED, ClearType::Hard),
    (OPTION_SELECT_BAR_EXHARD_CLEARED, ClearType::ExHard),
    (OPTION_SELECT_BAR_FULL_COMBO_CLEARED, ClearType::FullCombo),
    (OPTION_SELECT_BAR_PERFECT_CLEARED, ClearType::Perfect),
    (OPTION_SELECT_BAR_MAX_CLEARED, ClearType::Max),
];

/// What a course forbids or asks for, by the option each constraint answers
/// (`BooleanType.course_class` to `course_hcn`).
const CONSTRAINT_OPTIONS: [(i32, CourseConstraint); 14] = [
    (OPTION_GRADEBAR_CLASS, CourseConstraint::Class),
    (OPTION_GRADEBAR_MIRROR, CourseConstraint::Mirror),
    (OPTION_GRADEBAR_RANDOM, CourseConstraint::Random),
    (OPTION_GRADEBAR_NOSPEED, CourseConstraint::NoSpeed),
    (OPTION_GRADEBAR_NOGOOD, CourseConstraint::NoGood),
    (OPTION_GRADEBAR_NOGREAT, CourseConstraint::NoGreat),
    (OPTION_GRADEBAR_GAUGE_LR2, CourseConstraint::GaugeLr2),
    (OPTION_GRADEBAR_GAUGE_5KEYS, CourseConstraint::Gauge5Keys),
    (OPTION_GRADEBAR_GAUGE_7KEYS, CourseConstraint::Gauge7Keys),
    (OPTION_GRADEBAR_GAUGE_9KEYS, CourseConstraint::Gauge9Keys),
    (OPTION_GRADEBAR_GAUGE_24KEYS, CourseConstraint::Gauge24Keys),
    (OPTION_GRADEBAR_LN, CourseConstraint::Ln),
    (OPTION_GRADEBAR_CN, CourseConstraint::Cn),
    (OPTION_GRADEBAR_HCN, CourseConstraint::Hcn),
];

/// The replay slots' ids, in the order of the slots: the option that says a replay is there, the
/// one that says it is not, the one that says it was saved, and the one that says it is selected.
const REPLAY_OPTIONS: [[i32; 4]; REPLAY_SLOTS] = [
    [OPTION_REPLAYDATA, OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA_SAVED, OPTION_SELECT_REPLAYDATA],
    [OPTION_REPLAYDATA2, OPTION_NO_REPLAYDATA2, OPTION_REPLAYDATA2_SAVED, OPTION_SELECT_REPLAYDATA2],
    [OPTION_REPLAYDATA3, OPTION_NO_REPLAYDATA3, OPTION_REPLAYDATA3_SAVED, OPTION_SELECT_REPLAYDATA3],
    [OPTION_REPLAYDATA4, OPTION_NO_REPLAYDATA4, OPTION_REPLAYDATA4_SAVED, OPTION_SELECT_REPLAYDATA4],
];

/// The rate a score that was never set reads as (`ScoreDataProperty.update` over no score: a rate
/// over no notes is a whole one).
const RATE_OVER_NO_NOTES: f32 = 1.0;

/// A rate as a whole percent.
const PERCENT: i32 = 100;

/// A rate over the notes of the chart, which is a judgement count over them
/// (`FloatPropertyFactory.createJudgeRate`).
fn share_of(count: u32, notes: u32) -> f32 {
    count as f32 / notes as f32
}

/// Which rank an option id of a band that starts at `first` stands for, F as 0 up to AAA as 7 when
/// the band lists AAA first, or `None` for an id outside the band.
fn rank_in_band(id: i32, first: i32) -> Option<usize> {
    let step = usize::try_from(id - first).ok().filter(|step| *step < RANK_COUNT)?;
    Some(RANK_COUNT - 1 - step)
}

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct SelectState<'a> {
    /// The browser's frame, when it lends one.
    pub shown: Option<&'a SelectShown>,
}

impl<'a> SelectState<'a> {
    /// The frame of a browser.
    pub fn of(shown: &'a SelectShown) -> SelectState<'a> {
        SelectState { shown: Some(shown) }
    }

    /// The bar under the cursor.
    fn bar(&self) -> Option<&'a BarShown> {
        self.shown?.bar.as_ref()
    }

    /// The score on it.
    fn record(&self) -> Option<&'a BarRecord> {
        self.bar()?.record.as_ref()
    }

    /// The kind of the bar under the cursor.
    fn kind(&self) -> Option<BarKind> {
        self.bar().map(|bar| bar.kind)
    }

    /// The count of one folder's lamps added up, or none for a bar that is not a folder
    /// (`DirectoryBar.getLamps`). A folder that has not been counted holds none of anything.
    fn folder_lamps(&self) -> Option<[u32; LAMP_KINDS]> {
        let bar = self.bar().filter(|bar| bar.kind.is_directory())?;
        Some(bar.distribution.map_or([0; LAMP_KINDS], |distribution| distribution.lamps))
    }

    /// The numbers of the score on the bar, which read absent while it has none.
    fn score_number(&self, id: i32) -> Option<i32> {
        let shown = self.shown?;
        let standing = &shown.standing;
        let record = self.record();
        let sheet = record.map(|record| &record.sheet);
        let present = |value: i32| if record.is_some() { value } else { INTEGER_ABSENT };
        let count = |judge: i32| sheet.map_or(0, |sheet| whole(sheet.count(judge)));
        let count_on = |judge: i32, early: bool| sheet.map_or(0, |sheet| whole(sheet.count_on(judge, early)));
        Some(match id {
            NUMBER_SCORE | NUMBER_SCORE2 | NUMBER_SCORE3 => present(standing.now_ex),
            NUMBER_MAXSCORE => sheet.map_or(0, |sheet| whole(sheet.max_ex_score())),
            NUMBER_MAXCOMBO | NUMBER_MAXCOMBO2 | NUMBER_MAXCOMBO3 => sheet.map_or(INTEGER_ABSENT, |sheet| whole(sheet.max_combo)),
            NUMBER_MISSCOUNT | NUMBER_MISSCOUNT2 => sheet.map_or(INTEGER_ABSENT, |sheet| whole(sheet.min_bp)),
            NUMBER_PLAYCOUNT => record.map_or(INTEGER_ABSENT, |record| whole(record.play_count)),
            NUMBER_CLEARCOUNT => record.map_or(INTEGER_ABSENT, |record| whole(record.clear_count)),
            NUMBER_FAILCOUNT => record.map_or(INTEGER_ABSENT, |record| whole(record.play_count) - whole(record.clear_count)),
            NUMBER_PERFECT2..=NUMBER_POOR2 => sheet.map_or(INTEGER_ABSENT, |sheet| whole(sheet.count(id - NUMBER_PERFECT2))),
            NUMBER_PERFECT_RATE..=NUMBER_POOR_RATE => match sheet {
                Some(sheet) if sheet.notes > 0 => whole(sheet.count(id - NUMBER_PERFECT_RATE)) * PERCENT / whole(sheet.notes),
                _ => INTEGER_ABSENT,
            },
            NUMBER_POINT => standing.now_point,
            NUMBER_SCORE_RATE => present(standing.now_rate_int),
            NUMBER_SCORE_RATE_AFTERDOT => present(standing.now_rate_after_dot),
            NUMBER_TOTAL_RATE | NUMBER_SCORE_RATE2 => present(standing.rate_int),
            NUMBER_TOTAL_RATE_AFTERDOT | NUMBER_SCORE_RATE_AFTERDOT2 => present(standing.rate_after_dot),
            NUMBER_DIFF_EXSCORE | NUMBER_DIFF_EXSCORE2 | NUMBER_DIFF_TARGETSCORE | NUMBER_DIFF_HIGHSCORE | NUMBER_DIFF_HIGHSCORE2 => standing.now_ex,
            NUMBER_DIFF_NEXTRANK => standing.next_rank,
            NUMBER_HIGHSCORE | NUMBER_HIGHSCORE2 | NUMBER_BEST_RATE | NUMBER_BEST_RATE_AFTERDOT => 0,
            NUMBER_TARGET_SCORE
            | NUMBER_TARGET_SCORE2
            | NUMBER_RIVAL_SCORE
            | NUMBER_TARGET_SCORE_RATE_AFTERDOT
            | NUMBER_TARGET_TOTAL_RATE_AFTERDOT
            | NUMBER_TARGET_SCORE_RATE_AFTERDOT2 => 0,
            NUMBER_TARGET_MAXCOMBO | NUMBER_DIFF_MAXCOMBO | NUMBER_TARGET_MISSCOUNT | NUMBER_DIFF_MISSCOUNT => INTEGER_ABSENT,
            NUMBER_TARGET_SCORE_RATE | NUMBER_TARGET_TOTAL_RATE | NUMBER_TARGET_SCORE_RATE2 => PERCENT,
            NUMBER_PERFECT..=NUMBER_POOR => count(id - NUMBER_PERFECT),
            NUMBER_EARLY_PERFECT..=NUMBER_LATE_POOR => {
                let offset = id - NUMBER_EARLY_PERFECT;
                count_on(offset / SIDES_PER_JUDGEMENT, offset % SIDES_PER_JUDGEMENT == 0)
            }
            NUMBER_MISS => count(MISS),
            NUMBER_EARLY_MISS => count_on(MISS, true),
            NUMBER_LATE_MISS => count_on(MISS, false),
            NUMBER_TOTALEARLY => (GREAT..JUDGE_KINDS).map(|judge| count_on(judge, true)).sum(),
            NUMBER_TOTALLATE => (GREAT..JUDGE_KINDS).map(|judge| count_on(judge, false)).sum(),
            NUMBER_COMBOBREAK => count(BAD) + count(POOR),
            NUMBER_POOR_PLUS_MISS => count(POOR) + count(MISS),
            NUMBER_BAD_PLUS_POOR_PLUS_MISS => count(BAD) + count(POOR) + count(MISS),
            NUMBER_LASTPLAY_TIMESTAMP => {
                record.map(|record| record.last_played).filter(|date| (1..=i64::from(i32::MAX)).contains(date)).map_or(INTEGER_ABSENT, |date| date as i32)
            }
            NUMBER_LASTPLAY_YEAR..=NUMBER_LASTPLAY_SECOND => {
                let clock = record.and_then(|record| record.last_played_clock);
                clock.map_or(INTEGER_ABSENT, |clock| match id {
                    NUMBER_LASTPLAY_YEAR => clock.year,
                    NUMBER_LASTPLAY_MONTH => clock.month,
                    NUMBER_LASTPLAY_DAY => clock.day,
                    NUMBER_LASTPLAY_HOUR => clock.hour,
                    NUMBER_LASTPLAY_MINUTE => clock.minute,
                    _ => clock.second,
                })
            }
            _ => return None,
        })
    }
}

impl ClusterState for SelectState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let shown = self.shown?;
        let kind = self.kind();
        let bar = self.bar();
        let clear = self.record().map(|record| record.sheet.clear);
        let replays = bar.and_then(|bar| bar.replays);
        let constraints = bar.and_then(|bar| bar.course.as_ref()).map(|course| course.constraints.as_slice());
        if let Some((_, lamp)) = CLEAR_OPTIONS.iter().find(|(option, _)| *option == id) {
            return Some(clear == Some(clear_type_id(*lamp)));
        }
        if let Some((_, constraint)) = CONSTRAINT_OPTIONS.iter().find(|(option, _)| *option == id) {
            return Some(constraints.is_some_and(|constraints| constraints.contains(constraint)));
        }
        let standing = &shown.standing;
        if let Some(rank) = rank_in_band(id, OPTION_1P_AAA).or_else(|| rank_in_band(id, OPTION_RESULT_AAA_1P)).or_else(|| rank_in_band(id, OPTION_NOW_AAA_1P)) {
            return Some(standing.now_rank_band(rank));
        }
        if rank_in_band(id, OPTION_BEST_AAA_1P).is_some() {
            return Some(false);
        }
        if let Some(rank) = rank_in_band(id, OPTION_AAA) {
            return Some(standing.secured(CUMULATIVE_RANK_STEPS[RANK_COUNT - 1 - rank]));
        }
        if let Some(slot) = REPLAY_OPTIONS.iter().position(|options| options.contains(&id)) {
            let [exists, none, saved, selected] = REPLAY_OPTIONS[slot];
            let there = replays.map(|replays| replays[slot]);
            return Some(match id {
                _ if id == exists => there == Some(true),
                _ if id == none || id == saved => there == Some(false),
                _ if id == selected => shown.selected_replay == Some(slot),
                _ => return None,
            });
        }
        Some(match id {
            OPTION_FOLDERBAR => kind.is_some_and(BarKind::is_directory),
            OPTION_SONGBAR => matches!(kind, Some(BarKind::Song { .. })),
            OPTION_GRADEBAR => matches!(kind, Some(BarKind::Course { .. })),
            OPTION_PLAYABLEBAR => matches!(
                kind,
                Some(BarKind::Song { exists: true } | BarKind::Course { complete: true } | BarKind::RandomCourse { complete: true } | BarKind::Executable)
            ),
            OPTION_RANDOMSELECTBAR => matches!(kind, Some(BarKind::Executable)),
            OPTION_RANDOMCOURSEBAR => matches!(kind, Some(BarKind::RandomCourse { .. })),
            OPTION_PANEL1..=OPTION_PANEL3 => return shown.panel.map(|panel| i32::from(panel) == id - OPTION_PANEL1 + 1),
            OPTION_NOT_COMPARE_RIVAL => true,
            OPTION_COMPARE_RIVAL => false,
            OPTION_1PWIN => standing.now_ex > 0,
            OPTION_2PWIN => standing.now_ex < 0,
            OPTION_DRAW => standing.now_ex == 0,
            OPTION_SELECT_BAR_NOT_PLAYED => {
                matches!(kind, Some(BarKind::Song { .. } | BarKind::Course { .. })) && clear.is_none_or(|clear| clear == clear_type_id(ClearType::NoPlay))
            }
            _ => return None,
        })
    }

    fn integer(&self, id: i32) -> Option<i32> {
        let lamps = self.folder_lamps();
        match id {
            NUMBER_FOLDER_TOTALSONGS => Some(lamps.map_or(INTEGER_ABSENT, |lamps| lamps.iter().map(|count| whole(*count)).sum())),
            NUMBER_FOLDER_NOPLAY..=NUMBER_FOLDER_MAX => {
                let lamp = usize::try_from(id - NUMBER_FOLDER_NOPLAY).ok()?;
                Some(lamps.and_then(|lamps| lamps.get(lamp).copied()).map_or(INTEGER_ABSENT, whole))
            }
            _ => self.score_number(id),
        }
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        let shown = self.shown?;
        Some(match id {
            INDEX_DIFFICULTY_FILTER => DIFFICULTY_FILTER_ALL,
            BUTTON_MODE => {
                shown.mode_filter.and_then(|mode| MODE_FILTERS.iter().find(|(filter, ..)| *filter == mode)).map_or(MODE_FILTER_ALL.1, |(.., number)| *number)
            }
            BUTTON_SORT => SORT_NAMES
                .iter()
                .take(SORTS_NUMBERED)
                .position(|(sort, _)| *sort == shown.sort)
                .and_then(|place| i32::try_from(place).ok())
                .unwrap_or(SORT_UNNUMBERED),
            INDEX_CLEAR => self.record().map_or(IMAGE_INDEX_ABSENT, |record| i32::from(record.sheet.clear)),
            INDEX_TARGET_CLEAR | BUTTON_FAVORITTE_SONG => IMAGE_INDEX_ABSENT,
            BUTTON_FAVORITTE_CHART => self.bar().and_then(|bar| bar.favorite).map_or(IMAGE_INDEX_ABSENT, i32::from),
            _ => return None,
        })
    }

    fn rate(&self, id: i32) -> Option<f32> {
        let shown = self.shown?;
        let standing = &shown.standing;
        let sheet = self.record().map(|record| &record.sheet);
        let judged = |judge: i32| sheet.map_or(0.0, |sheet| share_of(sheet.count(judge), sheet.notes));
        Some(match id {
            RATE_MUSICSELECT_POSITION => {
                if shown.total == 0 {
                    0.0
                } else {
                    shown.index as f32 / shown.total as f32
                }
            }
            RATE_SCORE => standing.rate,
            RATE_SCORE_FINAL => standing.now_rate,
            RATE_BESTSCORE_NOW | RATE_BESTSCORE | RATE_TARGETSCORE_NOW => 0.0,
            RATE_TARGETSCORE => RATE_OVER_NO_NOTES,
            RATE_PGREAT..=RATE_POOR => judged(id - RATE_PGREAT),
            RATE_MAXCOMBO => sheet.map_or(0.0, |sheet| share_of(sheet.max_combo, sheet.notes)),
            RATE_EXSCORE => sheet.map_or(0.0, |sheet| share_of(sheet.ex_score(), sheet.notes) / 2.0),
            _ => return None,
        })
    }

    fn float(&self, id: i32) -> Option<f32> {
        let shown = self.shown?;
        let standing = &shown.standing;
        let sheet = self.record().map(|record| &record.sheet);
        Some(match id {
            FLOAT_PERFECT_RATE..=FLOAT_POOR_RATE => match sheet {
                Some(sheet) if sheet.notes > 0 => share_of(sheet.count(id - FLOAT_PERFECT_RATE), sheet.notes),
                _ => FLOAT_ABSENT,
            },
            FLOAT_SCORE_RATE => sheet.map_or(FLOAT_ABSENT, |_| standing.now_rate),
            FLOAT_TOTAL_RATE | FLOAT_SCORE_RATE2 => sheet.map_or(FLOAT_ABSENT, |_| standing.rate),
            FLOAT_BEST_RATE => 0.0,
            FLOAT_RIVAL_RATE | FLOAT_TARGET_RATE | FLOAT_TARGET_RATE2 => RATE_OVER_NO_NOTES,
            _ => return None,
        })
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        let shown = self.shown?;
        let stage = |place: i32| {
            let stages = self.bar().and_then(|bar| bar.course.as_ref()).map(|course| course.stages.as_slice());
            usize::try_from(place).ok().and_then(|place| stages?.get(place)).map_or(Cow::Borrowed(""), |title| Cow::Borrowed(title.as_str()))
        };
        match id {
            STRING_TITLE | STRING_FULLTITLE => self.bar().filter(|bar| bar.kind.is_directory()).map(|bar| Cow::Borrowed(bar.title.as_str())),
            STRING_SEARCHWORD => Some(Cow::Borrowed("")),
            STRING_MODE_FILTER => Some(Cow::Borrowed(
                shown.mode_filter.and_then(|mode| MODE_FILTERS.iter().find(|(filter, ..)| *filter == mode)).map_or(MODE_FILTER_ALL.0, |(_, name, _)| name),
            )),
            STRING_SORT => Some(Cow::Borrowed(SORT_NAMES.iter().find(|(sort, _)| *sort == shown.sort).map_or("", |(_, name)| name))),
            STRING_DIFFICULTY_FILTER => Some(Cow::Borrowed(DIFFICULTY_FILTER_ALL_NAME)),
            STRING_COURSE1_TITLE..=STRING_COURSE10_TITLE => Some(stage(id - STRING_COURSE1_TITLE)),
            STRING_DIRECTORY => Some(Cow::Owned(shown.directory_path())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
