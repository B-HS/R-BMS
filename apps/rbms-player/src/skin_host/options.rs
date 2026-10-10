//! Property cluster E: the player's settings.
//!
//! Where the values come from: what the player chose: gauge, random and double options, hi-speed
//! and its fix, BGA, assists, long note mode, auto-save slots and the other switches a skin shows
//! as a picked image.
//!
//! On a result screen these are the choices the run was played with, kept in its
//! [`ResultSnapshot`](super::result::snapshot::ResultSnapshot) as [`PlayedOptions`], and the gauge the
//! screen has the graph on; the cluster reads them from there ([`OptionsState::of_result`]). The settings of the browser and the play screen, which
//! read the live configuration, are answered when those screens bring theirs.
//!
//! What this cluster answers on a result screen: image indices 40 (the gauge the graph is on), 42-43
//! (the random options), 54 (the double option), 61-63, 89-90 and 301-308; options 60-62. The
//! indices that show a setting of the configuration screens -- hi-speed, lane cover, judge
//! algorithm and the like (55, 72, 75, 78, 321-324, 330-332, 340-343, 350-353, 360-361, 400) -- and
//! the numbers 10, 12, 57-59 and 310-313, float 310 and rates 17-19 are not answered yet.
//!
//! A value is read as `IntegerPropertyFactory.IndexType` and `BooleanPropertyFactory` read it for a
//! result screen: the random options are the ones the replay recorded, the gauge is the one the
//! graph is switched to, and the options of the target score are absent because the target here is
//! a score and not a play. `no_save_clear` (62) is defined as the opposite of what its name says,
//! the same as `enable_save_score` (61), and is kept so.
//!
//! A cluster that has no snapshot answers nothing, so the ids fall through to whatever else can.

use rbms_chart::shuffle::NoteOption;
use rbms_skin::property::IMAGE_INDEX_ABSENT;
use rbms_skin::property::generated::*;

use super::result::snapshot::FinishedRun;
use super::{ClusterState, INDEX_TARGET_OPTION_FIRST, INDEX_TARGET_OPTION_LAST};

/// What a favourite mark reads as: not marked, marked, and hidden. The player has no way to hide a
/// chart, so only the first two are ever shown (`IndexType.favorite_chart`).
const FAVORITE_NONE: i32 = 0;
const FAVORITE_MARKED: i32 = 1;

/// The random options in the order the reference numbers them, off first (`Random.OPTION_GENERAL`,
/// whose two EX options this player does not have).
const OPTION_GENERAL: [NoteOption; 8] = [
    NoteOption::Off,
    NoteOption::Mirror,
    NoteOption::Random,
    NoteOption::RRandom,
    NoteOption::SRandom,
    NoteOption::Rotate,
    NoteOption::HRandom,
    NoteOption::AllScratch,
];

/// The random option as the reference numbers it: its place in [`OPTION_GENERAL`].
pub fn random_option_index(option: NoteOption) -> i32 {
    OPTION_GENERAL.iter().position(|general| *general == option).and_then(|index| i32::try_from(index).ok()).unwrap_or_default()
}

/// The choices a run was played with, in the numbers the reference shows them as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayedOptions {
    /// The random option of the first player's lanes and of the second's
    /// (`ReplayData.randomoption`, `randomoption2`).
    pub random: i32,
    pub random_2p: i32,
    /// The double option (`ReplayData.doubleoption`).
    pub double_option: i32,
    /// The assists a run shows beside its score: a custom judge window, a constant scroll speed,
    /// the judge area shown, legacy long notes, processed notes marked, the BPM guide and no mines.
    pub custom_judge: bool,
    pub constant_scroll: bool,
    pub judge_area: bool,
    pub legacy_long_note: bool,
    pub mark_note: bool,
    pub bpm_guide: bool,
    pub no_mine: bool,
    /// The long note mode, as the reference numbers it (0 long note, 1 charge note, 2 hell charge).
    pub long_note_mode: i32,
}

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct OptionsState<'a> {
    /// The finished run, when there is one on the screen.
    pub result: Option<FinishedRun<'a>>,
}

impl<'a> OptionsState<'a> {
    /// The choices of the finished run on a result screen.
    pub fn of_result(finished: FinishedRun<'a>) -> OptionsState<'a> {
        OptionsState { result: Some(finished) }
    }
}

/// How a mark that may be unknown is shown: absent while it is not known.
fn favorite_index(mark: Option<bool>) -> i32 {
    mark.map_or(IMAGE_INDEX_ABSENT, |marked| if marked { FAVORITE_MARKED } else { FAVORITE_NONE })
}

impl ClusterState for OptionsState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let run = self.result?.snapshot;
        match id {
            OPTION_DISABLE_SAVE_SCORE => Some(!run.updates_score),
            OPTION_ENABLE_SAVE_SCORE | OPTION_NO_SAVE_CLEAR => Some(run.updates_score),
            _ => None,
        }
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        let finished = self.result?;
        let run = finished.snapshot;
        let options = &run.options;
        Some(match id {
            BUTTON_GAUGE_1P => finished.scene.gauge_type as i32,
            BUTTON_RANDOM_1P => options.random,
            BUTTON_RANDOM_2P => options.random_2p,
            BUTTON_DPOPTION => options.double_option,
            INDEX_TARGET_OPTION_FIRST..=INDEX_TARGET_OPTION_LAST => IMAGE_INDEX_ABSENT,
            BUTTON_FAVORITTE_SONG => favorite_index(run.favorite_song),
            BUTTON_FAVORITTE_CHART => favorite_index(run.favorite_chart),
            BUTTON_ASSIST_EXJUDGE => i32::from(options.custom_judge),
            BUTTON_ASSIST_CONSTANT => i32::from(options.constant_scroll),
            BUTTON_ASSIST_JUDGEAREA => i32::from(options.judge_area),
            BUTTON_ASSIST_LEGACY => i32::from(options.legacy_long_note),
            BUTTON_ASSIST_MARKNOTE => i32::from(options.mark_note),
            BUTTON_ASSIST_BPMGUIDE => i32::from(options.bpm_guide),
            BUTTON_ASSIST_NOMINE => i32::from(options.no_mine),
            BUTTON_LNMODE => options.long_note_mode,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests;
