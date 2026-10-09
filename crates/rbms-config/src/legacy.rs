use std::path::{Component, Path};

use rbms_chart::shuffle::NoteOption;
use serde::Deserialize;

use crate::audio::{AudioOptions, DEFAULT_BUS_VOLUME, DEFAULT_MASTER_VOLUME, DEFAULT_POLYPHONY_VOICES};
use crate::gauge::gauge_from_name;
use crate::schema::{
    CURRENT_SCHEMA_VERSION, Config, DEFAULT_HISPEED, DEFAULT_PLAYER_ID, DEFAULT_SKIN, DEFAULT_SKIN_FOLDER, DisplayOptions, JUDGE_RATE_DEFAULT_PERCENT,
    JudgeOptions, LANE_SHADE_MIN, LibraryOptions, NetworkOptions, PlayOptions, SkinOptions, TOTAL_FROM_CHART, TableSource,
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
                ..PlayOptions::default()
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
                ..DisplayOptions::default()
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
                ..NetworkOptions::default()
            },
            library: LibraryOptions { folders: Vec::new(), songs_folder: old.songs_folder, preview: old.preview, ..LibraryOptions::default() },
            skin: SkinOptions::default(),
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

/// Directory the retired skin bundle's first generation was installed into below the skin folder.
///
/// Later generations appended [`RETIRED_BUNDLE_GENERATION_INFIX`] and a number, and the DISPLAY
/// preset that switched the bundle on was this same name written in capitals with a space between
/// the words, so this one spelling is enough to recognise everything a configuration kept of it.
pub(crate) const RETIRED_BUNDLE_DIRECTORY: &str = "steel-neon";

/// What a later generation of the retired bundle put between the first generation's directory name
/// and its own number.
pub(crate) const RETIRED_BUNDLE_GENERATION_INFIX: &str = "-v";

/// What joins the words of the retired bundle's directory name.
const DIRECTORY_WORD_SEPARATOR: &str = "-";

/// What joins the same words in the DISPLAY preset the retired bundle was switched on with.
const PRESET_WORD_SEPARATOR: &str = " ";

/// Whether a DISPLAY preset is the one that switched the retired bundle on.
fn is_retired_preset(preset: &str) -> bool {
    preset.trim().replace(PRESET_WORD_SEPARATOR, DIRECTORY_WORD_SEPARATOR).eq_ignore_ascii_case(RETIRED_BUNDLE_DIRECTORY)
}

/// Whether a directory name is one a generation of the retired bundle was installed into.
fn is_retired_bundle_directory(name: &str) -> bool {
    let Some(generation) = name.strip_prefix(RETIRED_BUNDLE_DIRECTORY) else {
        return false;
    };
    generation.is_empty()
        || generation
            .strip_prefix(RETIRED_BUNDLE_GENERATION_INFIX)
            .is_some_and(|number| !number.is_empty() && number.bytes().all(|digit| digit.is_ascii_digit()))
}

/// The name a path component carries, for the components that name something.
fn normal_name(component: Component<'_>) -> Option<&str> {
    match component {
        Component::Normal(name) => name.to_str(),
        _ => None,
    }
}

/// The first directory of a path taken relative to the skin folder, which is `None` when the path
/// names a document lying directly in that folder.
fn first_directory_name(relative: &Path) -> Option<&str> {
    let mut names = relative.components().filter_map(normal_name);
    let first = names.next()?;
    names.next()?;
    Some(first)
}

/// The directory below [`DEFAULT_SKIN_FOLDER`], for a document reached through the default folder
/// rather than through one the player named.
fn directory_below_default_folder(path: &Path) -> Option<&str> {
    let names: Vec<&str> = path.components().filter_map(normal_name).collect();
    let at = names.iter().rposition(|name| *name == DEFAULT_SKIN_FOLDER)?;
    names.get(at + 1).copied().filter(|_| names.len() > at + 2)
}

/// Whether a stored document path points into a directory the retired bundle was installed into.
///
/// Only a directory sitting directly below the skin folder counts -- the one the player named, or
/// the default one beside the settings file -- because that is the only place an install ever wrote.
/// A document the player keeps anywhere else is theirs, whatever its folder is called.
fn is_retired_bundle_document(folder: Option<&str>, path: &str) -> bool {
    let path = Path::new(path);
    let below_named_folder = folder.map(Path::new).and_then(|folder| path.strip_prefix(folder).ok()).and_then(first_directory_name);
    below_named_folder.or_else(|| directory_below_default_folder(path)).is_some_and(is_retired_bundle_directory)
}

impl Config {
    /// Drop what a configuration kept of the skin bundle this player no longer ships.
    ///
    /// The DISPLAY preset that switched the bundle on falls back to the layout a fresh install
    /// starts on, and every screen still pointing at one of the bundle's documents goes back to the
    /// built-in screen together with the choices stored for that document. A document the player
    /// selected from anywhere else is left exactly as it was. The folders themselves are not
    /// touched: only the references to them go.
    pub fn retire_bundled_skin(&mut self) {
        if is_retired_preset(&self.display.skin) {
            self.display.skin = DEFAULT_SKIN.to_string();
        }
        let folder = self.skin.folder.clone();
        self.skin.selected.retain(|_, path| !is_retired_bundle_document(folder.as_deref(), path));
        self.skin.custom.retain(|path, _| !is_retired_bundle_document(folder.as_deref(), path));
    }
}
