use std::path::Path;

use rbms_audio::{AudioOptions, DEFAULT_MAX_VOICES};
use serde::{Deserialize, Serialize};

/// Master output gain a fresh install starts at. The per-bus defaults carry the loudness split, so
/// the master itself stays transparent.
pub const DEFAULT_MASTER_VOLUME: f32 = 1.0;

/// Per-bus (key / bgm / system) output gain a fresh install starts at.
pub const DEFAULT_BUS_VOLUME: f32 = 0.5;

/// Percent one left/right step moves a volume row.
pub const AUDIO_VOLUME_STEP_PERCENT: i32 = 5;

/// Loudest a volume row goes.
pub const AUDIO_VOLUME_MAX_PERCENT: i32 = 100;

/// Output buffer sizes in frames the BUFFER SIZE row offers after AUTO.
pub const AUDIO_BUFFER_FRAMES_CHOICES: [u32; 8] = [128, 192, 256, 384, 512, 768, 1024, 2048];

/// Output rates in Hz the SAMPLE RATE row offers after AUTO.
pub const AUDIO_SAMPLE_RATE_HZ_CHOICES: [u32; 4] = [44_100, 48_000, 88_200, 96_000];

/// Fewest simultaneous voices the POLYPHONY row allows.
pub const AUDIO_POLYPHONY_MIN_VOICES: usize = 64;

/// Most simultaneous voices the POLYPHONY row allows.
pub const AUDIO_POLYPHONY_MAX_VOICES: usize = 1024;

/// Voices one left/right step moves the POLYPHONY row.
pub const AUDIO_POLYPHONY_STEP_VOICES: usize = 64;

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
    /// Bearer token from the last successful IR login/register. `None` = signed out (guest).
    /// Cleared by LOGOUT. The password that produced it is never stored.
    pub ir_token: Option<String>,
    /// Login id the stored token belongs to, shown in the ACCOUNT row.
    pub ir_login_id: Option<String>,
    /// Address kept only to prefill the REGISTER form; display-only, never used to authenticate.
    pub ir_email: Option<String>,
    /// Mirror the local settings + key config to the account with the SYNC SETTINGS actions.
    pub sync_settings: bool,
    /// Upload the saved replay of a ranked submission right after the score lands.
    pub auto_upload_replay: bool,
    /// Cached rival player ids, refreshed from the server on login and after a rival edit.
    pub rivals: Vec<String>,
    /// Output device name the stream is opened on. `None` = the system default device.
    pub audio_device: Option<String>,
    /// Requested output buffer size in frames. `None` = whatever the backend picks.
    pub audio_buffer_frames: Option<u32>,
    /// Requested output rate in Hz. `None` = the device's own default rate.
    pub audio_sample_rate: Option<u32>,
    /// Voices the mixer may sound at once.
    pub audio_polyphony: usize,
    /// Master output gain applied after the buses are summed.
    pub vol_master: f32,
    /// Gain of the key bus (note keysounds).
    pub vol_key: f32,
    /// Gain of the bgm bus (auto-played channels and song previews).
    pub vol_bg: f32,
    /// Gain of the system bus (interface sounds).
    pub vol_system: f32,
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
            player_id: rbms_ir::GUEST_PLAYER_ID.into(),
            ir_token: None,
            ir_login_id: None,
            ir_email: None,
            sync_settings: false,
            auto_upload_replay: true,
            rivals: Vec::new(),
            audio_device: None,
            audio_buffer_frames: None,
            audio_sample_rate: None,
            audio_polyphony: DEFAULT_MAX_VOICES,
            vol_master: DEFAULT_MASTER_VOLUME,
            vol_key: DEFAULT_BUS_VOLUME,
            vol_bg: DEFAULT_BUS_VOLUME,
            vol_system: DEFAULT_BUS_VOLUME,
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

/// A stored gain clamped to the range the volume rows can produce, falling back to `fallback` when
/// a hand-edited file holds something that is not a number.
pub fn clamp_volume(gain: f32, fallback: f32) -> f32 {
    if gain.is_nan() { fallback } else { gain.clamp(0.0, 1.0) }
}

/// A gain as the whole percent its row shows.
pub fn volume_percent(gain: f32) -> i32 {
    let max = AUDIO_VOLUME_MAX_PERCENT as f32;
    (gain * max).round().clamp(0.0, max) as i32
}

/// One left/right step on a volume row, as a gain. Stepping goes through whole percent so a row
/// held down never drifts off the values it displays.
pub fn step_volume(gain: f32, d: i32) -> f32 {
    let percent = (volume_percent(gain) + d * AUDIO_VOLUME_STEP_PERCENT).clamp(0, AUDIO_VOLUME_MAX_PERCENT);
    percent as f32 / AUDIO_VOLUME_MAX_PERCENT as f32
}

/// One step through `AUTO` (`None`) followed by `choices`. A stored value that is not in `choices`
/// (a hand-edited file) steps from the `AUTO` position.
pub fn cycle_optional_u32(current: Option<u32>, choices: &[u32], d: i32) -> Option<u32> {
    let len = choices.len() as i32 + 1;
    let at = current.and_then(|v| choices.iter().position(|c| *c == v)).map_or(0, |i| i as i32 + 1);
    let next = (at + d).rem_euclid(len);
    (next > 0).then(|| choices[(next - 1) as usize])
}

/// One step through the system default (`None`) followed by the enumerated device names. A stored
/// name the host no longer reports steps from the default position.
pub fn cycle_device(current: Option<&str>, names: &[String], d: i32) -> Option<String> {
    let len = names.len() as i32 + 1;
    let at = current.and_then(|v| names.iter().position(|n| n == v)).map_or(0, |i| i as i32 + 1);
    let next = (at + d).rem_euclid(len);
    (next > 0).then(|| names[(next - 1) as usize].clone())
}

/// One step on the POLYPHONY row, clamped to the voice range the mixer is allowed to run with.
pub fn step_polyphony(current: usize, d: i32) -> usize {
    let stepped = current as i64 + d as i64 * AUDIO_POLYPHONY_STEP_VOICES as i64;
    stepped.clamp(AUDIO_POLYPHONY_MIN_VOICES as i64, AUDIO_POLYPHONY_MAX_VOICES as i64) as usize
}

/// The AUDIO tab's live values: the four gains, which reach a running stream immediately, and the
/// four output parameters, which only take effect when the stream is (re)opened. Mirrored to disk
/// through [`PlaySettings`].
#[derive(Clone, Debug, PartialEq)]
pub struct AudioSettings {
    /// Output device name, `None` for the system default.
    pub device: Option<String>,
    /// Requested buffer size in frames, `None` for the backend default.
    pub buffer_frames: Option<u32>,
    /// Requested output rate in Hz, `None` for the device default.
    pub sample_rate: Option<u32>,
    /// Voices the mixer may sound at once.
    pub polyphony: usize,
    /// Master gain applied after the buses are summed.
    pub master: f32,
    /// Gain of the key bus.
    pub key: f32,
    /// Gain of the bgm bus.
    pub bg: f32,
    /// Gain of the system bus.
    pub system: f32,
    reopen_pending: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        AudioSettings {
            device: None,
            buffer_frames: None,
            sample_rate: None,
            polyphony: DEFAULT_MAX_VOICES,
            master: DEFAULT_MASTER_VOLUME,
            key: DEFAULT_BUS_VOLUME,
            bg: DEFAULT_BUS_VOLUME,
            system: DEFAULT_BUS_VOLUME,
            reopen_pending: false,
        }
    }
}

impl AudioSettings {
    /// Live values restored from a settings file. Every field is clamped to what the rows can
    /// produce, so a hand-edited file cannot open the stream outside its supported range.
    pub fn from_settings(s: &PlaySettings) -> AudioSettings {
        AudioSettings {
            device: s.audio_device.clone().filter(|d| !d.trim().is_empty()),
            buffer_frames: s.audio_buffer_frames.filter(|f| *f > 0),
            sample_rate: s.audio_sample_rate.filter(|r| *r > 0),
            polyphony: s.audio_polyphony.clamp(AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_MAX_VOICES),
            master: clamp_volume(s.vol_master, DEFAULT_MASTER_VOLUME),
            key: clamp_volume(s.vol_key, DEFAULT_BUS_VOLUME),
            bg: clamp_volume(s.vol_bg, DEFAULT_BUS_VOLUME),
            system: clamp_volume(s.vol_system, DEFAULT_BUS_VOLUME),
            reopen_pending: false,
        }
    }

    /// Output parameters for [`rbms_audio::AudioEngine::open`].
    pub fn options(&self) -> AudioOptions {
        AudioOptions { device_name: self.device.clone(), sample_rate: self.sample_rate, buffer_frames: self.buffer_frames, max_voices: self.polyphony }
    }

    /// Whether an output parameter has moved since the stream was last opened.
    pub fn reopen_pending(&self) -> bool {
        self.reopen_pending
    }

    /// Record that the stream has to be reopened for the current parameters to be heard.
    pub fn mark_reopen_pending(&mut self) {
        self.reopen_pending = true;
    }

    /// Clear the pending reopen once the stream has been opened with [`AudioSettings::options`].
    pub fn clear_reopen_pending(&mut self) {
        self.reopen_pending = false;
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
        assert_eq!(s.player_id, rbms_ir::GUEST_PLAYER_ID, "an unconfigured client submits under the only id the server accepts without a token");
        assert_eq!(s.ir_token, None);
        assert_eq!(s.ir_login_id, None);
        assert_eq!(s.ir_email, None);
        assert!(!s.sync_settings, "settings sync is opt-in");
        assert!(s.auto_upload_replay, "replay upload rides along with a ranked submit by default");
        assert!(s.rivals.is_empty());
        assert_eq!(s.audio_device, None, "the system default device until one is picked");
        assert_eq!(s.audio_buffer_frames, None);
        assert_eq!(s.audio_sample_rate, None);
        assert_eq!(s.audio_polyphony, DEFAULT_MAX_VOICES);
        assert!((s.vol_master - DEFAULT_MASTER_VOLUME).abs() < 1e-6);
        assert!((s.vol_key - DEFAULT_BUS_VOLUME).abs() < 1e-6);
        assert!((s.vol_bg - DEFAULT_BUS_VOLUME).abs() < 1e-6);
        assert!((s.vol_system - DEFAULT_BUS_VOLUME).abs() < 1e-6);
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
        s.ir_token = Some("tok-123".into());
        s.ir_login_id = Some("dj".into());
        s.ir_email = Some("dj@example.test".into());
        s.sync_settings = true;
        s.auto_upload_replay = false;
        s.rivals = vec!["rivalone".into(), "rivaltwo".into()];
        s.audio_device = Some("Studio Monitors".into());
        s.audio_buffer_frames = Some(384);
        s.audio_sample_rate = Some(96_000);
        s.audio_polyphony = 256;
        s.vol_master = 0.85;
        s.vol_key = 0.7;
        s.vol_bg = 0.35;
        s.vol_system = 0.15;
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
        assert_eq!(back.ir_token.as_deref(), Some("tok-123"));
        assert_eq!(back.ir_login_id.as_deref(), Some("dj"));
        assert_eq!(back.ir_email.as_deref(), Some("dj@example.test"));
        assert!(back.sync_settings);
        assert!(!back.auto_upload_replay);
        assert_eq!(back.rivals, vec!["rivalone".to_string(), "rivaltwo".to_string()]);
        assert_eq!(back.audio_device.as_deref(), Some("Studio Monitors"));
        assert_eq!(back.audio_buffer_frames, Some(384));
        assert_eq!(back.audio_sample_rate, Some(96_000));
        assert_eq!(back.audio_polyphony, 256);
        assert!((back.vol_master - 0.85).abs() < 1e-6);
        assert!((back.vol_key - 0.7).abs() < 1e-6);
        assert!((back.vol_bg - 0.35).abs() < 1e-6);
        assert!((back.vol_system - 0.15).abs() < 1e-6);
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
        assert_eq!(s.ir_token, None, "IR fields added later default without breaking old files");
        assert!(s.auto_upload_replay);
        assert!(s.rivals.is_empty());
        assert_eq!(s.audio_device, None, "audio fields added later default without breaking old files");
        assert_eq!(s.audio_polyphony, DEFAULT_MAX_VOICES);
        assert!((s.vol_key - DEFAULT_BUS_VOLUME).abs() < 1e-6);
    }

    #[test]
    fn a_settings_file_written_before_the_audio_tab_still_loads() {
        let legacy = r#"(
            hispeed: 3.0,
            gauge: "HARD",
            random: "MIRROR",
            judge_rate: 120,
            server_url: Some("https://ir.example/api"),
            player_id: "dj",
            rivals: ["friend"],
        )"#;
        let s: PlaySettings = ron::from_str(legacy).expect("a pre-audio settings file still parses");
        assert!((s.hispeed - 3.0).abs() < 1e-9);
        assert_eq!(s.gauge, "HARD");
        assert_eq!(s.judge_rate, 120);
        assert_eq!(s.rivals, vec!["friend".to_string()]);
        let audio = AudioSettings::from_settings(&s);
        assert_eq!(audio, AudioSettings::default(), "a file with no audio keys opens with the shipped defaults");
    }

    #[test]
    fn empty_unit_ron_is_all_defaults() {
        let s: PlaySettings = ron::from_str("()").unwrap();
        let d = PlaySettings::default();
        assert_eq!(s.gauge, d.gauge);
        assert_eq!(s.judge_rate, d.judge_rate);
        assert_eq!(s.preview, d.preview);
        assert_eq!(s.sync_settings, d.sync_settings);
        assert_eq!(s.auto_upload_replay, d.auto_upload_replay);
        assert_eq!(s.audio_polyphony, d.audio_polyphony);
        assert_eq!(s.audio_buffer_frames, d.audio_buffer_frames);
        assert!((s.vol_master - d.vol_master).abs() < 1e-6);
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

    #[test]
    fn the_shipped_audio_defaults_match_the_settings_defaults() {
        assert_eq!(AudioSettings::default(), AudioSettings::from_settings(&PlaySettings::default()));
    }

    #[test]
    fn restoring_audio_settings_clamps_a_hand_edited_file() {
        let s = PlaySettings {
            audio_device: Some("   ".into()),
            audio_buffer_frames: Some(0),
            audio_sample_rate: Some(0),
            audio_polyphony: 100_000,
            vol_master: 4.0,
            vol_key: -1.0,
            vol_bg: f32::NAN,
            vol_system: 0.25,
            ..PlaySettings::default()
        };
        let audio = AudioSettings::from_settings(&s);
        assert_eq!(audio.device, None, "a blank device name means the system default");
        assert_eq!(audio.buffer_frames, None, "zero frames is not a buffer size");
        assert_eq!(audio.sample_rate, None);
        assert_eq!(audio.polyphony, AUDIO_POLYPHONY_MAX_VOICES);
        assert!((audio.master - 1.0).abs() < 1e-6);
        assert!((audio.key - 0.0).abs() < 1e-6);
        assert!((audio.bg - DEFAULT_BUS_VOLUME).abs() < 1e-6, "a NaN gain falls back instead of silencing the bus");
        assert!((audio.system - 0.25).abs() < 1e-6);
        assert!(!audio.reopen_pending(), "restoring settings is not a parameter change");
    }

    #[test]
    fn audio_options_carry_the_four_output_parameters() {
        let s = PlaySettings {
            audio_device: Some("Studio Monitors".into()),
            audio_buffer_frames: Some(384),
            audio_sample_rate: Some(48_000),
            audio_polyphony: 256,
            ..PlaySettings::default()
        };
        let opts = AudioSettings::from_settings(&s).options();
        assert_eq!(opts.device_name.as_deref(), Some("Studio Monitors"));
        assert_eq!(opts.buffer_frames, Some(384));
        assert_eq!(opts.sample_rate, Some(48_000));
        assert_eq!(opts.max_voices, 256);
        let default_opts = AudioSettings::default().options();
        assert_eq!(default_opts, rbms_audio::AudioOptions::default(), "untouched audio settings open the engine exactly as its own default does");
    }

    #[test]
    fn the_pending_reopen_flag_is_set_and_cleared_explicitly() {
        let mut audio = AudioSettings::default();
        assert!(!audio.reopen_pending());
        audio.mark_reopen_pending();
        assert!(audio.reopen_pending());
        audio.clear_reopen_pending();
        assert!(!audio.reopen_pending());
    }

    #[test]
    fn a_volume_steps_by_five_percent_and_stops_at_the_ends() {
        assert_eq!(volume_percent(step_volume(0.5, 1)), 55);
        assert_eq!(volume_percent(step_volume(0.5, -1)), 45);
        assert_eq!(volume_percent(step_volume(1.0, 1)), AUDIO_VOLUME_MAX_PERCENT, "the loudest step stays at the top");
        assert_eq!(volume_percent(step_volume(0.0, -1)), 0, "a muted row cannot go negative");
        assert_eq!(volume_percent(step_volume(0.02, -1)), 0, "a partial step down lands on mute, not below it");
    }

    #[test]
    fn stepping_a_volume_up_and_back_down_lands_on_the_same_percent() {
        let steps = (AUDIO_VOLUME_MAX_PERCENT - volume_percent(DEFAULT_BUS_VOLUME)) / AUDIO_VOLUME_STEP_PERCENT;
        let mut gain = DEFAULT_BUS_VOLUME;
        for _ in 0..steps {
            gain = step_volume(gain, 1);
        }
        assert_eq!(volume_percent(gain), AUDIO_VOLUME_MAX_PERCENT);
        for _ in 0..steps {
            gain = step_volume(gain, -1);
        }
        assert_eq!(volume_percent(gain), volume_percent(DEFAULT_BUS_VOLUME), "whole-percent steps do not drift");
    }

    #[test]
    fn a_volume_row_only_ever_shows_a_multiple_of_its_step() {
        let mut gain = 0.0;
        for _ in 0..=(AUDIO_VOLUME_MAX_PERCENT / AUDIO_VOLUME_STEP_PERCENT) {
            assert_eq!(volume_percent(gain) % AUDIO_VOLUME_STEP_PERCENT, 0, "a step left the row off the percent grid");
            gain = step_volume(gain, 1);
        }
        assert_eq!(volume_percent(gain), AUDIO_VOLUME_MAX_PERCENT, "stepping past the loudest value stays there");
    }

    #[test]
    fn volume_percent_reports_whole_percent_within_the_range() {
        assert_eq!(volume_percent(0.0), 0);
        assert_eq!(volume_percent(0.5), 50);
        assert_eq!(volume_percent(1.0), AUDIO_VOLUME_MAX_PERCENT);
        assert_eq!(volume_percent(9.0), AUDIO_VOLUME_MAX_PERCENT, "an out-of-range gain still shows a legal percent");
        assert_eq!(volume_percent(-2.0), 0);
    }

    #[test]
    fn an_optional_choice_row_cycles_auto_first_and_wraps_both_ways() {
        let choices = AUDIO_BUFFER_FRAMES_CHOICES;
        assert_eq!(cycle_optional_u32(None, &choices, 1), Some(choices[0]));
        assert_eq!(cycle_optional_u32(Some(choices[0]), &choices, -1), None);
        assert_eq!(cycle_optional_u32(Some(choices[choices.len() - 1]), &choices, 1), None, "past the last size wraps to AUTO");
        assert_eq!(cycle_optional_u32(None, &choices, -1), Some(choices[choices.len() - 1]));
        assert_eq!(cycle_optional_u32(Some(333), &choices, 1), Some(choices[0]), "a size the row never offers steps from AUTO");
    }

    #[test]
    fn the_sample_rate_row_offers_auto_and_the_four_rates() {
        assert_eq!(cycle_optional_u32(None, &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), Some(44_100));
        assert_eq!(cycle_optional_u32(Some(44_100), &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), Some(48_000));
        assert_eq!(cycle_optional_u32(Some(96_000), &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), None);
    }

    #[test]
    fn the_device_row_cycles_the_default_and_the_reported_names() {
        let names = vec!["Built-in Output".to_string(), "Studio Monitors".to_string()];
        assert_eq!(cycle_device(None, &names, 1).as_deref(), Some("Built-in Output"));
        assert_eq!(cycle_device(Some("Built-in Output"), &names, 1).as_deref(), Some("Studio Monitors"));
        assert_eq!(cycle_device(Some("Studio Monitors"), &names, 1), None, "past the last device is the system default");
        assert_eq!(cycle_device(None, &names, -1).as_deref(), Some("Studio Monitors"));
        assert_eq!(
            cycle_device(Some("Unplugged Interface"), &names, 1).as_deref(),
            Some("Built-in Output"),
            "a device the host no longer reports steps from the default"
        );
        assert_eq!(cycle_device(None, &[], 1), None, "with no devices the row stays on the system default");
    }

    #[test]
    fn polyphony_steps_by_sixty_four_within_the_voice_range() {
        assert_eq!(step_polyphony(DEFAULT_MAX_VOICES, 1), DEFAULT_MAX_VOICES + AUDIO_POLYPHONY_STEP_VOICES);
        assert_eq!(step_polyphony(DEFAULT_MAX_VOICES, -1), DEFAULT_MAX_VOICES - AUDIO_POLYPHONY_STEP_VOICES);
        assert_eq!(step_polyphony(AUDIO_POLYPHONY_MIN_VOICES, -1), AUDIO_POLYPHONY_MIN_VOICES);
        assert_eq!(step_polyphony(AUDIO_POLYPHONY_MAX_VOICES, 1), AUDIO_POLYPHONY_MAX_VOICES);
        assert_eq!(step_polyphony(0, -1), AUDIO_POLYPHONY_MIN_VOICES, "a hand-edited zero is pulled back into range");
    }
}
