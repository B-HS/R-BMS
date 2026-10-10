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
//! A cluster that has no link answers nothing, so the ids fall through to whatever else can.

use std::borrow::Cow;

use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};
use rbms_skin::timer::{TimerId, timer_id};

use super::{ClusterState, ResultScene};
use crate::ir_outcome::IrStatus;

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

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct IrState<'a> {
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

/// The id of the text that names the target (`StringType.target`), which the reference numbers
/// without a constant.
const STRING_TARGET_NAME: i32 = 3;

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
        let link = self.link?;
        match id {
            OPTION_OFFLINE => Some(!link.online),
            OPTION_ONLINE => Some(link.online),
            OPTION_IR_NOPLAYER | OPTION_IR_FAILED | OPTION_IR_WAITING | OPTION_IR_BUSY => Some(false),
            _ => None,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
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
        self.link?;
        (id == RATE_RANKING_POSITION)
            .then(|| self.scene.map_or(0, |scene| scene.ranking_offset) as f32 / self.scene.map_or(1, |scene| scene.ranking_total.max(1)) as f32)
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        self.link?;
        (NUMBER_RANKING1_EXSCORE..=NUMBER_RANKING10_CLEAR).contains(&id).then_some(IMAGE_INDEX_ABSENT)
    }

    fn float(&self, id: i32) -> Option<f32> {
        self.link?;
        let rival = (FLOAT_RIVAL_PERFECT_RATE..=FLOAT_RIVAL_POOR_RATE).contains(&id);
        (SPREAD_FLOATS.contains(&id) || rival).then_some(FLOAT_ABSENT)
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        self.link?;
        match id {
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
