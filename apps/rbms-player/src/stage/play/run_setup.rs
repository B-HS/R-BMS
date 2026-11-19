//! What a run was started with and from, as the play screen's skin shows it beside the run
//! ([`RunSetup`]): the folder of the song browser it was picked in, the arrangement each side's
//! lanes were laid out with, and the assists it is played with.
//!
//! Everything is read off the run, not off the settings alone: the session knows whether SCRATCH
//! AUTO plays a lane, which LN MODE and judge widths it judges by and how many sides the chart has,
//! and the caller hands the arrangement the run was laid out with, which a borrowed layout, a
//! watched replay or a practice slice decide. CONSTANT and LEGACY NOTE change only what the note
//! field shows and are taken from the settings, which is where the field takes them from.
//!
//! The sample run the skin menu and the play options draw is the exception ([`settings_setup`]):
//! it is played with none of the player's assists and laid out with no arrangement, and is shown
//! with the ones the settings hold, as every other option figure beside it is.

use std::sync::Arc;

use rbms_chart::shuffle::NoteOption;
use rbms_config::{Config, LN_MODE_LABELS};
use rbms_judge::ln::LnMode;
use rbms_model::Mode;
use rbms_play::PlaySession;

use crate::AppShared;
use crate::app_play::scratch_auto_lanes;
use crate::judge_setup::{is_custom_judge, judge_setup_of};
use crate::skin_host::play::{ARRANGED_SIDES, RunSetup};

/// How many of the folders open in the song browser name where a run was started: the innermost
/// and the one round it, which for a level of a difficulty table are the table and the level.
const ORIGIN_DEPTH: usize = 2;

/// What the titles of those folders are joined by.
const ORIGIN_SEPARATOR: &str = " / ";

/// What the names of the assists are joined by.
const ASSIST_SEPARATOR: &str = " · ";

/// The names the option panel gives the assists a run can be played with.
const SCRATCH_AUTO: &str = "SCRATCH AUTO";
const EX_JUDGE: &str = "EX JUDGE";
const CONSTANT: &str = "CONSTANT";
const LEGACY_NOTE: &str = "LEGACY NOTE";

/// Where a run starting now was picked: the last [`ORIGIN_DEPTH`] titles of the folders open in the
/// song browser, outermost first, joined by [`ORIGIN_SEPARATOR`]. Nothing for a run the browser did
/// not pick from a folder: a course stage, a replay being watched, and a run started with no folder
/// open (the top of the browser, or a chart named on the command line before the browser was ever
/// opened).
fn origin_of(shared: &AppShared) -> Arc<str> {
    if shared.course_run.is_some() || shared.replay.is_some() {
        return Arc::from("");
    }
    let open = shared.open_folders();
    let innermost = open.len().saturating_sub(ORIGIN_DEPTH);
    Arc::from(open.get(innermost..).unwrap_or_default().join(ORIGIN_SEPARATOR))
}

/// The assists a run is played with as [`assists_of`] names them: whether SCRATCH AUTO plays a lane
/// of it, the LN MODE it is judged by, and whether it is judged wider than its chart asks.
struct Assisted {
    scratch_auto: bool,
    ln_mode: LnMode,
    ex_judge: bool,
}

impl Assisted {
    /// The assists `session` is played with.
    fn of_session(session: &PlaySession) -> Assisted {
        Assisted { scratch_auto: session.auto_lanes().contains(&true), ln_mode: session.ln_mode(), ex_judge: is_custom_judge(&session.judge_setup()) }
    }

    /// The assists the settings hold for a run of `mode` started now: SCRATCH AUTO only where the
    /// mode has a turntable for it to play.
    fn of_settings(config: &Config, mode: Mode) -> Assisted {
        Assisted {
            scratch_auto: scratch_auto_lanes(mode, config.play.scratch_auto).contains(&true),
            ln_mode: config.judge.ln_mode,
            ex_judge: is_custom_judge(&judge_setup_of(config)),
        }
    }
}

/// The assists `assisted` names, in the order SCRATCH AUTO, the LN MODE, EX JUDGE, CONSTANT and
/// LEGACY NOTE, each only when it is on. The LN MODE is named only where it is not LN and
/// `ln_mode_decides` -- the chart leaves its long notes to it -- since it changes nothing of a chart
/// that states its own.
fn assists_of(shared: &AppShared, assisted: &Assisted, ln_mode_decides: bool) -> Arc<str> {
    let play = &shared.config.play;
    let ln_mode = assisted.ln_mode;
    let ln_mode_name = (ln_mode_decides && ln_mode != LnMode::LongNote).then(|| LN_MODE_LABELS.get(usize::from(ln_mode.id())).copied()).flatten();
    let named = [
        assisted.scratch_auto.then_some(SCRATCH_AUTO),
        ln_mode_name,
        assisted.ex_judge.then_some(EX_JUDGE),
        play.constant_speed.then_some(CONSTANT),
        play.legacy_note.then_some(LEGACY_NOTE),
    ];
    Arc::from(named.into_iter().flatten().collect::<Vec<_>>().join(ASSIST_SEPARATOR))
}

/// What a run of `mode` laid out with `arrangement` and played with `assisted` was started with and
/// from: the folder the browser has open, the arrangement on each side the mode has, and its assists.
fn setup_of(shared: &AppShared, mode: Mode, arrangement: NoteOption, assisted: &Assisted, ln_mode_decides: bool) -> RunSetup {
    let sides = usize::from(mode.player).clamp(1, ARRANGED_SIDES);
    let arrangements = std::array::from_fn(|side| if side < sides { arrangement.label() } else { "" });
    RunSetup { origin: origin_of(shared), arrangements, assists: assists_of(shared, assisted, ln_mode_decides) }
}

/// What the run of `session` was started with and from: the folder the browser has open, the
/// arrangement `arrangement` the run laid its lanes out with, on each side the chart has, and the
/// assists it is played with.
pub(crate) fn run_setup(shared: &AppShared, session: &PlaySession, arrangement: NoteOption, ln_mode_decides: bool) -> RunSetup {
    setup_of(shared, session.model().mode, arrangement, &Assisted::of_session(session), ln_mode_decides)
}

/// What a run of `mode` started now would be started with and from, by the settings: the folder the
/// browser has open, the arrangement and the assists the settings hold. This is what a sample run
/// is shown with, which is played with none of the player's assists and laid out with no
/// arrangement, while the option figures beside it are the settings' too.
pub(crate) fn settings_setup(shared: &AppShared, mode: Mode, ln_mode_decides: bool) -> RunSetup {
    setup_of(shared, mode, shared.config.play.random, &Assisted::of_settings(&shared.config, mode), ln_mode_decides)
}

#[cfg(test)]
mod tests;
