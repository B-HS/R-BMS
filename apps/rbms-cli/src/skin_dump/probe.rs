//! Finding the smallest Lua budgets a skin runs inside.
//!
//! The interpreter counts instructions in blocks, and a skin that outruns its allowance is cut off.
//! The count is not exposed, so it is measured the only way left: the skin is loaded again under a
//! smaller allowance each time, and the smallest one it still loads and draws under is the answer.
//! The search is a bisection that stops within a twentieth of the answer, and an allowance below
//! the interpreter's counting block (a thousand instructions) passes for any skin that never runs a
//! block, so a result near zero reads as "under one block".

use std::path::Path;

use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, is_lua_skin};
use rbms_skin::lua::{FrameBudget, LoadBudget, LuaBudget};
use rbms_skin::property::MapHost;

use super::frames;
use super::report::ProbeReport;
use super::scenario::Scenario;

/// Frames a call allowance is checked over.
pub const PROBE_FRAMES: usize = 50;

/// The search stops once the range is under this fraction of the answer: one over this number.
const TOLERANCE_DIVISOR: u64 = 20;

/// The smallest `x` in `low..=high` that `passes`, to within a twentieth, assuming that whatever
/// passes at `x` passes at every larger value. `None` when not even `high` passes.
pub fn smallest(low: u64, high: u64, mut passes: impl FnMut(u64) -> bool) -> Option<u64> {
    if !passes(high) {
        return None;
    }
    if passes(low) {
        return Some(low);
    }
    let (mut failing, mut passing) = (low, high);
    while passing - failing > passing / TOLERANCE_DIVISOR + 1 {
        let middle = failing + (passing - failing) / 2;
        if passes(middle) {
            passing = middle;
        } else {
            failing = middle;
        }
    }
    Some(passing)
}

fn load_under(entry: &Path, options: SkinLoadOptions<'_>, scenario: &Scenario, budget: LuaBudget) -> Option<(LoadedSkin, MapHost)> {
    let host = scenario.host().ok()?;
    let skin = load_lua_skin(entry, &LuaSkinOptions { load: options, budget }, &host).ok()?;
    Some((skin, host))
}

fn swallowed(skin: &LoadedSkin) -> u64 {
    skin.runtime().map_or(0, |runtime| runtime.diagnostics().swallowed.iter().map(|error| error.count).sum())
}

/// Measures the instruction allowance of one load pass and of one call of a frame.
pub fn run(entry: &Path, options: SkinLoadOptions<'_>, scenario: &Scenario) -> ProbeReport {
    let defaults = LuaBudget::default();
    let mut report = ProbeReport {
        load_instructions: None,
        load_instructions_limit: defaults.load.max_instructions,
        call_instructions: None,
        call_instructions_limit: defaults.frame.max_call_instructions,
        probe_frames: PROBE_FRAMES,
        note: None,
    };
    if !is_lua_skin(entry) {
        report.note = Some("only a Lua skin has an instruction allowance".to_owned());
        return report;
    }
    let Some((reference, _)) = load_under(entry, options, scenario, defaults) else {
        report.note = Some("the skin does not load under the default budget".to_owned());
        return report;
    };
    let (destinations, caught) = (reference.def.destination.len(), swallowed(&reference));
    drop(reference);

    report.load_instructions = smallest(0, defaults.load.max_instructions, |instructions| {
        let budget = LuaBudget { load: LoadBudget { max_instructions: instructions, max_micros: u64::MAX }, ..defaults };
        load_under(entry, options, scenario, budget).is_some_and(|(skin, _)| skin.def.destination.len() == destinations && swallowed(&skin) == caught)
    });

    report.call_instructions = smallest(0, defaults.frame.max_call_instructions, |instructions| {
        let budget = LuaBudget { frame: FrameBudget { max_call_instructions: instructions, ..defaults.frame }, ..defaults };
        load_under(entry, options, scenario, budget)
            .is_some_and(|(skin, mut host)| skin.runtime().is_some_and(|runtime| frames::count_over_budget(runtime, &mut host, PROBE_FRAMES) == Some(0)))
    });
    if report.call_instructions.is_none() {
        report.note = Some("frames already run over budget under the default allowance".to_owned());
    }
    report
}
