use rbms_chart::shuffle::NoteOption;
use serde::Deserialize;

use crate::audio::{AudioOptions, DEFAULT_BUS_VOLUME, DEFAULT_MASTER_VOLUME, DEFAULT_POLYPHONY_VOICES};
use crate::gauge::gauge_from_name;
use crate::schema::{
    CURRENT_SCHEMA_VERSION, Config, DEFAULT_HISPEED, DEFAULT_PLAYER_ID, DEFAULT_SKIN, DisplayOptions, JUDGE_RATE_DEFAULT_PERCENT, JudgeOptions, LANE_SHADE_MIN,
    LibraryOptions, NetworkOptions, PlayOptions, TOTAL_FROM_CHART, TableSource,
};

/// File name of the pre-migration folder list, read once next to `settings.ron` when a version-less
/// settings file is migrated.
pub const LEGACY_FOLDERS_FILE: &str = "folders.ron";

/// File name of the pre-migration difficulty-table list, read the same way.
pub const LEGACY_TABLES_FILE: &str = "tables.ron";

/// The flat, version-less `settings.ron` every build before the unified configuration wrote.
///
/// Kept as its own type so migration reads the old file exactly as it was written; the defaults
/// below are the ones that file was created with, which is what a partially written file falls back
/// to. Deserialize only: this build never writes the old shape again.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct LegacyV0 {
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
    pub preview: bool,
    pub songs_folder: Option<String>,
    pub server_url: Option<String>,
    pub player_id: String,
    pub ir_token: Option<String>,
    pub ir_login_id: Option<String>,
    pub ir_email: Option<String>,
    pub sync_settings: bool,
    pub auto_upload_replay: bool,
    pub rivals: Vec<String>,
    pub audio_device: Option<String>,
    pub audio_buffer_frames: Option<u32>,
    pub audio_sample_rate: Option<u32>,
    pub audio_polyphony: usize,
    pub vol_master: f32,
    pub vol_key: f32,
    pub vol_bg: f32,
    pub vol_system: f32,
}

impl Default for LegacyV0 {
    fn default() -> Self {
        LegacyV0 {
            hispeed: DEFAULT_HISPEED,
            gauge: "NORMAL".into(),
            lift: LANE_SHADE_MIN,
            cover: LANE_SHADE_MIN,
            scratch_left: false,
            scratch_auto: false,
            autoplay: true,
            random: "OFF".into(),
            constant_speed: false,
            offset_ms: 0,
            auto_offset: false,
            judge_rate: JUDGE_RATE_DEFAULT_PERCENT,
            total_override: TOTAL_FROM_CHART,
            bga: true,
            skin: DEFAULT_SKIN.into(),
            auto_replay: true,
            debug: false,
            font_path: None,
            score_graph: true,
            replay_analysis: true,
            preview: true,
            songs_folder: None,
            server_url: None,
            player_id: DEFAULT_PLAYER_ID.into(),
            ir_token: None,
            ir_login_id: None,
            ir_email: None,
            sync_settings: false,
            auto_upload_replay: true,
            rivals: Vec::new(),
            audio_device: None,
            audio_buffer_frames: None,
            audio_sample_rate: None,
            audio_polyphony: DEFAULT_POLYPHONY_VOICES,
            vol_master: DEFAULT_MASTER_VOLUME,
            vol_key: DEFAULT_BUS_VOLUME,
            vol_bg: DEFAULT_BUS_VOLUME,
            vol_system: DEFAULT_BUS_VOLUME,
        }
    }
}

impl From<LegacyV0> for Config {
    fn from(old: LegacyV0) -> Config {
        Config {
            schema_version: CURRENT_SCHEMA_VERSION,
            play: PlayOptions {
                autoplay: old.autoplay,
                hispeed: old.hispeed,
                constant_speed: old.constant_speed,
                random: NoteOption::from_str(&old.random),
                gauge: gauge_from_name(&old.gauge),
                lift: old.lift,
                cover: old.cover,
                scratch_left: old.scratch_left,
                scratch_auto: old.scratch_auto,
                total_override: old.total_override,
                auto_replay: old.auto_replay,
            },
            judge: {
                let mut judge = JudgeOptions { offset_ms: old.offset_ms, auto_offset: old.auto_offset, ..JudgeOptions::default() };
                judge.spread_uniform_judge_rate(old.judge_rate);
                judge
            },
            display: DisplayOptions {
                bga: old.bga,
                skin: old.skin,
                font_path: old.font_path,
                score_graph: old.score_graph,
                replay_analysis: old.replay_analysis,
                debug: old.debug,
            },
            audio: AudioOptions {
                device: old.audio_device,
                buffer_frames: old.audio_buffer_frames,
                sample_rate: old.audio_sample_rate,
                polyphony: old.audio_polyphony,
                master: old.vol_master,
                key: old.vol_key,
                bg: old.vol_bg,
                system: old.vol_system,
                ..AudioOptions::default()
            },
            network: NetworkOptions {
                server_url: old.server_url,
                player_id: old.player_id,
                ir_token: old.ir_token,
                ir_login_id: old.ir_login_id,
                ir_email: old.ir_email,
                sync_settings: old.sync_settings,
                auto_upload_replay: old.auto_upload_replay,
                rivals: old.rivals,
            },
            library: LibraryOptions { folders: Vec::new(), songs_folder: old.songs_folder, preview: old.preview, tables: Vec::new() },
        }
    }
}

/// The pre-migration `folders.ron`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyFolders {
    folders: Vec<String>,
}

/// The pre-migration `tables.ron`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyTables {
    tables: Vec<TableSource>,
}

/// Fold the pre-migration sibling lists into `config.library`, keeping what the configuration
/// already holds and appending only entries it does not have yet. Unreadable siblings are skipped:
/// the lists are a convenience, never a reason to fail the load.
///
/// The originals are left on disk so an older build can still be run against them.
pub fn merge_legacy_lists(config: &mut Config, folders_ron: Option<&str>, tables_ron: Option<&str>) {
    if let Some(parsed) = folders_ron.and_then(|raw| ron::from_str::<LegacyFolders>(raw).ok()) {
        for folder in parsed.folders {
            if !config.library.folders.contains(&folder) {
                config.library.folders.push(folder);
            }
        }
    }
    if let Some(parsed) = tables_ron.and_then(|raw| ron::from_str::<LegacyTables>(raw).ok()) {
        for source in parsed.tables {
            if !config.library.tables.iter().any(|t| t.location == source.location) {
                config.library.tables.push(source);
            }
        }
    }
}
