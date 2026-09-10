//! The three play-option vocabularies the PLAY tab gained for the option overlay: which tempo the
//! green number is pinned to, what is done to the lanes as a whole, and how leaving a run mid-song
//! is confirmed.
//!
//! Each one is a closed list stored as a separator-free token, so the settings file survives a
//! reworded label and a renamed variant.

use serde::{Deserialize, Deserializer, Serializer};

/// Which tempo a fixed green number is computed against.
///
/// Mirrors the reference implementation's `fixhispeed` (`PlayConfig.java:43-49`) value for value,
/// including its shipped default of the chart's main tempo. `Off` leaves the scroll speed to the
/// HI-SPEED row alone, which is what this engine did before the row existed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FixHiSpeed {
    Off,
    StartBpm,
    MaxBpm,
    #[default]
    MainBpm,
    MinBpm,
}

/// How many tempos a fixed green number can be pinned to, `Off` included.
pub const FIX_HISPEED_COUNT: usize = 5;

impl FixHiSpeed {
    /// Every choice, in the order the row steps through them, which is also the reference
    /// implementation's own numbering.
    pub const ALL: [FixHiSpeed; FIX_HISPEED_COUNT] = [FixHiSpeed::Off, FixHiSpeed::StartBpm, FixHiSpeed::MaxBpm, FixHiSpeed::MainBpm, FixHiSpeed::MinBpm];

    /// Name the row shows.
    pub fn label(self) -> &'static str {
        match self {
            FixHiSpeed::Off => "OFF",
            FixHiSpeed::StartBpm => "START",
            FixHiSpeed::MaxBpm => "MAX",
            FixHiSpeed::MainBpm => "MAIN",
            FixHiSpeed::MinBpm => "MIN",
        }
    }

    /// Token the choice is stored under.
    pub fn token(self) -> &'static str {
        match self {
            FixHiSpeed::Off => "OFF",
            FixHiSpeed::StartBpm => "STARTBPM",
            FixHiSpeed::MaxBpm => "MAXBPM",
            FixHiSpeed::MainBpm => "MAINBPM",
            FixHiSpeed::MinBpm => "MINBPM",
        }
    }

    /// The choice a stored token names, falling back to the shipped default.
    pub fn from_token(token: &str) -> FixHiSpeed {
        FixHiSpeed::ALL.into_iter().find(|entry| entry.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }
}

/// What is done to the lanes as a whole, as opposed to the note shuffle inside them.
///
/// The values are the reference implementation's `doubleoption` 0..3 in its own order
/// (`BMSPlayer.java:238-297`), so a recorded option number means the same thing in both. The
/// symmetric and synchronised shuffles are not on this axis in the reference either: they belong to
/// the note-shuffle option.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LaneOption {
    #[default]
    Off,
    /// Swap the two sides of a chart that already has them.
    Flip,
    /// Deal one side's chart to both sides.
    Battle,
    /// Battle with the scratch lane played for you.
    BattleAutoScratch,
}

/// How many lane options there are.
pub const LANE_OPTION_COUNT: usize = 4;

impl LaneOption {
    /// Every option, in the reference implementation's own numbering.
    pub const ALL: [LaneOption; LANE_OPTION_COUNT] = [LaneOption::Off, LaneOption::Flip, LaneOption::Battle, LaneOption::BattleAutoScratch];

    /// Name the row shows.
    pub fn label(self) -> &'static str {
        match self {
            LaneOption::Off => "OFF",
            LaneOption::Flip => "FLIP",
            LaneOption::Battle => "BATTLE",
            LaneOption::BattleAutoScratch => "BATTLE AUTO-SC",
        }
    }

    /// Token the option is stored under.
    pub fn token(self) -> &'static str {
        match self {
            LaneOption::Off => "OFF",
            LaneOption::Flip => "FLIP",
            LaneOption::Battle => "BATTLE",
            LaneOption::BattleAutoScratch => "BATTLEAUTOSCRATCH",
        }
    }

    /// The number this option is recorded as, which is the reference implementation's own.
    pub fn recorded_value(self) -> u8 {
        match self {
            LaneOption::Off => 0,
            LaneOption::Flip => 1,
            LaneOption::Battle => 2,
            LaneOption::BattleAutoScratch => 3,
        }
    }

    /// Whether the option plays a lane for the player, which is what makes a run assisted.
    /// Battle and battle-with-auto-scratch do (`BMSPlayer.java:252-253`); flip does not.
    pub fn is_assist(self) -> bool {
        matches!(self, LaneOption::Battle | LaneOption::BattleAutoScratch)
    }

    /// The option a stored token names, falling back to none.
    pub fn from_token(token: &str) -> LaneOption {
        LaneOption::ALL.into_iter().find(|entry| entry.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }
}

/// What an Escape during a run has to be to abandon it.
///
/// The shipped value leaves the key immediate, which is what it has always been; the other two are
/// there for a player who keeps hitting it by accident.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PlayEscape {
    #[default]
    Immediate,
    /// Held down for [`PLAY_ESCAPE_HOLD_MS`] before the run is abandoned.
    Hold,
    /// Pressed twice within [`PLAY_ESCAPE_DOUBLE_MS`].
    Double,
}

/// How many ways of leaving a run there are.
pub const PLAY_ESCAPE_COUNT: usize = 3;

/// How long Escape must be held down to abandon a run in [`PlayEscape::Hold`].
pub const PLAY_ESCAPE_HOLD_MS: u64 = 600;

/// How long a second Escape still counts as the other half of a pair in [`PlayEscape::Double`].
pub const PLAY_ESCAPE_DOUBLE_MS: u64 = 500;

impl PlayEscape {
    /// Every choice, in the order the row steps through them.
    pub const ALL: [PlayEscape; PLAY_ESCAPE_COUNT] = [PlayEscape::Immediate, PlayEscape::Hold, PlayEscape::Double];

    /// Name the row shows.
    pub fn label(self) -> &'static str {
        match self {
            PlayEscape::Immediate => "IMMEDIATE",
            PlayEscape::Hold => "HOLD",
            PlayEscape::Double => "DOUBLE TAP",
        }
    }

    /// Token the choice is stored under.
    pub fn token(self) -> &'static str {
        match self {
            PlayEscape::Immediate => "IMMEDIATE",
            PlayEscape::Hold => "HOLD",
            PlayEscape::Double => "DOUBLE",
        }
    }

    /// The choice a stored token names, falling back to the immediate one.
    pub fn from_token(token: &str) -> PlayEscape {
        PlayEscape::ALL.into_iter().find(|entry| entry.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }
}

/// Stores [`FixHiSpeed`] as its token.
pub mod fix_hispeed_token {
    use super::{Deserialize, Deserializer, FixHiSpeed, Serializer};

    /// Writes the token.
    pub fn serialize<S: Serializer>(value: &FixHiSpeed, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    /// Reads the token back.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<FixHiSpeed, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(FixHiSpeed::from_token(&token))
    }
}

/// Stores [`LaneOption`] as its token.
pub mod lane_option_token {
    use super::{Deserialize, Deserializer, LaneOption, Serializer};

    /// Writes the token.
    pub fn serialize<S: Serializer>(value: &LaneOption, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    /// Reads the token back.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<LaneOption, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(LaneOption::from_token(&token))
    }
}

/// Stores [`PlayEscape`] as its token.
pub mod play_escape_token {
    use super::{Deserialize, Deserializer, PlayEscape, Serializer};

    /// Writes the token.
    pub fn serialize<S: Serializer>(value: &PlayEscape, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    /// Reads the token back.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PlayEscape, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(PlayEscape::from_token(&token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_has_its_own_label_and_token_and_reads_back() {
        for (labels, tokens) in [
            (FixHiSpeed::ALL.map(FixHiSpeed::label).to_vec(), FixHiSpeed::ALL.map(FixHiSpeed::token).to_vec()),
            (LaneOption::ALL.map(LaneOption::label).to_vec(), LaneOption::ALL.map(LaneOption::token).to_vec()),
            (PlayEscape::ALL.map(PlayEscape::label).to_vec(), PlayEscape::ALL.map(PlayEscape::token).to_vec()),
        ] {
            let count = labels.len();
            let mut labels = labels;
            let mut tokens = tokens;
            labels.sort_unstable();
            labels.dedup();
            tokens.sort_unstable();
            tokens.dedup();
            assert_eq!(labels.len(), count, "two choices share a label");
            assert_eq!(tokens.len(), count, "two choices share a token");
        }
        for entry in FixHiSpeed::ALL {
            assert_eq!(FixHiSpeed::from_token(entry.token()), entry);
        }
        for entry in LaneOption::ALL {
            assert_eq!(LaneOption::from_token(entry.token()), entry);
        }
        for entry in PlayEscape::ALL {
            assert_eq!(PlayEscape::from_token(entry.token()), entry);
        }
    }

    #[test]
    fn an_unknown_token_falls_back_to_the_shipped_choice() {
        assert_eq!(FixHiSpeed::from_token("nonsense"), FixHiSpeed::MainBpm, "PlayConfig.java:43-49 ships MAINBPM");
        assert_eq!(LaneOption::from_token("nonsense"), LaneOption::Off);
        assert_eq!(PlayEscape::from_token("nonsense"), PlayEscape::Immediate);
    }

    #[test]
    fn the_lane_options_keep_the_numbering_a_recorded_run_carries() {
        for (at, option) in LaneOption::ALL.into_iter().enumerate() {
            assert_eq!(option.recorded_value() as usize, at, "{option:?} is recorded under the wrong number");
        }
    }

    /// The reference implementation raises the assist level for battle and leaves flip alone
    /// (`BMSPlayer.java:252-253` against `:294-295`), so the lamp demotion and the submission gate
    /// have to agree with it.
    #[test]
    fn only_the_options_that_play_a_lane_for_you_count_as_an_assist() {
        assert!(!LaneOption::Off.is_assist());
        assert!(!LaneOption::Flip.is_assist(), "flip only mirrors a chart that already has both sides");
        assert!(LaneOption::Battle.is_assist());
        assert!(LaneOption::BattleAutoScratch.is_assist());
    }
}
