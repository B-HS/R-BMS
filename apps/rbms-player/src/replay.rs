use std::path::Path;

use serde::{Deserialize, Serialize};

/// One recorded input: a press or release of `lane` at song time `t` (µs, raw — no offset).
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ReplayEvent {
    pub t: i64,
    pub lane: usize,
    pub press: bool,
}

/// A recorded interactive play: the chart it was played on, the exact note-shuffle that was
/// used (random + seed → identical lane layout on playback), the judge offset that was active,
/// and the timestamped input stream.
#[derive(Clone, Serialize, Deserialize)]
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
    pub fn load(path: &Path) -> Result<Replay, String> {
        let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        ron::from_str(&s).map_err(|e| e.to_string())
    }

    pub fn save(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("replay dir create failed ({}): {e}", dir.display());
            }
        }
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => match std::fs::write(path, &s) {
                Ok(_) => println!("replay saved: {}", path.display()),
                Err(e) => eprintln!("replay write failed ({}): {e}", path.display()),
            },
            Err(e) => eprintln!("replay save failed: {e}"),
        }
    }
}
