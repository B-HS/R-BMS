//! Property cluster H: the internet ranking and rivals.
//!
//! Where the values come from: what the ranking service answered: connection state, this player's
//! rank, the ranking's clear and score spread, the top ten, and the rival's score. This build asks
//! the service one thing, to take the score of the run that has just ended, and keeps how that went
//! in [`IrStatus`]. That is the only part of the cluster with a source: whether a service is set up
//! at all, and whether the run was sent and how it fared. The place the service ranked the run at
//! is kept there as text, not as a number, so the rank and the previous rank are zero until the
//! caller, which knows them, sets them on the [`IrLink`]. The frame of a result screen fills the link
//! in from that status, and the result scenes switch the connection timers from the same link
//! ([`IrLink::timers`]), so what a skin's options say and what its timers show are one reading. How many players the ranking has, and how
//! far it is scrolled, are the result screen's own and arrive in its scene. The ranking's own tables --
//! who else is on it, the spread of lamps, the rivals' scores -- are not read from the service at
//! all, and a skin that asks for one reads the absent value of its kind, as it does when the
//! reference has not heard back.
//!
//! What this cluster answers: numbers 179-180, 182, 200, 202-220, 222-242, 280-289 and 380-399;
//! floats 203, 205, 207, 209, 211, 213, 215, 217, 219, 223, 225, 227, 229 and 285-289; options
//! 50-51, 603-604, 606 and 608; image indices 380-399; rate 8, how far the ranking is scrolled;
//! strings 1, 3, 120-129 and 1020-1021. The target's own score (271) is cluster B's.
//!
//! Whether the run is on the ranking is read as the reference's result screen reads it
//! (`AbstractResult.getState`): offline until a submission is under way, then processing, then
//! finished. The rank, the previous rank and the number of players are zero while the ranking is
//! empty and absent only offline, and the options of the browser's ranking panel are off.
//!
//! On the song browser the cluster answers from the ranking of the chart under the cursor instead
//! ([`IrBrowser`], what `MusicSelector.getCurrentRankingData` holds): whether it is being fetched,
//! was fetched or failed, the player's place on it, how many scores came back, and how many of those
//! sit on each lamp. The reference counts the scores the service returned; this player asks the
//! service for the top of the board, so the count is of the scores it got. The ranking's own list
//! -- who is on it, with what -- and the rivals are not read from the board here, and a skin that
//! asks for one reads the absent value of its kind. The connection timers of the browser are not
//! the result screen's: the begin timer is on only while the fetch is under way
//! ([`IrBrowser::timers`]).
//!
//! A cluster that has neither a link nor a browser answers nothing, so the ids fall through to
//! whatever else can.

use std::borrow::Cow;

use rbms_judge::clear_type_from_id;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};
use rbms_skin::timer::{TimerId, timer_id};

use super::{ClusterState, ResultScene, STRING_TARGET_NAME};
use crate::format::clear_label_color;
use crate::ir_outcome::IrStatus;
use crate::ir_ranking::RankingState;

/// How the submission of the run stands (`AbstractResult.STATE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IrPhase {
    /// Nothing is being sent: no service is set up, or the run was never eligible.
    #[default]
    Offline,
    /// The score is on its way to the service.
    Processing,
    /// The service has answered, with a score taken or an error.
    Finished {
        /// Whether the service took the score.
        succeeded: bool,
    },
}

impl From<&IrStatus> for IrPhase {
    /// How the player's submission stands: a run that was never sent and one with no service to send
    /// to read alike, as offline. The status keeps the service's answer as a line of text, not as
    /// numbers.
    fn from(status: &IrStatus) -> IrPhase {
        match status {
            IrStatus::Off | IrStatus::Skipped(_) => IrPhase::Offline,
            IrStatus::Sending => IrPhase::Processing,
            IrStatus::Reported(report) => IrPhase::Finished { succeeded: !report.failed },
        }
    }
}

/// Which of the three connection timers the phase has switched on
/// (`TIMER_IR_CONNECT_BEGIN`, `_SUCCESS` and `_FAIL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IrTimers {
    pub begin: bool,
    pub success: bool,
    pub fail: bool,
}

impl IrTimers {
    /// The timers that are on, with the id of each.
    pub fn on(self) -> impl Iterator<Item = TimerId> {
        [(self.begin, timer_id::IR_CONNECT_BEGIN), (self.success, timer_id::IR_CONNECT_SUCCESS), (self.fail, timer_id::IR_CONNECT_FAIL)]
            .into_iter()
            .filter_map(|(on, id)| on.then_some(id))
    }
}

/// How the run stands with the ranking service, at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IrLink {
    /// Whether a service is set up (`main.getIRStatus().length > 0`).
    pub online: bool,
    pub phase: IrPhase,
    /// The place the service ranked the run at, or zero while it has not (`getIRRank`).
    pub rank: i32,
    /// The place the player held before, or zero (`getOldIRRank`).
    pub previous_rank: i32,
}

impl IrLink {
    /// The link of a run whose submission stands in `phase`. The rank and the previous rank are zero
    /// until the caller, which knows them, sets them.
    pub fn new(online: bool, phase: IrPhase) -> IrLink {
        IrLink { online, phase, ..IrLink::default() }
    }

    /// The connection timers this phase has switched on: one when the send starts, and one more
    /// for how it ended (`MusicResult.prepare`).
    pub fn timers(&self) -> IrTimers {
        match self.phase {
            IrPhase::Offline => IrTimers::default(),
            IrPhase::Processing => IrTimers { begin: true, ..IrTimers::default() },
            IrPhase::Finished { succeeded } => IrTimers { begin: true, success: succeeded, fail: !succeeded },
        }
    }

    /// A number of the ranking, absent while no submission is under way.
    fn ranking_number(&self, number: i32) -> i32 {
        if self.phase == IrPhase::Offline { INTEGER_ABSENT } else { number }
    }
}

/// How many clear lamps a ranking is spread over, no play to max (`RankingData.lamps`).
pub const CLEAR_LAMPS: usize = 11;

/// How the fetch of a ranking stands (`RankingData.ACCESS`, `FINISH` and `FAIL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrAccess {
    /// The service is being asked.
    Accessing,
    /// The service has answered with scores.
    Finished,
    /// The service could not be reached or refused.
    Failed,
}

/// The ranking of one chart as the browser holds it (`RankingData`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrBoard {
    pub access: IrAccess,
    /// The place the player holds, or zero when the player is not on it (`getRank`).
    pub rank: i32,
    /// How many scores the ranking holds (`getTotalPlayer`), zero until it has been fetched.
    pub total: i32,
    /// How many of those sit on each lamp, from no play to max (`getClearCount`).
    pub lamps: [i32; CLEAR_LAMPS],
}

/// The ranking of the chart under the browser's cursor, and whether there is a service at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IrBrowser {
    /// Whether a service is set up (`main.getIRStatus().length > 0`).
    pub online: bool,
    /// The ranking held for the bar under the cursor, or `None` while there is none
    /// (`getCurrentRankingData() == null`).
    pub board: Option<IrBoard>,
    /// How far down the ranking the list is scrolled (`MusicSelector.getRankingOffset`).
    pub offset: i32,
}

impl IrBrowser {
    /// The browser's view of a ranking the panel holds for the chart under the cursor. A browser
    /// with no service holds no ranking, whatever the cache has.
    pub(crate) fn of_ranking(online: bool, ranking: Option<&RankingState>) -> IrBrowser {
        let board = ranking.filter(|_| online).map(|state| match state {
            RankingState::Loading => IrBoard { access: IrAccess::Accessing, rank: 0, total: 0, lamps: [0; CLEAR_LAMPS] },
            RankingState::Failed(_) => IrBoard { access: IrAccess::Failed, rank: 0, total: 0, lamps: [0; CLEAR_LAMPS] },
            RankingState::Ready(board) => {
                let mut lamps = [0; CLEAR_LAMPS];
                for row in &board.rows {
                    let lamp =
                        lamps.iter_mut().enumerate().find(|(id, _)| u8::try_from(*id).is_ok_and(|id| clear_label_color(clear_type_from_id(id)).0 == row.lamp));
                    if let Some((_, count)) = lamp {
                        *count += 1;
                    }
                }
                IrBoard {
                    access: IrAccess::Finished,
                    rank: board.you.as_ref().and_then(|you| you.rank).map_or(0, |rank| i32::try_from(rank).unwrap_or(i32::MAX)),
                    total: i32::try_from(board.rows.len()).unwrap_or(i32::MAX),
                    lamps,
                }
            }
        });
        IrBrowser { online, board, offset: 0 }
    }

    /// The connection timers of the browser (`MusicSelector.render`): each is on exactly while the
    /// ranking is in the state it is named for, and none is on while there is no ranking.
    pub fn timers(&self) -> IrTimers {
        let access = self.board.map(|board| board.access);
        IrTimers { begin: access == Some(IrAccess::Accessing), success: access == Some(IrAccess::Finished), fail: access == Some(IrAccess::Failed) }
    }

    /// The ranking, once it has been fetched.
    fn finished(&self) -> Option<IrBoard> {
        self.board.filter(|board| board.access == IrAccess::Finished)
    }

    /// How many of the scores sit on the lamps `of` names, once fetched.
    fn count(&self, of: impl Iterator<Item = usize>) -> Option<i32> {
        self.finished().map(|board| of.filter_map(|lamp| board.lamps.get(lamp)).sum())
    }

    fn boolean(&self, id: i32) -> Option<bool> {
        Some(match id {
            OPTION_OFFLINE => !self.online,
            OPTION_ONLINE => self.online,
            OPTION_IR_NOPLAYER => self.finished().is_some_and(|board| board.total == 0),
            OPTION_IR_FAILED | OPTION_IR_BUSY => self.board.is_some_and(|board| board.access == IrAccess::Failed),
            OPTION_IR_WAITING => self.board.is_none(),
            _ => return None,
        })
    }

    fn integer(&self, id: i32) -> Option<i32> {
        let finished = self.finished();
        let share = |count: Option<i32>, scale: i32| match (count, finished) {
            (Some(count), Some(board)) if board.total > 0 => Some((i64::from(count) * i64::from(scale) / i64::from(board.total)) as i32),
            _ => None,
        };
        let tenths = |count: Option<i32>| share(count, PER_MILLE).map(|thousandths| thousandths % TENTHS_MODULUS);
        let lamp_of = |ids: [i32; CLEAR_LAMPS]| ids.iter().position(|lamp| *lamp == id);
        if let Some(lamp) = lamp_of(CLEAR_COUNT_NUMBERS) {
            return Some(self.count(std::iter::once(lamp)).unwrap_or(INTEGER_ABSENT));
        }
        if let Some(lamp) = lamp_of(CLEAR_RATE_NUMBERS) {
            return Some(share(self.count(std::iter::once(lamp)), PERCENT).unwrap_or(INTEGER_ABSENT));
        }
        if let Some(lamp) = lamp_of(CLEAR_RATE_AFTERDOT_NUMBERS) {
            return Some(tenths(self.count(std::iter::once(lamp))).unwrap_or(INTEGER_ABSENT));
        }
        Some(match id {
            NUMBER_IR_RANK => finished.map_or(INTEGER_ABSENT, |board| board.rank),
            NUMBER_IR_TOTALPLAYER | NUMBER_IR_TOTALPLAYER2 => finished.map_or(INTEGER_ABSENT, |board| board.total),
            NUMBER_IR_PREVRANK | NUMBER_IR_UPDATE_WAITING_TIME => INTEGER_ABSENT,
            NUMBER_IR_PLAYER_TOTAL_CLEAR => self.count(CLEARED).unwrap_or(INTEGER_ABSENT),
            NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE => share(self.count(CLEARED), PERCENT).unwrap_or(INTEGER_ABSENT),
            NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE_AFTERDOT => tenths(self.count(CLEARED)).unwrap_or(INTEGER_ABSENT),
            NUMBER_IR_PLAYER_TOTAL_FULLCOMBO => self.count(FULL_COMBO_UP).unwrap_or(INTEGER_ABSENT),
            NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE => share(self.count(FULL_COMBO_UP), PERCENT).unwrap_or(INTEGER_ABSENT),
            NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT => tenths(self.count(FULL_COMBO_UP)).unwrap_or(INTEGER_ABSENT),
            NUMBER_RIVAL_PERFECT..=NUMBER_RIVAL_POOR_RATE | NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR => INTEGER_ABSENT,
            _ => return None,
        })
    }

    fn float(&self, id: i32) -> Option<f32> {
        let finished = self.finished();
        let share = |count: Option<i32>| match (count, finished) {
            (Some(count), Some(board)) if board.total > 0 => count as f32 / board.total as f32,
            _ => FLOAT_ABSENT,
        };
        if let Some(lamp) = CLEAR_RATE_FLOATS.iter().position(|float| *float == id) {
            return Some(share(self.count(std::iter::once(lamp))));
        }
        match id {
            FLOAT_IR_TOTALCLEARRATE => Some(share(self.count(CLEARED))),
            FLOAT_IR_TOTALFULLCOMBORATE => Some(share(self.count(FULL_COMBO_UP))),
            FLOAT_RIVAL_PERFECT_RATE..=FLOAT_RIVAL_POOR_RATE => Some(FLOAT_ABSENT),
            _ => None,
        }
    }

    fn rate(&self, id: i32) -> Option<f32> {
        (id == RATE_RANKING_POSITION).then(|| self.offset as f32 / self.board.map_or(1, |board| board.total.max(1)) as f32)
    }
}

/// The numbers that count the scores on each lamp, from no play to max
/// (`IntegerPropertyPattern.IR_CLEAR_COUNT`).
const CLEAR_COUNT_NUMBERS: [i32; CLEAR_LAMPS] = [
    NUMBER_IR_PLAYER_NOPLAY,
    NUMBER_IR_PLAYER_FAILED,
    NUMBER_IR_PLAYER_ASSIST,
    NUMBER_IR_PLAYER_LIGHTASSIST,
    NUMBER_IR_PLAYER_EASY,
    NUMBER_IR_PLAYER_NORMAL,
    NUMBER_IR_PLAYER_HARD,
    NUMBER_IR_PLAYER_EXHARD,
    NUMBER_IR_PLAYER_FULLCOMBO,
    NUMBER_IR_PLAYER_PERFECT,
    NUMBER_IR_PLAYER_MAX,
];

/// The numbers of the share of the scores on each lamp, in whole percent and in the tenths after
/// them (`IR_CLEAR_RATE`, `IR_CLEAR_RATE_AFTERDOT`).
const CLEAR_RATE_NUMBERS: [i32; CLEAR_LAMPS] = [
    NUMBER_IR_PLAYER_NOPLAY_RATE,
    NUMBER_IR_PLAYER_FAILED_RATE,
    NUMBER_IR_PLAYER_ASSIST_RATE,
    NUMBER_IR_PLAYER_LIGHTASSIST_RATE,
    NUMBER_IR_PLAYER_EASY_RATE,
    NUMBER_IR_PLAYER_NORMAL_RATE,
    NUMBER_IR_PLAYER_HARD_RATE,
    NUMBER_IR_PLAYER_EXHARD_RATE,
    NUMBER_IR_PLAYER_FULLCOMBO_RATE,
    NUMBER_IR_PLAYER_PERFECT_RATE,
    NUMBER_IR_PLAYER_MAX_RATE,
];
const CLEAR_RATE_AFTERDOT_NUMBERS: [i32; CLEAR_LAMPS] = [
    NUMBER_IR_PLAYER_NOPLAY_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_FAILED_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_ASSIST_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_LIGHTASSIST_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_EASY_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_NORMAL_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_HARD_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_EXHARD_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_FULLCOMBO_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_PERFECT_RATE_AFTERDOT,
    NUMBER_IR_PLAYER_MAX_RATE_AFTERDOT,
];

/// The floats of the share of the scores on each lamp (`FloatPropertyPattern.IR_CLEAR_RATE`).
const CLEAR_RATE_FLOATS: [i32; CLEAR_LAMPS] = [
    FLOAT_IR_PLAYER_NOPLAY_RATE,
    FLOAT_IR_PLAYER_FAILED_RATE,
    FLOAT_IR_PLAYER_ASSIST_RATE,
    FLOAT_IR_PLAYER_LIGHTASSIST_RATE,
    FLOAT_IR_PLAYER_EASY_RATE,
    FLOAT_IR_PLAYER_NORMAL_RATE,
    FLOAT_IR_PLAYER_HARD_RATE,
    FLOAT_IR_PLAYER_EXHARD_RATE,
    FLOAT_IR_PLAYER_FULLCOMBO_RATE,
    FLOAT_IR_PLAYER_PERFECT_RATE,
    FLOAT_IR_PLAYER_MAX_RATE,
];

/// The lamps that count as a clear (assist easy and up) and as a full combo (full combo and up).
const CLEARED: std::ops::RangeInclusive<usize> = 2..=10;
const FULL_COMBO_UP: std::ops::RangeInclusive<usize> = 8..=10;

/// The scales a share is read at: whole percent, and a thousandth cut to its last digit.
const PERCENT: i32 = 100;
const PER_MILLE: i32 = 1000;
const TENTHS_MODULUS: i32 = 10;

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct IrState<'a> {
    /// The ranking under the browser's cursor, on the song browser.
    pub browser: Option<IrBrowser>,
    /// How the run stands with the service, when there is a run to stand.
    pub link: Option<IrLink>,
    /// What the result screen holds itself: how many players the ranking has and how far it is
    /// scrolled.
    pub scene: Option<ResultScene>,
    /// The name of the service and of the player on it (`irname`, `irUserName`).
    pub service_name: &'a str,
    pub user_name: &'a str,
    /// The name of the target the run was paced against (`rival`, `target`).
    pub target_name: &'a str,
}

/// The first and last of the numbers of the clear spread the ranking keeps
/// (`IntegerPropertyPattern.IR_CLEAR_*`).
const CLEAR_SPREAD_FIRST: i32 = NUMBER_IR_PLAYER_NOPLAY;
const CLEAR_SPREAD_LAST: i32 = NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT;

/// The id of the one number in the clear spread that the reference leaves without a property.
const UNDEFINED_SPREAD_NUMBER: i32 = 221;

/// The floats of the clear spread: each lamp's share of the players, and the two totals
/// (`FloatPropertyPattern.IR_CLEAR_RATE`, `FloatType.ir_totalclearrate` and `ir_totalfullcomborate`).
const SPREAD_FLOATS: [i32; 13] = [
    FLOAT_IR_PLAYER_NOPLAY_RATE,
    FLOAT_IR_PLAYER_FAILED_RATE,
    FLOAT_IR_PLAYER_ASSIST_RATE,
    FLOAT_IR_PLAYER_LIGHTASSIST_RATE,
    FLOAT_IR_PLAYER_EASY_RATE,
    FLOAT_IR_PLAYER_NORMAL_RATE,
    FLOAT_IR_PLAYER_HARD_RATE,
    FLOAT_IR_PLAYER_EXHARD_RATE,
    FLOAT_IR_PLAYER_FULLCOMBO_RATE,
    FLOAT_IR_PLAYER_PERFECT_RATE,
    FLOAT_IR_PLAYER_MAX_RATE,
    FLOAT_IR_TOTALCLEARRATE,
    FLOAT_IR_TOTALFULLCOMBORATE,
];

impl ClusterState for IrState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        if let Some(browser) = self.browser {
            return browser.boolean(id);
        }
        let link = self.link?;
        match id {
            OPTION_OFFLINE => Some(!link.online),
            OPTION_ONLINE => Some(link.online),
            OPTION_IR_NOPLAYER | OPTION_IR_FAILED | OPTION_IR_WAITING | OPTION_IR_BUSY => Some(false),
            _ => None,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
        if let Some(browser) = self.browser {
            return browser.integer(id);
        }
        let link = self.link?;
        match id {
            NUMBER_IR_RANK => Some(link.ranking_number(link.rank)),
            NUMBER_IR_TOTALPLAYER | NUMBER_IR_TOTALPLAYER2 => Some(link.ranking_number(self.scene.map_or(0, |scene| scene.ranking_total))),
            NUMBER_IR_PREVRANK => Some(link.ranking_number(link.previous_rank)),
            UNDEFINED_SPREAD_NUMBER => None,
            CLEAR_SPREAD_FIRST..=CLEAR_SPREAD_LAST => Some(INTEGER_ABSENT),
            NUMBER_RIVAL_PERFECT..=NUMBER_RIVAL_POOR_RATE => Some(INTEGER_ABSENT),
            NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR => Some(INTEGER_ABSENT),
            _ => None,
        }
    }

    fn rate(&self, id: i32) -> Option<f32> {
        if let Some(browser) = self.browser {
            return browser.rate(id);
        }
        self.link?;
        (id == RATE_RANKING_POSITION)
            .then(|| self.scene.map_or(0, |scene| scene.ranking_offset) as f32 / self.scene.map_or(1, |scene| scene.ranking_total.max(1)) as f32)
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        if self.browser.is_none() {
            self.link?;
        }
        (NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR).contains(&id).then_some(IMAGE_INDEX_ABSENT)
    }

    fn float(&self, id: i32) -> Option<f32> {
        if let Some(browser) = self.browser {
            return browser.float(id);
        }
        self.link?;
        let rival = (FLOAT_RIVAL_PERFECT_RATE..=FLOAT_RIVAL_POOR_RATE).contains(&id);
        (SPREAD_FLOATS.contains(&id) || rival).then_some(FLOAT_ABSENT)
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        if self.browser.is_none() {
            self.link?;
        }
        match id {
            STRING_RIVAL if self.browser.is_some() => Some(Cow::Borrowed("")),
            STRING_RIVAL | STRING_TARGET_NAME => Some(Cow::Borrowed(self.target_name)),
            STRING_RANKING1_NAME..=STRING_RANKING10_NAME => Some(Cow::Borrowed("")),
            STRING_IR_NAME => Some(Cow::Borrowed(self.service_name)),
            STRING_IR_USER_NAME => Some(Cow::Borrowed(self.user_name)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
