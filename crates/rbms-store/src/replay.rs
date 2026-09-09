use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::StoreError;

/// One recorded input: a press or release of `lane` at song time `t` (µs, raw — no offset).
///
/// `backward` is the direction a scratch lane was spun. A scratch lane has two keys and the
/// reference remembers which one grabbed a long note so the other one ends it
/// (`JudgeManager.java:358-372, 435, 449`), so a replay that dropped the direction could not
/// reproduce a back-spin. Key lanes only ever spin forwards, and a replay recorded before the
/// direction was written down reads as forward.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayEvent {
    pub t: i64,
    pub lane: usize,
    pub press: bool,
    pub backward: bool,
}

/// How many judge tiers a JUDGE WIDTH percentage covers: PGREAT, GREAT and GOOD. Pinned against
/// `rbms_play::JUDGE_WIDTH_TIER_COUNT` by the player rather than depending on the play crate here.
pub const REPLAY_JUDGE_WIDTH_TIER_COUNT: usize = 3;

/// JUDGE WIDTH percentage that leaves a mode's own windows alone, which is what a replay recorded
/// before the widths were written down was played at.
pub const REPLAY_UNMODIFIED_RATE_PERCENT: i32 = 100;

/// Judge algorithm a replay recorded before the algorithm was written down was played under: the
/// only one this engine had.
pub const REPLAY_LEGACY_ALGORITHM: &str = "Duration";

/// Long-note flavour a replay recorded before LN MODE was written down was played under: plain long
/// notes, which is what an unresolved chart note used to judge as.
pub const REPLAY_LEGACY_LN_MODE: &str = "LN";

/// Gauge table a replay recorded before the table was written down was played on: the one the
/// chart's own mode selects.
pub const REPLAY_LEGACY_GAUGE_SET: &str = "AUTO";

/// Gauge auto-shift a replay recorded before it was written down was played under: none.
pub const REPLAY_LEGACY_GAUGE_AUTO_SHIFT: &str = "NONE";

/// Auto-shift floor a replay recorded before it was written down was played under.
pub const REPLAY_LEGACY_BOTTOM_SHIFTABLE_GAUGE: &str = "assist";

/// The JUDGE settings a run was played under, recorded so playback reproduces the judgements the
/// run actually got rather than the ones the current settings would give.
///
/// A replay written before these were recorded reads as the engine's pre-parity behaviour: the
/// `Duration` algorithm at every window's stock width.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayJudge {
    /// The reference implementation's `JudgeAlgorithm` constant name.
    pub algorithm: String,
    /// `[PGREAT, GREAT, GOOD]` JUDGE WIDTH percentages for key lanes.
    pub judge_rate_key: [i32; REPLAY_JUDGE_WIDTH_TIER_COUNT],
    /// The same three percentages for scratch lanes.
    pub judge_rate_scratch: [i32; REPLAY_JUDGE_WIDTH_TIER_COUNT],
    /// LN MARGIN as a percentage of the mode's own long-note release window.
    pub longnote_margin_rate: i32,
    /// The flavour long notes the chart left unstated were played as. It decides how many judged
    /// objects the chart has, so a run recorded on one flavour and replayed on another does not
    /// even share an EX denominator (`BMSPlayer.java:893` records it for the same reason).
    pub ln_mode: String,
    /// The gauge table the nine gauges were built from, or the AUTO token for the one the chart's
    /// mode selects. It decides the damage every judgment costs.
    pub gauge_set: String,
    /// How the selected gauge was allowed to move during the run.
    pub gauge_auto_shift: String,
    /// The floor the per-frame auto-shift could drop the selection to.
    pub bottom_shiftable_gauge: String,
}

impl Default for ReplayJudge {
    fn default() -> Self {
        ReplayJudge {
            algorithm: REPLAY_LEGACY_ALGORITHM.to_string(),
            judge_rate_key: [REPLAY_UNMODIFIED_RATE_PERCENT; REPLAY_JUDGE_WIDTH_TIER_COUNT],
            judge_rate_scratch: [REPLAY_UNMODIFIED_RATE_PERCENT; REPLAY_JUDGE_WIDTH_TIER_COUNT],
            longnote_margin_rate: REPLAY_UNMODIFIED_RATE_PERCENT,
            ln_mode: REPLAY_LEGACY_LN_MODE.to_string(),
            gauge_set: REPLAY_LEGACY_GAUGE_SET.to_string(),
            gauge_auto_shift: REPLAY_LEGACY_GAUGE_AUTO_SHIFT.to_string(),
            bottom_shiftable_gauge: REPLAY_LEGACY_BOTTOM_SHIFTABLE_GAUGE.to_string(),
        }
    }
}

/// A recorded interactive play: the chart it was played on, the exact note-shuffle that was
/// used (random + seed → identical lane layout on playback), the judge offset and JUDGE settings
/// that were active, and the timestamped input stream.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Replay {
    pub chart_path: String,
    pub md5: String,
    pub mode: String,
    pub random: String,
    pub seed: u64,
    pub offset_ms: i32,
    #[serde(default)]
    pub scratch_auto: bool,
    #[serde(default)]
    pub gauge: String,
    /// The JUDGE settings the run was judged under.
    #[serde(default)]
    pub judge: ReplayJudge,
    pub events: Vec<ReplayEvent>,
}

impl Replay {
    pub fn load(path: &Path) -> Result<Replay, StoreError> {
        let s = std::fs::read_to_string(path).map_err(StoreError::Read)?;
        Ok(ron::from_str(&s)?)
    }

    /// Serialize and durably replace `path`, reporting why on failure.
    pub fn try_save(&self, path: &Path) -> Result<(), StoreError> {
        let s = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        write_atomic(path, &s).map_err(StoreError::Write)
    }

    /// [`Replay::try_save`] for the fire-and-forget call site on the result screen: success and
    /// failure are both reported on the console and neither interrupts play.
    pub fn save(&self, path: &Path) {
        match self.try_save(path) {
            Ok(()) => println!("replay saved: {}", path.display()),
            Err(StoreError::Write(e)) => eprintln!("replay write failed ({}): {e}", path.display()),
            Err(e) => eprintln!("replay save failed: {e}"),
        }
    }
}
