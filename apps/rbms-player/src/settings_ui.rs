//! The settings screen's row model: the tab layout and the index -> (label, value) / adjust pair
//! that every settings row goes through. Split out of `main.rs`/`app_select.rs` so the row
//! definitions live in one file; the key/mouse routing that calls into them stays with the screens.
//!
//! Row indices are global and shared with `ir_panel` (the NETWORK tab continues the numbering) and
//! with the AUDIO tab defined below (which continues past the NETWORK rows).
#![allow(clippy::wildcard_imports)]
use rbms_audio::cpal::traits::{DeviceTrait, HostTrait};
use rbms_audio::{AudioOpenReport, Bus};

use crate::app_play::audio_reopen_blocked;
use crate::ir_panel::NETWORK_SETTING_ROWS;
use crate::settings::{
    AUDIO_BUFFER_FRAMES_CHOICES, AUDIO_SAMPLE_RATE_HZ_CHOICES, AudioSettings, cycle_device, cycle_optional_u32, step_polyphony, step_volume, volume_percent,
};
use crate::*;

const GAUGE_CYCLE: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];
/// Row index of KEY CONFIG, which opens its own screen instead of adjusting a value.
pub(crate) const SETTING_KEYCONFIG: usize = 11;
/// Row index of FONT, which opens a file picker instead of adjusting a value.
pub(crate) const SETTING_FONT: usize = 18;

/// Global settings-row indices owned by the AUDIO tab, continuing past the NETWORK rows.
pub(crate) const SETTING_MASTER_VOLUME: usize = 35;
pub(crate) const SETTING_KEY_VOLUME: usize = 36;
pub(crate) const SETTING_BGM_VOLUME: usize = 37;
pub(crate) const SETTING_SYSTEM_VOLUME: usize = 38;
pub(crate) const SETTING_AUDIO_DEVICE: usize = 39;
pub(crate) const SETTING_AUDIO_BUFFER: usize = 40;
pub(crate) const SETTING_AUDIO_SAMPLE_RATE: usize = 41;
pub(crate) const SETTING_AUDIO_POLYPHONY: usize = 42;

/// The AUDIO tab, top to bottom: the four gains first, then the four output parameters.
pub(crate) const AUDIO_SETTING_ROWS: &[usize] = &[
    SETTING_MASTER_VOLUME,
    SETTING_KEY_VOLUME,
    SETTING_BGM_VOLUME,
    SETTING_SYSTEM_VOLUME,
    SETTING_AUDIO_DEVICE,
    SETTING_AUDIO_BUFFER,
    SETTING_AUDIO_SAMPLE_RATE,
    SETTING_AUDIO_POLYPHONY,
];

/// Value of an output-parameter row left to the backend.
pub(crate) const AUDIO_AUTO_VALUE: &str = "AUTO";

/// Tab the audio rows live under.
pub(crate) const AUDIO_TAB_NAME: &str = "AUDIO";

/// Shown while the rows describe a stream the app has not been able to open at all.
pub(crate) const AUDIO_UNAVAILABLE_STATUS: &str = "NO OUTPUT DEVICE - RUNNING SILENT";

/// Shown while a parameter change is held back because a chart owns the stream.
pub(crate) const AUDIO_PENDING_STATUS: &str = "APPLIED WHEN YOU LEAVE THE SONG";

/// Shown when the opened stream is exactly what the rows ask for.
pub(crate) const AUDIO_MATCHED_STATUS: &str = "OUTPUT MATCHES THESE SETTINGS";

/// The AUDIO tab's status line. A stream that had to be downgraded says so in the engine's own
/// words, so a row reading 96000 while the device runs at 48000 is visible rather than silent.
pub(crate) fn audio_status_text(report: Option<&AudioOpenReport>, failed: bool, reopen_waiting: bool) -> String {
    if reopen_waiting {
        return AUDIO_PENDING_STATUS.to_string();
    }
    let Some(report) = report else {
        return if failed { AUDIO_UNAVAILABLE_STATUS.to_string() } else { String::new() };
    };
    let opened = format!("{} - {} HZ - {} CH", report.device_name, report.sample_rate, report.channels);
    if report.notes.is_empty() {
        return format!("{AUDIO_MATCHED_STATUS}: {opened}");
    }
    format!("{opened} ({})", report.notes.join("; "))
}

/// Value of the AUDIO DEVICE row while the system default device is used.
pub(crate) const AUDIO_DEFAULT_DEVICE_VALUE: &str = "DEFAULT";

/// Settings grouped into tabs by category. Each entry is `(tab name, [global setting indices])`
/// where the indices map to `setting_line`/`adjust_setting`.
pub(crate) const SETTING_TABS: &[(&str, &[usize])] = &[
    ("PLAY", &[0, 1, 2, 3, 16]),
    ("GAUGE", &[4, 13]),
    ("JUDGE", &[9, 12, 15]),
    ("DISPLAY", &[14, 18, 19, 20, 21, 5, 6, 10, 17]),
    ("INPUT", &[7, 8, 11]),
    ("NETWORK", NETWORK_SETTING_ROWS),
    (AUDIO_TAB_NAME, AUDIO_SETTING_ROWS),
];

/// How a step on an AUDIO row reaches the audio engine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AudioRowChange {
    /// The index is not an AUDIO row.
    None,
    /// A gain moved; it can be pushed to the running stream at once.
    Gain,
    /// An output parameter moved; it is only heard once the stream is reopened.
    Reopen,
}

/// Label and value of one AUDIO row, or `None` when the index belongs to another tab.
pub(crate) fn audio_setting_line(audio: &AudioSettings, index: usize) -> Option<(&'static str, String)> {
    let line = match index {
        SETTING_MASTER_VOLUME => ("MASTER VOL", percent_value(audio.master)),
        SETTING_KEY_VOLUME => ("KEY VOL", percent_value(audio.key)),
        SETTING_BGM_VOLUME => ("BGM VOL", percent_value(audio.bg)),
        SETTING_SYSTEM_VOLUME => ("SYSTEM VOL", percent_value(audio.system)),
        SETTING_AUDIO_DEVICE => ("AUDIO DEVICE", audio.device.clone().unwrap_or_else(|| AUDIO_DEFAULT_DEVICE_VALUE.to_string())),
        SETTING_AUDIO_BUFFER => ("BUFFER SIZE", auto_or_number(audio.buffer_frames)),
        SETTING_AUDIO_SAMPLE_RATE => ("SAMPLE RATE", auto_or_number(audio.sample_rate)),
        SETTING_AUDIO_POLYPHONY => ("POLYPHONY", audio.polyphony.to_string()),
        _ => return None,
    };
    Some(line)
}

/// Value of a gain row.
fn percent_value(gain: f32) -> String {
    format!("{}%", volume_percent(gain))
}

/// Value of an output-parameter row that may be left to the backend.
fn auto_or_number(value: Option<u32>) -> String {
    value.map_or_else(|| AUDIO_AUTO_VALUE.to_string(), |v| v.to_string())
}

/// Apply one left/right step to an AUDIO row and report how the change reaches the engine.
/// `devices` is the enumerated output device list and is only read by the AUDIO DEVICE row.
///
/// A parameter row that lands back on the value it already held leaves no pending reopen, so
/// cycling a one-entry device list does not restart the output stream for nothing.
pub(crate) fn adjust_audio_row(audio: &mut AudioSettings, index: usize, d: i32, devices: &[String]) -> AudioRowChange {
    let moved = match index {
        SETTING_MASTER_VOLUME => {
            audio.master = step_volume(audio.master, d);
            return AudioRowChange::Gain;
        }
        SETTING_KEY_VOLUME => {
            audio.key = step_volume(audio.key, d);
            return AudioRowChange::Gain;
        }
        SETTING_BGM_VOLUME => {
            audio.bg = step_volume(audio.bg, d);
            return AudioRowChange::Gain;
        }
        SETTING_SYSTEM_VOLUME => {
            audio.system = step_volume(audio.system, d);
            return AudioRowChange::Gain;
        }
        SETTING_AUDIO_DEVICE => {
            let next = cycle_device(audio.device.as_deref(), devices, d);
            let moved = next != audio.device;
            audio.device = next;
            moved
        }
        SETTING_AUDIO_BUFFER => {
            let next = cycle_optional_u32(audio.buffer_frames, &AUDIO_BUFFER_FRAMES_CHOICES, d);
            let moved = next != audio.buffer_frames;
            audio.buffer_frames = next;
            moved
        }
        SETTING_AUDIO_SAMPLE_RATE => {
            let next = cycle_optional_u32(audio.sample_rate, &AUDIO_SAMPLE_RATE_HZ_CHOICES, d);
            let moved = next != audio.sample_rate;
            audio.sample_rate = next;
            moved
        }
        SETTING_AUDIO_POLYPHONY => {
            let next = step_polyphony(audio.polyphony, d);
            let moved = next != audio.polyphony;
            audio.polyphony = next;
            moved
        }
        _ => return AudioRowChange::None,
    };
    if moved {
        audio.mark_reopen_pending();
    }
    AudioRowChange::Reopen
}

/// Output device names the AUDIO DEVICE row cycles through, in host order. These are the same
/// description names `rbms_audio` resolves [`rbms_audio::AudioOptions::device_name`] against, so a
/// picked row names a device the engine can find. A device whose description cannot be read is left
/// out rather than offered under a name that would not match. Empty when the host cannot be
/// enumerated, which leaves the row on the system default.
///
/// Enumerating a host takes tens of milliseconds and runs on the frame loop's own thread, so this
/// is called once when the settings screen opens rather than on every keystroke.
fn output_device_names() -> Vec<String> {
    rbms_audio::cpal::default_host()
        .output_devices()
        .map(|devices| devices.filter_map(|device| device.description().ok().map(|d| d.name().to_string())).collect())
        .unwrap_or_default()
}

impl App {
    /// Label and current value of one global settings row. Indices past the local rows fall
    /// through to the NETWORK tab rows owned by `app_network`, then to the AUDIO tab rows.
    pub(crate) fn setting_line(&self, i: usize) -> (&'static str, String) {
        let on = |b: bool| {
            if b { "ON".to_string() } else { "OFF".to_string() }
        };
        match i {
            0 => ("AUTOPLAY", on(self.autoplay)),
            1 => ("HI-SPEED", format!("{:.2}", self.config.hispeed)),
            2 => ("SPEED FIX", if self.config.constant_speed { "CONSTANT".to_string() } else { "FLOATING".to_string() }),
            3 => ("RANDOM", self.config.random.label().to_string()),
            4 => ("GAUGE", gauge_name(self.config.gauge).to_string()),
            5 => ("LIFT", format!("{}%", (self.config.lift * 100.0).round() as i32)),
            6 => ("LANE COVER", format!("{}%", (self.config.cover * 100.0).round() as i32)),
            7 => ("SCRATCH SIDE", if self.config.scratch_left { "LEFT".to_string() } else { "RIGHT".to_string() }),
            8 => ("SCRATCH AUTO", on(self.config.scratch_auto)),
            9 => ("JUDGE OFFSET", format!("{:+} MS", self.config.offset_ms)),
            10 => ("BGA", on(self.config.bga)),
            11 => ("KEY CONFIG", ">".to_string()),
            12 => ("JUDGE WIDTH", format!("{}%", self.config.judge_rate)),
            13 => ("TOTAL", if self.config.total_override > 0.0 { format!("{}", self.config.total_override.round() as i32) } else { "AUTO".to_string() }),
            14 => ("SKIN", if self.config.skin_path.is_some() { "CUSTOM".to_string() } else { self.config.skin_name.clone() }),
            15 => ("AUTO CAL", on(self.config.auto_offset)),
            16 => ("AUTO REPLAY", on(self.config.auto_replay)),
            17 => ("DEBUG MODE", on(self.config.debug)),
            18 => ("FONT", if self.config.font_path.is_some() { "CUSTOM".to_string() } else { "DEFAULT".to_string() }),
            19 => ("SCORE GRAPH", on(self.config.score_graph)),
            20 => ("REPLAY ANALYSIS", on(self.config.replay_analysis)),
            21 => ("PREVIEW", on(self.config.preview)),
            _ => self.network_setting_line(i).or_else(|| audio_setting_line(&self.config.audio, i)).unwrap_or(("", String::new())),
        }
    }

    /// Apply one left/right step to a global settings row. Toggling REPLAY ANALYSIS re-evaluates
    /// an in-progress replay so the change takes effect during the current play.
    pub(crate) fn adjust_setting(&mut self, global: usize, d: i32) {
        match global {
            0 => self.autoplay = !self.autoplay,
            1 => self.config.hispeed = (self.config.hispeed + d as f64 * 0.25).clamp(0.5, 10.0),
            2 => self.config.constant_speed = !self.config.constant_speed,
            3 => {
                let idx = NoteOption::ALL.iter().position(|o| *o == self.config.random).unwrap_or(0);
                self.config.random = NoteOption::ALL[((idx as i32 + d).rem_euclid(NoteOption::ALL.len() as i32)) as usize];
            }
            4 => {
                let idx = GAUGE_CYCLE.iter().position(|g| *g == self.config.gauge).unwrap_or(2);
                self.config.gauge = GAUGE_CYCLE[((idx as i32 + d).rem_euclid(GAUGE_CYCLE.len() as i32)) as usize];
            }
            5 => self.config.lift = (self.config.lift + d as f32 * 0.05).clamp(0.0, 0.9),
            6 => self.config.cover = (self.config.cover + d as f32 * 0.05).clamp(0.0, 0.9),
            7 => self.config.scratch_left = !self.config.scratch_left,
            8 => self.config.scratch_auto = !self.config.scratch_auto,
            9 => self.config.offset_ms = (self.config.offset_ms + d * 5).clamp(-200, 200),
            10 => self.config.bga = !self.config.bga,
            12 => self.config.judge_rate = (self.config.judge_rate + d * 5).clamp(50, 200),
            13 => self.config.total_override = (self.config.total_override + d as f64 * 10.0).max(0.0),
            15 => self.config.auto_offset = !self.config.auto_offset,
            16 => self.config.auto_replay = !self.config.auto_replay,
            17 => self.config.debug = !self.config.debug,
            19 => self.config.score_graph = !self.config.score_graph,
            20 => {
                self.config.replay_analysis = !self.config.replay_analysis;
                if self.stage == Stage::Play {
                    self.analysis = self.replay.is_some() && self.config.replay_analysis;
                }
            }
            21 => self.config.preview = !self.config.preview,
            18 => {
                if d < 0 {
                    self.reset_font();
                } else {
                    self.pick_font();
                }
            }
            14 => {
                self.config.skin_path = None;
                self.config.skin_name = if self.config.skin_name.eq_ignore_ascii_case("WIDE") { "NORMAL".into() } else { "WIDE".into() };
            }
            _ => self.adjust_audio_setting(global, d),
        }
    }

    /// Step an AUDIO tab row. A gain reaches the running stream at once; an output parameter is
    /// only recorded, leaving [`App::audio_reopen_pending`] set for the frame loop to act on.
    fn adjust_audio_setting(&mut self, index: usize, d: i32) {
        let devices = std::mem::take(&mut self.audio_devices);
        let change = adjust_audio_row(&mut self.config.audio, index, d, &devices);
        self.audio_devices = devices;
        if change == AudioRowChange::Gain {
            self.apply_audio_gains();
        }
    }

    /// Enter the settings screen, re-reading the host's output device list on the way in so the
    /// AUDIO DEVICE row cycles through what is plugged in now without enumerating on every
    /// keystroke.
    pub(crate) fn open_settings(&mut self) {
        self.audio_devices = output_device_names();
        self.stage = Stage::Settings;
    }

    /// The AUDIO tab's status line: why the stream is not exactly what the rows ask for, and
    /// whether a change is still waiting for a chart to finish before it can be applied.
    pub(crate) fn audio_status_line(&self) -> Option<String> {
        if !self.on_audio_tab() {
            return None;
        }
        let waiting = self.audio_reopen_at.is_some() && audio_reopen_blocked(&self.stage, self.player.is_some(), self.ks_rx.is_some());
        Some(audio_status_text(self.audio_report.as_ref(), self.audio_failed, waiting))
    }

    /// Whether the settings screen is showing the AUDIO tab.
    pub(crate) fn on_audio_tab(&self) -> bool {
        SETTING_TABS.get(self.set_tab).is_some_and(|(name, _)| *name == AUDIO_TAB_NAME)
    }

    /// Push the master and per-bus gains to the running output stream. Also called right after the
    /// engine is opened, so a restored settings file is heard from the first note.
    pub(crate) fn apply_audio_gains(&mut self) {
        if let Some(audio) = self.audio.as_mut() {
            audio.set_master_gain(self.config.audio.master);
            audio.set_bus_gain(Bus::System, self.config.audio.system);
            audio.set_bus_gain(Bus::Key, self.config.audio.key);
            audio.set_bus_gain(Bus::Bg, self.config.audio.bg);
        }
    }

    /// Output parameters the audio engine should be opened with.
    pub(crate) fn audio_options(&self) -> rbms_audio::AudioOptions {
        self.config.audio.options()
    }

    /// Whether an AUDIO row moved a parameter that only a stream reopen can apply.
    pub(crate) fn audio_reopen_pending(&self) -> bool {
        self.config.audio.reopen_pending()
    }

    /// Clear the pending reopen once the stream has been opened with [`App::audio_options`].
    pub(crate) fn clear_audio_reopen_pending(&mut self) {
        self.config.audio.clear_reopen_pending();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_STEP_VOICES, AUDIO_VOLUME_STEP_PERCENT, DEFAULT_BUS_VOLUME, DEFAULT_MASTER_VOLUME};

    #[test]
    fn every_settings_row_appears_in_exactly_one_tab() {
        let mut rows: Vec<usize> = SETTING_TABS.iter().flat_map(|(_, items)| items.iter().copied()).collect();
        let total = rows.len();
        rows.sort_unstable();
        rows.dedup();
        assert_eq!(rows.len(), total, "a settings row is listed in two tabs");
    }

    #[test]
    fn tab_names_and_the_network_rows_are_the_moved_layout() {
        let names: Vec<&str> = SETTING_TABS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec!["PLAY", "GAUGE", "JUDGE", "DISPLAY", "INPUT", "NETWORK", "AUDIO"]);
        assert_eq!(SETTING_TABS.iter().find(|(name, _)| *name == "NETWORK").expect("NETWORK tab").1, NETWORK_SETTING_ROWS);
        assert_eq!(SETTING_TABS.last().expect("AUDIO tab").1, AUDIO_SETTING_ROWS);
    }

    #[test]
    fn the_rows_that_open_a_screen_keep_their_indices() {
        assert_eq!(SETTING_KEYCONFIG, 11);
        assert_eq!(SETTING_FONT, 18);
        let input_rows = SETTING_TABS.iter().find(|(name, _)| *name == "INPUT").expect("INPUT tab").1;
        assert!(input_rows.contains(&SETTING_KEYCONFIG));
        let display_rows = SETTING_TABS.iter().find(|(name, _)| *name == "DISPLAY").expect("DISPLAY tab").1;
        assert!(display_rows.contains(&SETTING_FONT));
    }

    #[test]
    fn the_audio_rows_continue_past_the_last_network_row() {
        assert_eq!(AUDIO_SETTING_ROWS[0], crate::ir_panel::SETTING_RIVALS + 1, "the AUDIO tab starts one past the highest NETWORK row");
        for (offset, row) in AUDIO_SETTING_ROWS.iter().enumerate() {
            assert_eq!(*row, AUDIO_SETTING_ROWS[0] + offset, "the AUDIO rows are contiguous");
            assert!(!NETWORK_SETTING_ROWS.contains(row), "an AUDIO row is not a NETWORK row");
        }
        assert_eq!(AUDIO_SETTING_ROWS.len(), 8);
    }

    #[test]
    fn every_audio_row_has_a_distinct_label_and_a_value() {
        let audio = AudioSettings::default();
        let mut labels: Vec<&str> = Vec::new();
        for &row in AUDIO_SETTING_ROWS {
            let (label, value) = audio_setting_line(&audio, row).expect("an AUDIO row has a line");
            assert!(!value.is_empty(), "row {row} shows nothing");
            labels.push(label);
        }
        let count = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), count, "no two AUDIO rows share a label");
    }

    #[test]
    fn a_row_from_another_tab_is_not_an_audio_row() {
        let audio = AudioSettings::default();
        assert_eq!(audio_setting_line(&audio, 0), None, "AUTOPLAY belongs to the PLAY tab");
        assert_eq!(audio_setting_line(&audio, crate::ir_panel::SETTING_RIVALS), None);
        assert_eq!(audio_setting_line(&audio, AUDIO_SETTING_ROWS[AUDIO_SETTING_ROWS.len() - 1] + 1), None);
        assert_eq!(adjust_audio_row(&mut AudioSettings::default(), 0, 1, &[]), AudioRowChange::None);
    }

    #[test]
    fn the_default_audio_rows_read_as_the_shipped_settings() {
        let audio = AudioSettings::default();
        assert_eq!(audio_setting_line(&audio, SETTING_MASTER_VOLUME), Some(("MASTER VOL", "100%".to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_KEY_VOLUME), Some(("KEY VOL", "50%".to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_BGM_VOLUME), Some(("BGM VOL", "50%".to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_SYSTEM_VOLUME), Some(("SYSTEM VOL", "50%".to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_DEVICE), Some(("AUDIO DEVICE", AUDIO_DEFAULT_DEVICE_VALUE.to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_BUFFER), Some(("BUFFER SIZE", AUDIO_AUTO_VALUE.to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_SAMPLE_RATE), Some(("SAMPLE RATE", AUDIO_AUTO_VALUE.to_string())));
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_POLYPHONY), Some(("POLYPHONY", rbms_audio::DEFAULT_MAX_VOICES.to_string())));
    }

    #[test]
    fn a_gain_row_step_shows_up_in_its_own_value_only() {
        let mut audio = AudioSettings::default();
        assert_eq!(adjust_audio_row(&mut audio, SETTING_KEY_VOLUME, 1, &[]), AudioRowChange::Gain);
        assert_eq!(audio_setting_line(&audio, SETTING_KEY_VOLUME).expect("KEY VOL").1, "55%");
        assert_eq!(audio_setting_line(&audio, SETTING_BGM_VOLUME).expect("BGM VOL").1, "50%", "one gain row does not move another");
        assert_eq!(audio_setting_line(&audio, SETTING_MASTER_VOLUME).expect("MASTER VOL").1, "100%");
        assert!(!audio.reopen_pending(), "a gain never asks for a stream reopen");
    }

    #[test]
    fn every_gain_row_moves_the_gain_it_names() {
        let rows = [
            (SETTING_MASTER_VOLUME, volume_percent(DEFAULT_MASTER_VOLUME)),
            (SETTING_KEY_VOLUME, volume_percent(DEFAULT_BUS_VOLUME)),
            (SETTING_BGM_VOLUME, volume_percent(DEFAULT_BUS_VOLUME)),
            (SETTING_SYSTEM_VOLUME, volume_percent(DEFAULT_BUS_VOLUME)),
        ];
        for (row, start) in rows {
            let mut audio = AudioSettings::default();
            assert_eq!(adjust_audio_row(&mut audio, row, -1, &[]), AudioRowChange::Gain);
            let shown = audio_setting_line(&audio, row).expect("a gain row").1;
            assert_eq!(shown, format!("{}%", start - AUDIO_VOLUME_STEP_PERCENT), "row {row} did not step down by one volume step");
        }
    }

    #[test]
    fn the_output_parameter_rows_ask_for_a_reopen_and_cycle_their_values() {
        let mut audio = AudioSettings::default();
        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_BUFFER, 1, &[]), AudioRowChange::Reopen);
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_BUFFER).expect("BUFFER SIZE").1, AUDIO_BUFFER_FRAMES_CHOICES[0].to_string());
        assert!(audio.reopen_pending());

        audio.clear_reopen_pending();
        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_SAMPLE_RATE, 1, &[]), AudioRowChange::Reopen);
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_SAMPLE_RATE).expect("SAMPLE RATE").1, AUDIO_SAMPLE_RATE_HZ_CHOICES[0].to_string());
        assert!(audio.reopen_pending());

        audio.clear_reopen_pending();
        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_POLYPHONY, -1, &[]), AudioRowChange::Reopen);
        assert_eq!(audio.polyphony, rbms_audio::DEFAULT_MAX_VOICES - AUDIO_POLYPHONY_STEP_VOICES);
        assert!(audio.reopen_pending());

        audio.clear_reopen_pending();
        let devices = vec!["Built-in Output".to_string()];
        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_DEVICE, 1, &devices), AudioRowChange::Reopen);
        assert_eq!(audio_setting_line(&audio, SETTING_AUDIO_DEVICE).expect("AUDIO DEVICE").1, "Built-in Output");
        assert!(audio.reopen_pending());
    }

    #[test]
    fn a_parameter_row_that_cannot_move_leaves_no_pending_reopen() {
        let mut audio = AudioSettings::default();
        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_DEVICE, 1, &[]), AudioRowChange::Reopen);
        assert_eq!(audio.device, None, "with no devices reported the row stays on the system default");
        assert!(!audio.reopen_pending(), "a row that did not move must not restart the output stream");

        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_POLYPHONY, -1, &[]), AudioRowChange::Reopen);
        assert!(audio.reopen_pending());
        audio.clear_reopen_pending();
        for _ in 0..32 {
            adjust_audio_row(&mut audio, SETTING_AUDIO_POLYPHONY, -1, &[]);
        }
        assert_eq!(audio.polyphony, AUDIO_POLYPHONY_MIN_VOICES);
        audio.clear_reopen_pending();
        assert_eq!(adjust_audio_row(&mut audio, SETTING_AUDIO_POLYPHONY, -1, &[]), AudioRowChange::Reopen);
        assert!(!audio.reopen_pending(), "a clamped row that did not move must not restart the output stream");
    }

    fn report(sample_rate: u32, notes: &[&str]) -> AudioOpenReport {
        AudioOpenReport {
            device_name: "Built-in Output".to_string(),
            sample_rate,
            channels: 2,
            buffer: rbms_audio::cpal::BufferSize::Default,
            fallback_step: 0,
            notes: notes.iter().map(|n| (*n).to_string()).collect(),
        }
    }

    #[test]
    fn the_audio_status_names_the_stream_that_was_actually_opened() {
        let downgraded = report(48_000, &["requested 96000 Hz -> device supports 48000 Hz"]);
        let line = audio_status_text(Some(&downgraded), false, false);
        assert!(line.contains("48000"), "the opened rate must be visible next to the row value: {line}");
        assert!(line.contains("requested 96000 Hz"), "the downgrade reason must be shown: {line}");
        assert!(line.contains("Built-in Output"), "the opened device must be named: {line}");
    }

    #[test]
    fn the_audio_status_says_so_when_nothing_had_to_be_downgraded() {
        let line = audio_status_text(Some(&report(48_000, &[])), false, false);
        assert!(line.starts_with(AUDIO_MATCHED_STATUS), "got {line}");
        assert!(line.contains("48000"));
    }

    #[test]
    fn the_audio_status_reports_a_change_held_back_by_a_running_chart() {
        assert_eq!(audio_status_text(Some(&report(48_000, &[])), false, true), AUDIO_PENDING_STATUS);
        assert_eq!(audio_status_text(None, true, true), AUDIO_PENDING_STATUS, "a pending change outranks the open state");
    }

    #[test]
    fn the_audio_status_reports_a_stream_that_never_opened() {
        assert_eq!(audio_status_text(None, true, false), AUDIO_UNAVAILABLE_STATUS);
        assert_eq!(audio_status_text(None, false, false), "", "before the first open attempt there is nothing to say");
    }

    #[test]
    fn the_options_the_engine_opens_with_follow_the_parameter_rows() {
        let mut audio = AudioSettings::default();
        let devices = vec!["Studio Monitors".to_string()];
        adjust_audio_row(&mut audio, SETTING_AUDIO_DEVICE, 1, &devices);
        adjust_audio_row(&mut audio, SETTING_AUDIO_BUFFER, 1, &[]);
        adjust_audio_row(&mut audio, SETTING_AUDIO_SAMPLE_RATE, 2, &[]);
        adjust_audio_row(&mut audio, SETTING_AUDIO_POLYPHONY, 1, &[]);
        let opts = audio.options();
        assert_eq!(opts.device_name.as_deref(), Some("Studio Monitors"));
        assert_eq!(opts.buffer_frames, Some(AUDIO_BUFFER_FRAMES_CHOICES[0]));
        assert_eq!(opts.sample_rate, Some(AUDIO_SAMPLE_RATE_HZ_CHOICES[1]));
        assert_eq!(opts.max_voices, rbms_audio::DEFAULT_MAX_VOICES + AUDIO_POLYPHONY_STEP_VOICES);
    }
}
