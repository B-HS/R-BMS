//! What the settings screen needs from the rest of the app: the value of a row once the running
//! program has had its say, the AUDIO tab's status line, and the two audio steps that need
//! something only the program can enumerate.
//!
//! The rows themselves — their tabs, labels, kinds and ranges — are described once in
//! `rbms_config::SETTINGS`. This file only adds what the configuration document cannot know: the
//! live account, the password held in memory, and a skin forced on the command line. The device
//! list the AUDIO DEVICE row cycles through comes from [`rbms_audio::output_device_names`], which
//! owns the one thread the audio host may be questioned from.
#![allow(clippy::wildcard_imports)]
use rbms_audio::{AudioOpenReport, Bus};

use rbms_config::{AudioOptions, CUSTOM_VALUE, SettingId, SettingTab, cycle_device, descriptor, display_value, tab_rows};

use crate::skin_select::SkinRow;
use crate::*;

/// The row the skin document's own customisation rows are listed directly above.
const SKIN_CUSTOM_ANCHOR: SettingId = SettingId::SkinReload;

/// One row of the settings screen.
///
/// Every tab but SKIN is exactly the descriptor table's rows. A skin document declares its own
/// customisation rows, so the SKIN tab has [`SettingRow::Skin`] entries between the document row and
/// the actions under it, and the screen addresses a row by what it is rather than by an index into
/// one flat list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SettingRow {
    /// A row of `rbms_config::SETTINGS`.
    Fixed(SettingId),
    /// A customisation row the chosen skin document declares.
    Skin(SkinRow),
}

/// Shown while the rows describe a stream the app has not been able to open at all.
pub(crate) const AUDIO_UNAVAILABLE_STATUS: &str = "NO OUTPUT DEVICE - RUNNING SILENT";

/// Shown when the opened stream is exactly what the rows ask for.
pub(crate) const AUDIO_MATCHED_STATUS: &str = "OUTPUT MATCHES THESE SETTINGS";

/// The AUDIO tab's status line. A stream that had to be downgraded says so in the engine's own
/// words, so a row reading 96000 while the device runs at 48000 is visible rather than silent.
///
/// There is no "waiting for the song to end" state: the settings screen is only ever opened over
/// the browser, and a chart owns the stream only on LOADING, PLAY and RESULT — none of which the
/// settings screen can be shown from. A change made here is applied as soon as the debounce
/// elapses, which is why the line only ever describes the stream that is open.
pub(crate) fn audio_status_text(report: Option<&AudioOpenReport>, failed: bool) -> String {
    let Some(report) = report else {
        return if failed { AUDIO_UNAVAILABLE_STATUS.to_string() } else { String::new() };
    };
    let opened = format!("{} - {} HZ - {} CH", report.device_name, report.sample_rate, report.channels);
    if report.notes.is_empty() {
        return format!("{AUDIO_MATCHED_STATUS}: {opened}");
    }
    format!("{opened} ({})", report.notes.join("; "))
}

/// Whether the row belongs to the NETWORK tab, which `ir_panel` and `app_network` render because
/// they are the only ones that can see the live account and the password held in memory.
pub(crate) fn is_network_row(id: SettingId) -> bool {
    descriptor(id).tab == SettingTab::Network
}

/// Step the AUDIO DEVICE row through the system default and the enumerated output devices, and
/// report whether the row moved. A row that lands back on the device it already named leaves no
/// pending reopen, so cycling a one-entry list does not restart the output stream for nothing.
pub(crate) fn step_audio_device(audio: &mut AudioOptions, delta: i32, devices: &[String]) -> bool {
    let next = cycle_device(audio.device.as_deref(), devices, delta);
    if next == audio.device {
        return false;
    }
    audio.device = next;
    audio.mark_reopen_pending();
    true
}

/// Output parameters [`rbms_audio::AudioEngine::open`] should be handed for the configured audio
/// settings.
pub(crate) fn engine_options(audio: &AudioOptions) -> rbms_audio::AudioOptions {
    rbms_audio::AudioOptions {
        device_name: audio.device.clone(),
        sample_rate: audio.sample_rate,
        buffer_frames: audio.buffer_frames,
        max_voices: audio.polyphony,
    }
}

impl AppShared {
    /// Label and current value of one settings row.
    ///
    /// The NETWORK rows keep the renderer `app_network` already has for them, which is the only one
    /// that can see the live account and the password held in memory. Of the rest, the gauge reads
    /// with the same name the result screen gives it, and a skin forced on the command line
    /// outranks the one in the document; everything else is exactly what the document says.
    pub(crate) fn setting_line(&self, id: SettingId) -> (&'static str, String) {
        if let Some(line) = self.network_setting_line(id) {
            return line;
        }
        let label = descriptor(id).label;
        let value = match id {
            SettingId::Gauge => gauge_name(self.config.play.gauge).to_string(),
            SettingId::Skin if self.launch.skin_path.is_some() => CUSTOM_VALUE.to_string(),
            SettingId::SkinDocument => self.skins.document_value(&self.config),
            SettingId::SkinInfo => self.skins.info(&self.config),
            _ => display_value(&self.config, id),
        };
        (label, value)
    }

    /// The rows one tab shows, top to bottom.
    ///
    /// The SKIN tab is the only one whose length is not fixed: the chosen document's own
    /// customisation rows sit between the document row and the actions under it.
    pub(crate) fn settings_rows(&self, tab: SettingTab) -> Vec<SettingRow> {
        let fixed = tab_rows(tab, &self.config);
        if tab != SettingTab::Skin {
            return fixed.into_iter().map(SettingRow::Fixed).collect();
        }
        let custom = self.skins.rows(&self.config);
        let at = fixed.iter().position(|id| *id == SKIN_CUSTOM_ANCHOR).unwrap_or(fixed.len());
        let mut rows: Vec<SettingRow> = fixed[..at].iter().copied().map(SettingRow::Fixed).collect();
        rows.extend(custom.into_iter().map(SettingRow::Skin));
        rows.extend(fixed[at..].iter().copied().map(SettingRow::Fixed));
        rows
    }

    /// Label and value of one row of the settings screen.
    pub(crate) fn settings_line(&self, row: SettingRow) -> (String, String) {
        match row {
            SettingRow::Fixed(id) => {
                let (label, value) = self.setting_line(id);
                (label.to_string(), value)
            }
            SettingRow::Skin(row) => self.skins.line(&self.config, row),
        }
    }

    /// Step one of the chosen document's customisation rows, and report whether it moved.
    pub(crate) fn step_skin_row(&mut self, row: SkinRow, delta: i32) -> bool {
        self.skins.step(&mut self.config, row, delta)
    }

    /// Put one of the chosen document's customisation rows back to what its author chose.
    pub(crate) fn reset_skin_row(&mut self, row: SkinRow) -> bool {
        self.skins.reset_row(&mut self.config, row)
    }

    /// Step the SKIN row through the built-in screen and every document that draws this screen.
    pub(crate) fn cycle_skin_document(&mut self, delta: i32) -> bool {
        self.skins.cycle_document(&mut self.config, delta)
    }

    /// Drop every choice made in the chosen document and read it again as its author shipped it.
    pub(crate) fn reset_skin_document(&mut self) {
        self.skins.forget(&mut self.config);
        self.reload_skin();
    }

    /// Walk the skin folder again and read the chosen document with the choices made for it.
    pub(crate) fn rescan_skins(&mut self) {
        let settings_path = self.settings_path.clone();
        self.skins.rescan(&settings_path, &self.config);
    }

    /// Read the chosen document again, so what is on screen is what the rows say.
    pub(crate) fn reload_skin(&mut self) {
        self.skins.reload(&self.config);
    }

    /// Whether the document chosen for the open screen is not the one that has been read.
    pub(crate) fn skin_reload_pending(&self) -> bool {
        self.skins.needs_reload(&self.config)
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
        engine_options(&self.config.audio)
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
    use crate::ir_panel::{is_secret_row, is_text_row, network_row_label};
    use rbms_config::{
        AUDIO_BUFFER_FRAMES_CHOICES, AUDIO_POLYPHONY_MIN_VOICES, AUDIO_POLYPHONY_STEP_VOICES, AUDIO_SAMPLE_RATE_HZ_CHOICES, AUDIO_VOLUME_STEP_PERCENT,
        AdjustOutcome, DEFAULT_BUS_VOLUME, DEFAULT_MASTER_VOLUME, GAUGE_CYCLE, SETTING_COUNT, SettingKind, SettingTab, adjust, tab_rows, volume_percent,
    };

    /// Every row of the settings screen with the label and the value it showed before the rows were
    /// described in one table. A fresh document, so the values are the shipped defaults.
    const SHIPPED_ROWS: [(SettingId, &str, &str); SETTING_COUNT] = [
        (SettingId::Autoplay, "AUTOPLAY", "ON"),
        (SettingId::HiSpeed, "HI-SPEED", "2.00"),
        (SettingId::HiSpeedStep, "HI-SPEED STEP", "0.25"),
        (SettingId::SpeedFix, "SPEED FIX", "FLOATING"),
        (SettingId::FixHiSpeed, "HI-SPEED FIX", "MAIN"),
        (SettingId::Random, "RANDOM", "OFF"),
        (SettingId::LaneOption, "LANE OPTION", "OFF"),
        (SettingId::LegacyNote, "LEGACY NOTE", "OFF"),
        (SettingId::FiveKeyLayout, "5KEYS LAYOUT", "OFF"),
        (SettingId::PlayEscapeMode, "ESCAPE", "IMMEDIATE"),
        (SettingId::Gauge, "GAUGE", "NORMAL"),
        (SettingId::Lift, "LIFT", "0%"),
        (SettingId::LaneCover, "LANE COVER", "0%"),
        (SettingId::ScratchSide, "SCRATCH SIDE", "RIGHT"),
        (SettingId::ScratchAuto, "SCRATCH AUTO", "OFF"),
        (SettingId::JudgeOffset, "JUDGE OFFSET", "+0 MS"),
        (SettingId::Bga, "BGA", "ON"),
        (SettingId::KeyConfig, "KEY CONFIG", ">"),
        (SettingId::JudgeAlgorithm, "JUDGE ALGORITHM", "COMBO"),
        (SettingId::JudgeWidthKeyPGreat, "JUDGE WIDTH KEY PG", "100%"),
        (SettingId::JudgeWidthKeyGreat, "JUDGE WIDTH KEY GR", "100%"),
        (SettingId::JudgeWidthKeyGood, "JUDGE WIDTH KEY GD", "100%"),
        (SettingId::JudgeWidthScratchPGreat, "JUDGE WIDTH SCR PG", "100%"),
        (SettingId::JudgeWidthScratchGreat, "JUDGE WIDTH SCR GR", "100%"),
        (SettingId::JudgeWidthScratchGood, "JUDGE WIDTH SCR GD", "100%"),
        (SettingId::LongNoteMargin, "LN MARGIN", "100%"),
        (SettingId::LnMode, "LN MODE", "LN"),
        (SettingId::GaugeSet, "GAUGE SET", "AUTO"),
        (SettingId::GaugeAutoShift, "GAUGE AUTO SHIFT", "NONE"),
        (SettingId::BottomShiftableGauge, "BOTTOM SHIFTABLE", "ASSIST EASY"),
        (SettingId::Target, "TARGET", "RATE AAA"),
        (SettingId::Total, "TOTAL", "AUTO"),
        (SettingId::Skin, "SKIN", "NORMAL"),
        (SettingId::SkinScreen, "SCREEN", "PLAY 7KEYS"),
        (SettingId::SkinDocument, "SKIN", "DEFAULT"),
        (SettingId::SkinInfo, "LOADED", "BUILT-IN SCREEN"),
        (SettingId::SkinReload, "RELOAD", ">"),
        (SettingId::SkinReset, "RESET", ">"),
        (SettingId::AutoCal, "AUTO CAL", "OFF"),
        (SettingId::AutoReplay, "AUTO REPLAY", "ON"),
        (SettingId::DebugMode, "DEBUG MODE", "OFF"),
        (SettingId::Font, "FONT", "DEFAULT"),
        (SettingId::ScoreGraph, "SCORE GRAPH", "ON"),
        (SettingId::ResultGraphs, "RESULT GRAPHS", "ON"),
        (SettingId::ReplayAnalysis, "REPLAY ANALYSIS", "ON"),
        (SettingId::Preview, "PREVIEW", "ON"),
        (SettingId::EnableLift, "LIFT ON", "OFF"),
        (SettingId::EnableCover, "LANE COVER ON", "ON"),
        (SettingId::Hidden, "HIDDEN+", "0%"),
        (SettingId::EnableHidden, "HIDDEN+ ON", "OFF"),
        (SettingId::LaneCoverFineStep, "COVER FINE STEP", "0.0010"),
        (SettingId::WhiteNumber, "WHITE NUMBER", "OFF"),
        (SettingId::JudgeTextY, "JUDGE TEXT Y", "0%"),
        (SettingId::Letterbox, "LETTERBOX", "OFF"),
        (SettingId::Sort, "SORT", "DEFAULT"),
        (SettingId::FavoriteOnly, "FAVORITE ONLY", "OFF"),
        (SettingId::PreviewVolume, "PREVIEW VOLUME", "85%"),
        (SettingId::PreviewFade, "PREVIEW FADE", "200 MS"),
        (SettingId::ServerUrl, "SERVER URL", "(none)"),
        (SettingId::PlayerId, "PLAYER ID", "guest"),
        (SettingId::Account, "ACCOUNT", "guest"),
        (SettingId::Email, "EMAIL", "(none)"),
        (SettingId::Password, "PASSWORD", "(not set)"),
        (SettingId::Login, "LOGIN", ">"),
        (SettingId::Register, "REGISTER", ">"),
        (SettingId::Logout, "LOGOUT", ">"),
        (SettingId::SyncSettings, "SYNC SETTINGS", "OFF"),
        (SettingId::UploadSettings, "UPLOAD SETTINGS NOW", ">"),
        (SettingId::DownloadSettings, "DOWNLOAD SETTINGS NOW", ">"),
        (SettingId::AutoUploadReplay, "AUTO UPLOAD REPLAY", "ON"),
        (SettingId::Rivals, "RIVALS", "0"),
        (SettingId::IrProfiles, "IR PROFILES", "0"),
        (SettingId::MasterVolume, "MASTER VOL", "100%"),
        (SettingId::KeyVolume, "KEY VOL", "50%"),
        (SettingId::BgmVolume, "BGM VOL", "50%"),
        (SettingId::SystemVolume, "SYSTEM VOL", "50%"),
        (SettingId::AudioDevice, "AUDIO DEVICE", "DEFAULT"),
        (SettingId::AudioBuffer, "BUFFER SIZE", "AUTO"),
        (SettingId::AudioSampleRate, "SAMPLE RATE", "AUTO"),
        (SettingId::AudioPolyphony, "POLYPHONY", "512"),
        (SettingId::SoundFolder, "SOUND FOLDER", "NONE"),
        (SettingId::GuideSe, "GUIDE SE", "OFF"),
    ];

    /// An app with no library, no window and no server, so every row reads out of a fresh document.
    fn shared() -> AppShared {
        let dir = std::env::temp_dir().join(format!("rbms-settings-ui-tests-{}", std::process::id()));
        crate::App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron")).shared
    }

    #[test]
    fn every_row_still_reads_exactly_as_it_did_before_the_rows_were_described_in_one_table() {
        let shared = shared();
        for (id, label, value) in SHIPPED_ROWS {
            assert_eq!(shared.setting_line(id), (label, value.to_string()), "{id:?}");
        }
    }

    #[test]
    fn the_snapshot_covers_every_row_once_and_in_list_order() {
        let listed: Vec<SettingId> = SHIPPED_ROWS.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(listed, SettingId::ALL.to_vec());
    }

    #[test]
    fn the_rows_the_panel_renders_are_exactly_the_descriptor_network_tab() {
        let listed = tab_rows(SettingTab::Network, &Config::default());
        for id in SettingId::ALL {
            assert_eq!(is_network_row(id), listed.contains(&id), "{id:?}");
            assert_eq!(network_row_label(id), listed.contains(&id).then_some(descriptor(id).label), "{id:?} is labelled twice and differently");
        }
        for id in tab_rows(SettingTab::Audio, &Config::default()) {
            assert!(!is_network_row(id), "{id:?} is on the AUDIO tab");
        }
        assert!(!is_network_row(SettingId::Autoplay));
    }

    #[test]
    fn the_rows_the_panel_edits_as_text_are_the_rows_the_table_calls_text() {
        for id in SettingId::ALL {
            let text = matches!(descriptor(id).kind, SettingKind::Text { .. });
            assert_eq!(is_text_row(id), text, "{id:?}");
            let secret = matches!(descriptor(id).kind, SettingKind::Text { secret: true, .. });
            assert_eq!(is_secret_row(id), secret, "{id:?}");
        }
    }

    #[test]
    fn the_gauge_row_reads_with_the_name_the_result_screen_gives_the_gauge() {
        let mut shared = shared();
        for gauge in GAUGE_CYCLE {
            shared.config.play.gauge = gauge;
            assert_eq!(shared.setting_line(SettingId::Gauge).1, gauge_name(gauge));
            assert_eq!(display_value(&shared.config, SettingId::Gauge), gauge_name(gauge), "the table and the result screen disagree on {gauge:?}");
        }
    }

    #[test]
    fn a_skin_forced_on_the_command_line_outranks_the_one_in_the_document() {
        let mut shared = shared();
        assert_eq!(shared.setting_line(SettingId::Skin).1, "NORMAL");
        shared.launch.skin_path = Some("custom.ron".to_string());
        assert_eq!(shared.setting_line(SettingId::Skin).1, CUSTOM_VALUE);
    }

    #[test]
    fn a_gain_row_step_shows_up_in_its_own_value_only() {
        let mut shared = shared();
        assert_eq!(adjust(&mut shared.config, SettingId::KeyVolume, 1), AdjustOutcome::Changed);
        assert_eq!(shared.setting_line(SettingId::KeyVolume).1, "55%");
        assert_eq!(shared.setting_line(SettingId::BgmVolume).1, "50%", "one gain row does not move another");
        assert_eq!(shared.setting_line(SettingId::MasterVolume).1, "100%");
        assert!(!shared.audio_reopen_pending(), "a gain never asks for a stream reopen");
    }

    #[test]
    fn every_gain_row_moves_the_gain_it_names() {
        let rows = [
            (SettingId::MasterVolume, volume_percent(DEFAULT_MASTER_VOLUME)),
            (SettingId::KeyVolume, volume_percent(DEFAULT_BUS_VOLUME)),
            (SettingId::BgmVolume, volume_percent(DEFAULT_BUS_VOLUME)),
            (SettingId::SystemVolume, volume_percent(DEFAULT_BUS_VOLUME)),
        ];
        for (id, start) in rows {
            let mut config = Config::default();
            assert_eq!(adjust(&mut config, id, -1), AdjustOutcome::Changed);
            assert_eq!(display_value(&config, id), format!("{}%", start - AUDIO_VOLUME_STEP_PERCENT), "{id:?} did not step down by one volume step");
        }
    }

    #[test]
    fn the_output_parameter_rows_ask_for_a_reopen_and_cycle_their_values() {
        let mut config = Config::default();
        assert_eq!(adjust(&mut config, SettingId::AudioBuffer, 1), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::AudioBuffer), AUDIO_BUFFER_FRAMES_CHOICES[0].to_string());
        assert!(config.audio.reopen_pending());

        config.audio.clear_reopen_pending();
        assert_eq!(adjust(&mut config, SettingId::AudioSampleRate, 1), AdjustOutcome::Changed);
        assert_eq!(display_value(&config, SettingId::AudioSampleRate), AUDIO_SAMPLE_RATE_HZ_CHOICES[0].to_string());
        assert!(config.audio.reopen_pending());

        config.audio.clear_reopen_pending();
        assert_eq!(adjust(&mut config, SettingId::AudioPolyphony, -1), AdjustOutcome::Changed);
        assert_eq!(config.audio.polyphony, rbms_audio::DEFAULT_MAX_VOICES - AUDIO_POLYPHONY_STEP_VOICES);
        assert!(config.audio.reopen_pending());
    }

    #[test]
    fn the_device_row_steps_through_the_enumerated_devices() {
        let mut config = Config::default();
        assert_eq!(adjust(&mut config, SettingId::AudioDevice, 1), AdjustOutcome::Action(SettingId::AudioDevice), "only the program can enumerate devices");
        let devices = vec!["Built-in Output".to_string()];
        assert!(step_audio_device(&mut config.audio, 1, &devices));
        assert_eq!(display_value(&config, SettingId::AudioDevice), "Built-in Output");
        assert!(config.audio.reopen_pending());
    }

    #[test]
    fn a_device_row_that_cannot_move_leaves_no_pending_reopen() {
        let mut config = Config::default();
        assert!(!step_audio_device(&mut config.audio, 1, &[]));
        assert_eq!(config.audio.device, None, "with no devices reported the row stays on the system default");
        assert!(!config.audio.reopen_pending(), "a row that did not move must not restart the output stream");
    }

    #[test]
    fn a_parameter_row_that_cannot_move_leaves_no_pending_reopen() {
        let mut config = Config::default();
        while config.audio.polyphony > AUDIO_POLYPHONY_MIN_VOICES {
            adjust(&mut config, SettingId::AudioPolyphony, -1);
        }
        config.audio.clear_reopen_pending();
        assert_eq!(adjust(&mut config, SettingId::AudioPolyphony, -1), AdjustOutcome::Unchanged);
        assert!(!config.audio.reopen_pending(), "a clamped row that did not move must not restart the output stream");
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
        let line = audio_status_text(Some(&downgraded), false);
        assert!(line.contains("48000"), "the opened rate must be visible next to the row value: {line}");
        assert!(line.contains("requested 96000 Hz"), "the downgrade reason must be shown: {line}");
        assert!(line.contains("Built-in Output"), "the opened device must be named: {line}");
    }

    #[test]
    fn the_audio_status_says_so_when_nothing_had_to_be_downgraded() {
        let line = audio_status_text(Some(&report(48_000, &[])), false);
        assert!(line.starts_with(AUDIO_MATCHED_STATUS), "got {line}");
        assert!(line.contains("48000"));
    }

    #[test]
    fn the_audio_status_reports_a_stream_that_never_opened() {
        assert_eq!(audio_status_text(None, true), AUDIO_UNAVAILABLE_STATUS);
        assert_eq!(audio_status_text(None, false), "", "before the first open attempt there is nothing to say");
    }

    #[test]
    fn the_options_the_engine_opens_with_follow_the_parameter_rows() {
        let mut config = Config::default();
        step_audio_device(&mut config.audio, 1, &["Studio Monitors".to_string()]);
        adjust(&mut config, SettingId::AudioBuffer, 1);
        adjust(&mut config, SettingId::AudioSampleRate, 2);
        adjust(&mut config, SettingId::AudioPolyphony, 1);
        let opts = engine_options(&config.audio);
        assert_eq!(opts.device_name.as_deref(), Some("Studio Monitors"));
        assert_eq!(opts.buffer_frames, Some(AUDIO_BUFFER_FRAMES_CHOICES[0]));
        assert_eq!(opts.sample_rate, Some(AUDIO_SAMPLE_RATE_HZ_CHOICES[1]));
        assert_eq!(opts.max_voices, rbms_audio::DEFAULT_MAX_VOICES + AUDIO_POLYPHONY_STEP_VOICES);
    }
}
