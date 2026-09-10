//! What reaches the disk and what comes back: schema migration, a file this build is too old to
//! read, a file it cannot parse, and the round trip through `load` and `save`.

use super::*;

#[test]
fn a_versionless_document_migrates_from_the_flat_schema() {
    let (config, from) = migrate(r#"(hispeed: 3.0, gauge: "HARD", random: "MIRROR", judge_rate: 120, rivals: ["friend"])"#).expect("a v0 file migrates");
    assert_eq!(from, Some(LEGACY_SCHEMA_VERSION));
    assert!((config.play.hispeed - 3.0).abs() < 1e-9);
    assert_eq!(config.play.gauge, GaugeKind::Hard);
    assert_eq!(config.play.random, NoteOption::Mirror);
    assert_eq!(config.judge.judge_rate_key, [120; JUDGE_WIDTH_TIER_COUNT], "the one JUDGE WIDTH the flat file held covers every tier");
    assert_eq!(config.judge.judge_rate_scratch, [120; JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(config.network.rivals, vec!["friend".to_string()]);
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION, "the migrated document is stamped with the current schema");
}

#[test]
fn a_flat_file_written_before_the_audio_tab_still_migrates() {
    let (config, from) =
        migrate(r#"(hispeed: 3.0, gauge: "HARD", server_url: Some("https://ir.example/api"), player_id: "dj")"#).expect("a pre-audio file migrates");
    assert_eq!(from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(config.audio, AudioOptions::default(), "a file with no audio keys migrates to the shipped defaults");
    assert_eq!(config.network.player_id, "dj");
}

#[test]
fn a_current_document_needs_no_migration() {
    let (config, from) = migrate(&ron_of(&Config::default())).expect("a v1 file parses");
    assert_eq!(from, None);
    assert_eq!(config, Config::default());
}

#[test]
fn migration_clamps_what_the_old_file_held() {
    let (config, _) = migrate("(hispeed: 99.0, judge_rate: 4000, vol_master: 9.0, audio_polyphony: 100000)").expect("a v0 file migrates");
    assert!((config.play.hispeed - HISPEED_MAX).abs() < 1e-9);
    assert_eq!(config.judge.judge_rate_key, [JUDGE_RATE_MAX_PERCENT; JUDGE_WIDTH_TIER_COUNT]);
    assert!((config.audio.master - AUDIO_VOLUME_MAX_GAIN).abs() < 1e-6);
    assert_eq!(config.audio.polyphony, AUDIO_POLYPHONY_MAX_VOICES);
}

#[test]
fn a_newer_schema_is_refused_rather_than_migrated() {
    let error = migrate("(schema_version: 99)").expect_err("a newer schema cannot be read");
    match error {
        ConfigError::Migrate { from, .. } => assert_eq!(from, 99),
        other => panic!("expected a migrate error, got {other:?}"),
    }
}

#[test]
fn a_malformed_document_is_a_parse_error() {
    let error = migrate("definitely not ron )))").expect_err("nonsense is not a configuration");
    assert!(matches!(error, ConfigError::Parse(_)), "got {error:?}");
}

#[test]
fn load_missing_file_writes_and_returns_defaults() {
    let dir = temp_dir("missing");
    let path = dir.join("settings.ron");
    assert!(!path.exists());
    let outcome = load(&path).expect("a missing file is not an error");
    assert!(path.exists(), "load() of a missing settings file writes defaults out");
    assert_eq!(outcome.config, Config::default());
    assert_eq!(outcome.migrated_from, None);
    let again = load(&path).expect("the freshly written file loads");
    assert_eq!(again.config, Config::default(), "re-loading the freshly written file is stable");
    assert_eq!(again.migrated_from, None, "the file it just wrote is already current");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_malformed_file_backs_up_and_returns_defaults() {
    let dir = temp_dir("bad");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    std::fs::write(&path, "definitely not ron )))").expect("write");
    let outcome = load(&path).expect("a malformed file falls back rather than failing");
    assert_eq!(outcome.config, Config::default(), "malformed file => defaults");
    assert_eq!(outcome.backup.as_deref(), Some(path.with_extension("ron.bak").as_path()));
    assert!(path.with_extension("ron.bak").exists(), "malformed file backed up");
    assert!(!path.exists(), "the corrupt file is renamed away");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_newer_file_is_left_untouched_by_load() {
    let dir = temp_dir("newer");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    let text = "(schema_version: 99, play: (hispeed: 6.0))";
    std::fs::write(&path, text).expect("write");
    let error = load(&path).expect_err("a newer schema is reported");
    assert!(matches!(error, ConfigError::Migrate { from: 99, .. }), "got {error:?}");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), text, "the file a newer build wrote is not overwritten");
    assert!(!path.with_extension("ron.bak").exists(), "a readable newer file is not backed up either");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_then_load_round_trips_on_disk() {
    let dir = temp_dir("io");
    let path = dir.join("nested/settings.ron");
    let mut c = Config::default();
    c.play.hispeed = 5.5;
    c.library.folders = vec!["/songs".into()];
    c.library.tables = vec![TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into() }];
    for id in tab_rows(SettingTab::Judge, &c) {
        assert_eq!(adjust(&mut c, id, 1), AdjustOutcome::Changed, "{id:?} steps");
    }
    save(&c, &path).expect("save creates the parent directory");
    assert!(path.exists());
    let outcome = load(&path).expect("the saved file loads");
    assert_eq!(outcome.config, c, "every JUDGE row survives a save and a restart");
    assert_eq!(outcome.config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(outcome.migrated_from, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_absorbs_the_legacy_sibling_lists_of_a_versionless_file() {
    let dir = temp_dir("siblings");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    std::fs::write(&path, "(hispeed: 4.0)").expect("write");
    std::fs::write(dir.join(LEGACY_FOLDERS_FILE), r#"(folders: ["/songs", "/more"])"#).expect("write");
    std::fs::write(dir.join(LEGACY_TABLES_FILE), r#"(tables: [(name: "Insane", location: "https://example.com/insane.json")])"#).expect("write");

    let outcome = load(&path).expect("a v0 file with siblings loads");
    assert_eq!(outcome.migrated_from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(outcome.config.library.folders, vec!["/songs".to_string(), "/more".into()]);
    assert_eq!(outcome.config.library.tables.len(), 1);
    assert!(dir.join(LEGACY_FOLDERS_FILE).exists(), "the original list is left on disk so an older build still runs");
    assert!(dir.join(LEGACY_TABLES_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_current_file_ignores_the_legacy_sibling_lists() {
    let dir = temp_dir("current_siblings");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    save(&Config::default(), &path).expect("save");
    std::fs::write(dir.join(LEGACY_FOLDERS_FILE), r#"(folders: ["/removed"])"#).expect("write");

    let outcome = load(&path).expect("a v1 file loads");
    assert!(outcome.config.library.folders.is_empty(), "a folder removed after the migration does not come back");
    let _ = std::fs::remove_dir_all(&dir);
}
