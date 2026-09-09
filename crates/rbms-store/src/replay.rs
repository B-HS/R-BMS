use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::StoreError;

/// One recorded input: a press or release of `lane` at song time `t` (µs, raw — no offset).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ReplayEvent {
    pub t: i64,
    pub lane: usize,
    pub press: bool,
}

/// A recorded interactive play: the chart it was played on, the exact note-shuffle that was
/// used (random + seed → identical lane layout on playback), the judge offset that was active,
/// and the timestamped input stream.
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
