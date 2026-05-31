use std::path::Path;

use serde::{Deserialize, Serialize};

/// Persisted play options (everything in the Settings screen that should survive a restart).
/// Enums are stored as strings so the file stays human-editable; the app maps them to/from its
/// own types. Saved next to the key config under `~/.config/rbms/`.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaySettings {
    pub hispeed: f64,
    pub gauge: String,
    pub lift: f32,
    pub cover: f32,
    pub scratch_left: bool,
    pub scratch_auto: bool,
    pub autoplay: bool,
    pub random: String,
    pub constant_speed: bool,
    pub offset_ms: i32,
    pub auto_offset: bool,
    pub judge_rate: i32,
    pub total_override: f64,
    pub bga: bool,
    pub skin: String,
    pub auto_replay: bool,
    pub debug: bool,
    pub font_path: Option<String>,
    pub score_graph: bool,
    pub replay_analysis: bool,
    /// Play the focused song's `#PREVIEW` clip on the select screen.
    pub preview: bool,
    /// Last song folder opened (via launch arg or the in-app picker), restored on the next launch.
    pub songs_folder: Option<String>,
}

impl Default for PlaySettings {
    fn default() -> Self {
        PlaySettings {
            hispeed: 2.0,
            gauge: "NORMAL".into(),
            lift: 0.0,
            cover: 0.0,
            scratch_left: false,
            scratch_auto: false,
            autoplay: true,
            random: "OFF".into(),
            constant_speed: false,
            offset_ms: 0,
            auto_offset: false,
            judge_rate: 100,
            total_override: 0.0,
            bga: true,
            skin: "NORMAL".into(),
            auto_replay: true,
            debug: false,
            font_path: None,
            score_graph: true,
            replay_analysis: true,
            preview: true,
            songs_folder: None,
        }
    }
}

impl PlaySettings {
    pub fn load(path: &Path) -> PlaySettings {
        match std::fs::read_to_string(path) {
            Ok(s) => ron::from_str(&s).unwrap_or_else(|e| {
                let backup = path.with_extension("ron.bak");
                let _ = std::fs::rename(path, &backup);
                eprintln!("settings parse failed ({e}); backed up to {} and using defaults", backup.display());
                PlaySettings::default()
            }),
            Err(_) => {
                let s = PlaySettings::default();
                s.save(path);
                s
            }
        }
    }

    pub fn save(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("settings dir create failed ({}): {e}", dir.display());
            }
        }
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => {
                if let Err(e) = std::fs::write(path, &s) {
                    eprintln!("settings write failed ({}): {e}", path.display());
                }
            }
            Err(e) => eprintln!("settings save failed: {e}"),
        }
    }
}
