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
    assert_eq!(outcome.config, Config::first_run());
    assert_eq!(outcome.config.network.server_url.as_deref(), Some(DEFAULT_SERVER_URL), "a first run is signed up to the R-BMS server");
    assert_eq!(outcome.migrated_from, None);
    let again = load(&path).expect("the freshly written file loads");
    assert_eq!(again.config, Config::first_run(), "re-loading the freshly written file is stable");
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
    c.library.tables = vec![TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into(), group: String::new() }];
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

/// The rows the play menu lists are written where the settings screen wrote them: every one of them
/// stepped away from what it ships as survives a save and a restart, under the schema the file was
/// already on.
#[test]
fn every_row_of_the_play_menu_survives_a_save_and_a_restart() {
    let dir = temp_dir("play-menu");
    let path = dir.join("settings.ron");
    let mut c = Config::default();
    for tab in SettingTab::PLAY_MENU {
        for id in tab_rows(tab, &c) {
            assert_eq!(adjust(&mut c, id, 1), AdjustOutcome::Changed, "{id:?} steps");
        }
    }
    assert_ne!(c.play, Config::default().play);
    save(&c, &path).expect("save");
    let outcome = load(&path).expect("the saved file loads");
    assert_eq!(outcome.config, c, "a row of the play menu did not come back as it was written");
    assert_eq!(outcome.migrated_from, None, "moving a row to another screen changed the schema of the file");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The bits of a Unix file mode that say who may read or write the file, and the two modes the tests
/// tell apart.
#[cfg(unix)]
const PERMISSION_BITS: u32 = 0o777;
#[cfg(unix)]
const OWNER_ONLY: u32 = 0o600;
#[cfg(unix)]
const OPEN_TO_OTHERS: u32 = 0o644;

#[cfg(unix)]
fn mode_of(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).expect("the file is there").permissions().mode() & PERMISSION_BITS
}

#[cfg(unix)]
fn open_to_others(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(OPEN_TO_OTHERS)).expect("opened");
}

/// The file holds the bearer token, so a save leaves it to its owner, whether the file is new or
/// replaces one an older build left open to other users.
#[cfg(unix)]
#[test]
fn a_saved_configuration_is_readable_by_its_owner_alone() {
    let dir = temp_dir("private-save");
    let path = dir.join("settings.ron");
    let mut config = Config::default();
    config.network.ir_token = Some("secret-token".into());
    save(&config, &path).expect("save");
    assert_eq!(mode_of(&path), OWNER_ONLY, "a file with a bearer token was written open to other users");

    open_to_others(&path);
    save(&config, &path).expect("save over a file others could read");
    assert_eq!(mode_of(&path), OWNER_ONLY, "a save kept the mode of the file it replaced");
    assert_eq!(load(&path).expect("loads").config.network.ir_token.as_deref(), Some("secret-token"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The copy of a file a migration keeps holds whatever the file held, token included.
#[cfg(unix)]
#[test]
fn the_copy_a_migration_keeps_is_readable_by_its_owner_alone() {
    let dir = temp_dir("private-migration");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    let flat = r#"(hispeed: 4.0, ir_token: Some("secret-token"))"#;
    std::fs::write(&path, flat).expect("write");
    open_to_others(&path);

    let outcome = load(&path).expect("a v0 file loads");
    let backup = outcome.backup.expect("the migration kept the original");
    assert_eq!(std::fs::read_to_string(&backup).expect("reads"), flat, "the copy is the original");
    assert_eq!(mode_of(&backup), OWNER_ONLY, "the copy of a file with a token was left open to other users");
    assert_eq!(mode_of(&path), OWNER_ONLY, "loading closes the file an older build left open, before it is saved again");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A copy an older build made is never written again, so the load that finds it is what closes it.
#[cfg(unix)]
#[test]
fn loading_closes_the_copies_an_older_build_left_open_to_other_users() {
    let dir = temp_dir("private-old-copies");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    save(&Config::default(), &path).expect("save");
    let copies = [path.with_extension("ron.bak"), path.with_extension("ron.v2.bak"), path.with_extension("ron.v5.bak")];
    for copy in &copies {
        std::fs::write(copy, "(ir_token: Some(\"secret-token\"))").expect("write");
        open_to_others(copy);
    }
    let unrelated = dir.join("keyconfig.ron");
    std::fs::write(&unrelated, "()").expect("write");
    open_to_others(&unrelated);
    open_to_others(&path);

    load(&path).expect("loads");

    for copy in &copies {
        assert_eq!(mode_of(copy), OWNER_ONLY, "{} was left open to other users", copy.display());
    }
    assert_eq!(mode_of(&path), OWNER_ONLY);
    assert_eq!(mode_of(&unrelated), OPEN_TO_OTHERS, "a file that holds no credential is not this crate's to change");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The unreadable file moved aside is the file that held the token, whole.
#[cfg(unix)]
#[test]
fn an_unreadable_file_moved_aside_is_closed_to_other_users() {
    let dir = temp_dir("private-bad");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    std::fs::write(&path, "ir_token: Some(\"secret-token\") definitely not ron )))").expect("write");
    open_to_others(&path);

    let outcome = load(&path).expect("a malformed file falls back");
    let backup = outcome.backup.expect("the file was moved aside");
    assert_eq!(mode_of(&backup), OWNER_ONLY);
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

/// A row that holds a number, set through the count of its steps, is written in the field it
/// always was and in no new one: the file a build with sliders writes has the keys of the file a
/// build without them wrote, and reads back as the configuration it was written from.
#[test]
fn a_number_set_by_its_steps_is_written_in_the_fields_the_file_always_had() {
    let keys = |text: &str| text.lines().map(|line| line.split(':').next().unwrap_or_default().trim().to_string()).collect::<Vec<String>>();
    let shipped = ron_of(&Config::default());
    let mut config = Config::default();
    let set = [
        (SettingId::HiSpeed, 137),
        (SettingId::HiSpeedStep, 7),
        (SettingId::LaneCover, 253),
        (SettingId::Lift, 1),
        (SettingId::Hidden, 999),
        (SettingId::LaneCoverFineStep, 33),
        (SettingId::JudgeOffset, -17),
        (SettingId::JudgeWidthKeyGreat, 123),
        (SettingId::LongNoteMargin, 77),
        (SettingId::Total, 321),
        (SettingId::JudgeTextY, 41),
        (SettingId::PreviewVolume, 9),
        (SettingId::PreviewFade, 137),
        (SettingId::MasterVolume, 63),
        (SettingId::AudioPolyphony, 500),
    ];
    for (id, steps) in set {
        assert_eq!(set_number_steps(&mut config, id, steps), AdjustOutcome::Changed, "{id:?}");
    }
    let written = ron_of(&config);
    assert_ne!(written, shipped);
    assert_eq!(keys(&written), keys(&shipped), "a number set by its steps added a key to the file, or took one away");
    let (restored, from) = migrate(&written).expect("the file reads back");
    assert_eq!(from, None, "the file a slider wrote needs migrating");
    for (id, steps) in set {
        assert_eq!(number_steps(&restored, id), Some(steps), "{id:?} did not survive the file");
        assert_eq!(display_value(&restored, id), display_value(&config, id), "{id:?}");
    }
}

/// A file written before the rows were counted in steps reads as it did: a cover on a whole
/// percent, a speed on a quarter and a volume on a twentieth each stand on a step of their own, and
/// reading them changes nothing that is stored.
#[test]
fn a_file_written_before_the_steps_reads_on_them() {
    let (config, _) = migrate("(hispeed: 3.25, cover: 0.3, lift: 0.15, vol_master: 0.85, judge_rate: 120)").expect("a v0 file migrates");
    assert_eq!(number_steps(&config, SettingId::HiSpeed), Some(325));
    assert_eq!((number_steps(&config, SettingId::LaneCover), number_steps(&config, SettingId::Lift)), (Some(300), Some(150)));
    assert_eq!((display_value(&config, SettingId::LaneCover).as_str(), display_value(&config, SettingId::Lift).as_str()), ("30.0%", "15.0%"));
    assert_eq!(number_steps(&config, SettingId::MasterVolume), Some(85));
    assert_eq!(number_steps(&config, SettingId::JudgeWidthKeyPGreat), Some(120));
    assert_eq!(display_value(&config, SettingId::HiSpeed), "3.25");
    let before = config.clone();
    let mut read = config;
    for id in SettingId::ALL {
        if let Some(steps) = number_steps(&read, id) {
            assert_eq!(set_number_steps(&mut read, id, steps), AdjustOutcome::Unchanged, "{id:?} was rewritten by being put where it reads");
        }
    }
    assert_eq!(read, before);
}

/// A file from before the server had a default names none for every player who never typed one in,
/// so it is given the default once. A server it names is kept, and from the current schema on an
/// empty row is the player's choice to play offline.
#[test]
fn an_older_file_without_a_server_is_given_the_default_once_and_a_later_choice_is_kept() {
    let (older, from) = migrate(&format!("(schema_version: {NO_DEFAULT_SERVER_SCHEMA_VERSION})")).expect("a schema 5 file migrates");
    assert_eq!(from, Some(NO_DEFAULT_SERVER_SCHEMA_VERSION));
    assert_eq!(older.network.server_url.as_deref(), Some(DEFAULT_SERVER_URL));

    let named = format!(r#"(schema_version: {NO_DEFAULT_SERVER_SCHEMA_VERSION}, network: (server_url: Some("https://ir.example/api")))"#);
    assert_eq!(migrate(&named).expect("migrates").0.network.server_url.as_deref(), Some("https://ir.example/api"), "a server the file names stays");

    let (flat, _) = migrate("(hispeed: 3.0)").expect("a flat file migrates");
    assert_eq!(flat.network.server_url.as_deref(), Some(DEFAULT_SERVER_URL), "a file older still is given it too");

    let mut offline = older;
    offline.network.server_url = None;
    let (kept, from) = migrate(&ron_of(&offline)).expect("a current file parses");
    assert_eq!(from, None);
    assert_eq!(kept.network.server_url, None, "the player's choice to play offline was undone");
}

/// The tables already given are remembered in the file; a file that predates the list has given
/// none, and every other value of a schema 5 file reads as it always did.
#[test]
fn the_tables_already_given_are_kept_in_the_file() {
    let mut config = Config::default();
    config.library.seeded_default_tables = vec![INSANE_TABLE_ID.to_string()];
    let back: Config = ron::from_str(&ron_of(&config)).expect("a config round-trips");
    assert_eq!(back.library.seeded_default_tables, vec![INSANE_TABLE_ID.to_string()]);

    let older = format!(
        r#"(schema_version: {NO_DEFAULT_SERVER_SCHEMA_VERSION}, play: (hispeed: 2.5), library: (tables: [(name: "", location: "https://tables.test/t")]))"#
    );
    let (config, _) = migrate(&older).expect("a schema 5 file migrates");
    assert!(config.library.seeded_default_tables.is_empty(), "a file that predates the list has given no table");
    assert_eq!(config.library.tables.len(), 1);
    assert!((config.play.hispeed - 2.5).abs() < 1e-9);
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
}

/// A file written before the DISCORD STATUS row reads with the row on, a file that switched it off
/// keeps it off, and the row steps the value it shows.
#[test]
fn a_file_written_before_the_discord_row_reads_with_it_on() {
    let older = format!(r#"(schema_version: {CURRENT_SCHEMA_VERSION}, network: (auto_upload_trace: false))"#);
    let (config, from) = migrate(&older).expect("a current file without the row parses");
    assert_eq!(from, None);
    assert!(config.network.discord_status, "a file from before the row reads as on");
    assert!(!config.network.auto_upload_trace, "the rest of the file still reads as written");

    let mut off = Config::default();
    off.network.discord_status = false;
    let text = ron_of(&off);
    assert!(text.contains("discord_status: false"), "the value is written out even when it is the default's opposite");
    let back: Config = ron::from_str(&text).expect("a config round-trips");
    assert!(!back.network.discord_status, "a switched-off row came back on");

    let mut stepped = Config::default();
    assert_eq!(display_value(&stepped, SettingId::DiscordStatus), "ON");
    assert_eq!(adjust(&mut stepped, SettingId::DiscordStatus, 1), AdjustOutcome::Changed);
    assert!(!stepped.network.discord_status);
    assert_eq!(display_value(&stepped, SettingId::DiscordStatus), "OFF");
    assert_eq!(SETTING_COUNT, SettingId::ALL.len());
    assert_eq!(SettingId::ALL.last(), Some(&SettingId::DiscordStatus), "the row is added at the end of the list");
}
