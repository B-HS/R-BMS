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
    /// IR score server base URL (NETWORK tab / `--server`). `None` = offline (NullScoreServer).
    pub server_url: Option<String>,
    /// Player id submitted to the score server (NETWORK tab / `--player`). Defaults to `guest`.
    pub player_id: String,
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
            server_url: None,
            player_id: "guest".into(),
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
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => {
                if let Err(e) = crate::write_atomic(path, &s) {
                    eprintln!("settings write failed ({}): {e}", path.display());
                }
            }
            Err(e) => eprintln!("settings save failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sane() {
        let s = PlaySettings::default();
        assert!((s.hispeed - 2.0).abs() < 1e-9);
        assert_eq!(s.gauge, "NORMAL");
        assert_eq!(s.random, "OFF");
        assert!(s.autoplay);
        assert!(s.bga);
        assert!(s.auto_replay);
        assert_eq!(s.judge_rate, 100);
        assert_eq!(s.offset_ms, 0);
        assert!(s.preview);
        assert_eq!(s.songs_folder, None);
        assert_eq!(s.font_path, None);
        assert_eq!(s.server_url, None);
        assert_eq!(s.player_id, "guest");
    }

    #[test]
    fn ron_round_trip_preserves_every_field() {
        let mut s = PlaySettings::default();
        s.hispeed = 3.25;
        s.gauge = "HARD".into();
        s.lift = 0.15;
        s.cover = 0.4;
        s.scratch_left = true;
        s.scratch_auto = true;
        s.autoplay = false;
        s.random = "MIRROR".into();
        s.constant_speed = true;
        s.offset_ms = -33;
        s.auto_offset = true;
        s.judge_rate = 150;
        s.total_override = 320.0;
        s.bga = false;
        s.skin = "WIDE".into();
        s.auto_replay = false;
        s.debug = true;
        s.font_path = Some("/tmp/f.ttf".into());
        s.score_graph = false;
        s.replay_analysis = false;
        s.preview = false;
        s.songs_folder = Some("/songs".into());
        s.server_url = Some("https://ir.example/api".into());
        s.player_id = "dj".into();
        let txt = ron::ser::to_string_pretty(&s, ron::ser::PrettyConfig::default()).unwrap();
        let back: PlaySettings = ron::from_str(&txt).unwrap();
        assert!((back.hispeed - 3.25).abs() < 1e-9);
        assert_eq!(back.gauge, "HARD");
        assert!((back.lift - 0.15).abs() < 1e-6);
        assert!((back.cover - 0.4).abs() < 1e-6);
        assert!(back.scratch_left && back.scratch_auto);
        assert!(!back.autoplay);
        assert_eq!(back.random, "MIRROR");
        assert!(back.constant_speed);
        assert_eq!(back.offset_ms, -33);
        assert!(back.auto_offset);
        assert_eq!(back.judge_rate, 150);
        assert!((back.total_override - 320.0).abs() < 1e-9);
        assert!(!back.bga);
        assert_eq!(back.skin, "WIDE");
        assert!(!back.auto_replay);
        assert!(back.debug);
        assert_eq!(back.font_path.as_deref(), Some("/tmp/f.ttf"));
        assert!(!back.score_graph && !back.replay_analysis && !back.preview);
        assert_eq!(back.songs_folder.as_deref(), Some("/songs"));
        assert_eq!(back.server_url.as_deref(), Some("https://ir.example/api"));
        assert_eq!(back.player_id, "dj");
    }

    #[test]
    fn partial_ron_keeps_given_fields_and_defaults_the_rest() {
        // serde(default): only the explicitly written fields differ from default; everything
        // else (including fields added later) falls back, giving back-compat.
        let s: PlaySettings = ron::from_str(r#"(hispeed: 7.5, gauge: "EASY")"#).unwrap();
        assert!((s.hispeed - 7.5).abs() < 1e-9, "explicit field kept");
        assert_eq!(s.gauge, "EASY");
        assert_eq!(s.random, "OFF", "missing field defaulted");
        assert!(s.preview, "field added later defaults to true");
        assert_eq!(s.judge_rate, 100);
    }

    #[test]
    fn empty_unit_ron_is_all_defaults() {
        let s: PlaySettings = ron::from_str("()").unwrap();
        let d = PlaySettings::default();
        assert_eq!(s.gauge, d.gauge);
        assert_eq!(s.judge_rate, d.judge_rate);
        assert_eq!(s.preview, d.preview);
    }

    #[test]
    fn load_missing_file_writes_and_returns_defaults() {
        let dir = std::env::temp_dir().join(format!("rbms_settings_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.ron");
        assert!(!path.exists());
        let s = PlaySettings::load(&path);
        assert!(path.exists(), "load() of a missing settings file writes defaults out");
        assert_eq!(s.gauge, "NORMAL");
        // re-loading the freshly written file is stable
        let s2 = PlaySettings::load(&path);
        assert_eq!(s2.judge_rate, 100);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_malformed_file_backs_up_and_returns_defaults() {
        let dir = std::env::temp_dir().join(format!("rbms_settings_bad_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.ron");
        std::fs::write(&path, "definitely not ron )))").unwrap();
        let s = PlaySettings::load(&path);
        assert_eq!(s.gauge, "NORMAL", "malformed file => defaults");
        assert!(path.with_extension("ron.bak").exists(), "malformed file backed up");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
