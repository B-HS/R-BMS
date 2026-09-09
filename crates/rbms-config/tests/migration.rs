//! Migration of real pre-version configuration files.
//!
//! Every case starts from a committed fixture copied into a scratch directory under the names the
//! loader looks for, so the test exercises the same sibling-file discovery a user's `~/.config/rbms`
//! goes through.

use std::path::{Path, PathBuf};

use rbms_chart::shuffle::NoteOption;
use rbms_config::{
    AudioOptions, CURRENT_SCHEMA_VERSION, Config, ConfigError, DEFAULT_PLAYER_ID, LEGACY_FOLDERS_FILE, LEGACY_SCHEMA_VERSION, LEGACY_TABLES_FILE, load, save,
};
use rbms_judge::GaugeKind;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {} is committed: {e}", path.display()))
}

/// A scratch config directory holding `settings.ron` (and optionally the legacy sibling lists)
/// taken from the named fixtures.
fn staged(tag: &str, settings: &str, folders: Option<&str>, tables: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms_config_fixture_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    std::fs::write(dir.join("settings.ron"), fixture(settings)).expect("stage settings");
    if let Some(name) = folders {
        std::fs::write(dir.join(LEGACY_FOLDERS_FILE), fixture(name)).expect("stage folders");
    }
    if let Some(name) = tables {
        std::fs::write(dir.join(LEGACY_TABLES_FILE), fixture(name)).expect("stage tables");
    }
    dir
}

#[test]
fn a_default_flat_file_migrates_to_the_shipped_defaults() {
    let dir = staged("default", "settings-v0-default.ron", None, None);
    let outcome = load(&dir.join("settings.ron")).expect("the fixture loads");
    assert_eq!(outcome.migrated_from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(outcome.config, Config::default(), "a file written by an untouched install migrates to the shipped defaults");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_flat_field_lands_in_its_group() {
    let dir = staged("full", "settings-v0-full.ron", None, None);
    let c = load(&dir.join("settings.ron")).expect("the fixture loads").config;

    assert!((c.play.hispeed - 3.25).abs() < 1e-9);
    assert_eq!(c.play.gauge, GaugeKind::Hard);
    assert!((c.play.lift - 0.15).abs() < 1e-6);
    assert!((c.play.cover - 0.4).abs() < 1e-6);
    assert!(c.play.scratch_left && c.play.scratch_auto);
    assert!(!c.play.autoplay);
    assert_eq!(c.play.random, NoteOption::Mirror);
    assert!(c.play.constant_speed);
    assert!((c.play.total_override - 320.0).abs() < 1e-9);
    assert!(!c.play.auto_replay);

    assert_eq!(c.judge.offset_ms, -33);
    assert!(c.judge.auto_offset);
    assert_eq!(c.judge.judge_rate, 150);

    assert!(!c.display.bga);
    assert_eq!(c.display.skin, "WIDE", "the skin name is uppercased on the way in");
    assert!(c.display.debug);
    assert_eq!(c.display.font_path.as_deref(), Some("/tmp/f.ttf"));
    assert!(!c.display.score_graph);
    assert!(!c.display.replay_analysis);

    assert!(!c.library.preview);
    assert_eq!(c.library.songs_folder.as_deref(), Some("/songs"));

    assert_eq!(c.network.server_url.as_deref(), Some("https://ir.example/api"));
    assert_eq!(c.network.player_id, "dj");
    assert_eq!(c.network.ir_token.as_deref(), Some("tok-123"), "a signed-in install stays signed in across the migration");
    assert_eq!(c.network.ir_login_id.as_deref(), Some("dj"));
    assert_eq!(c.network.ir_email.as_deref(), Some("dj@example.test"));
    assert!(c.network.sync_settings);
    assert!(!c.network.auto_upload_replay);
    assert_eq!(c.network.rivals, vec!["rivalone".to_string(), "rivaltwo".into()]);

    assert_eq!(c.audio.device.as_deref(), Some("Studio Monitors"));
    assert_eq!(c.audio.buffer_frames, Some(384));
    assert_eq!(c.audio.sample_rate, Some(96_000));
    assert_eq!(c.audio.polyphony, 256);
    assert!((c.audio.master - 0.85).abs() < 1e-6);
    assert!((c.audio.key - 0.7).abs() < 1e-6);
    assert!((c.audio.bg - 0.35).abs() < 1e-6);
    assert!((c.audio.system - 0.15).abs() < 1e-6);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_partial_flat_file_defaults_everything_it_omits() {
    let dir = staged("partial", "settings-v0-partial.ron", None, None);
    let c = load(&dir.join("settings.ron")).expect("the fixture loads").config;
    assert!((c.play.hispeed - 7.5).abs() < 1e-9);
    assert_eq!(c.play.gauge, GaugeKind::Easy);
    assert_eq!(c.judge.judge_rate, 120);
    assert_eq!(c.network.rivals, vec!["friend".to_string()]);

    let defaults = Config::default();
    assert_eq!(c.play.random, defaults.play.random, "an omitted field keeps the shipped value");
    assert_eq!(c.library.preview, defaults.library.preview);
    assert_eq!(c.network.player_id, DEFAULT_PLAYER_ID);
    assert_eq!(c.audio, defaults.audio, "a file older than the audio tab migrates to the shipped audio defaults");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_key_in_a_flat_file_is_ignored() {
    let dir = staged("unknown", "settings-v0-unknown-key.ron", None, None);
    let c = load(&dir.join("settings.ron")).expect("an unknown key does not fail the load").config;
    assert!((c.play.hispeed - 4.5).abs() < 1e-9);
    assert_eq!(c.play.gauge, GaugeKind::ExHard);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_legacy_folder_and_table_files_are_absorbed_into_the_library() {
    let dir = staged("lists", "settings-v0-default.ron", Some("folders-v0.ron"), Some("tables-v0.ron"));
    let outcome = load(&dir.join("settings.ron")).expect("the fixtures load");
    assert_eq!(outcome.migrated_from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(outcome.config.library.folders, vec!["/songs/main".to_string(), "/songs/extra".into()], "order is kept");
    assert_eq!(outcome.config.library.tables.len(), 2);
    assert_eq!(outcome.config.library.tables[0].name, "Insane");
    assert_eq!(outcome.config.library.tables[0].location, "https://example.com/insane.json");
    assert_eq!(outcome.config.library.tables[1].name, "", "an unnamed source keeps its empty name");
    assert_eq!(outcome.config.library.tables[1].location, "/local/table.json");
    assert!(dir.join(LEGACY_FOLDERS_FILE).exists(), "the originals stay on disk so an older build can still be run");
    assert!(dir.join(LEGACY_TABLES_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_from_a_newer_build_is_refused_and_left_alone() {
    let dir = staged("newer", "settings-v99.ron", None, None);
    let path = dir.join("settings.ron");
    let before = std::fs::read_to_string(&path).expect("staged file");
    let error = load(&path).expect_err("a newer schema is refused");
    match error {
        ConfigError::Migrate { from, .. } => assert_eq!(from, 99),
        other => panic!("expected a migrate error, got {other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&path).expect("read"), before, "the newer file is neither rewritten nor backed up");
    assert!(!path.with_extension("ron.bak").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_migrated_configuration_saves_and_reloads_unchanged() {
    let dir = staged("roundtrip", "settings-v0-full.ron", Some("folders-v0.ron"), Some("tables-v0.ron"));
    let path = dir.join("settings.ron");
    let migrated = load(&path).expect("the fixtures load").config;
    save(&migrated, &path).expect("save");

    let reloaded = load(&path).expect("the saved file loads");
    assert_eq!(reloaded.config, migrated, "save then load is the identity");
    assert_eq!(reloaded.migrated_from, None, "the saved file is already current");
    assert_eq!(reloaded.config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_ne!(migrated.audio, AudioOptions::default(), "the fixture really did carry non-default audio through");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_migrated_file_is_kept_under_the_schema_it_was_written_in() {
    let dir = staged("keepv0", "settings-v0-full.ron", None, None);
    let path = dir.join("settings.ron");
    let original = std::fs::read_to_string(&path).expect("staged file");

    let outcome = load(&path).expect("the fixture loads");
    assert_eq!(outcome.migrated_from, Some(LEGACY_SCHEMA_VERSION));
    let backup = outcome.backup.clone().expect("a migrated file is copied aside before it is overwritten");
    assert_eq!(backup, path.with_extension("ron.v0.bak"), "the copy is named after the schema it holds");
    assert_eq!(std::fs::read_to_string(&backup).expect("read the copy"), original, "the copy is the file byte for byte");

    save(&outcome.config, &path).expect("the caller writes the current schema over the original");
    assert_eq!(std::fs::read_to_string(&backup).expect("read the copy"), original, "the overwrite does not disturb the copy");
    assert_ne!(std::fs::read_to_string(&path).expect("read"), original, "the original really was replaced");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_already_on_the_current_schema_is_not_copied_aside() {
    let dir = staged("nocopy", "settings-v0-default.ron", None, None);
    let path = dir.join("settings.ron");
    save(&Config::default(), &path).expect("write a current-schema file");
    let outcome = load(&path).expect("the current-schema file loads");
    assert_eq!(outcome.migrated_from, None);
    assert_eq!(outcome.backup, None, "nothing was migrated, so there is nothing to keep");
    assert!(!path.with_extension("ron.v0.bak").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_configuration_that_cannot_be_read_is_reported_rather_than_replaced_by_the_defaults() {
    let dir = staged("unreadable", "settings-v0-default.ron", None, None);
    let path = dir.join("settings.ron");
    let error = load(&dir).expect_err("a directory cannot be read as a configuration file");
    match error {
        ConfigError::Read(_) => {}
        other => panic!("expected a read error, got {other:?}"),
    }
    assert!(path.exists(), "the failing read leaves the directory's contents alone");
    let _ = std::fs::remove_dir_all(&dir);
}
