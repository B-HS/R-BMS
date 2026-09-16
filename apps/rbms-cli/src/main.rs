//! Headless companion to the player: inspect a chart, read the player's configuration, scan a song
//! folder, or query the local score book. The config, scan and scores subcommands drive
//! `rbms-config`, `rbms-library` and `rbms-store` without a window, which is how those crates get
//! exercised outside the GUI.

#![forbid(unsafe_code)]

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rbms_config::{Config, ConfigError, LoadOutcome, SettingTab, descriptor, display_value, tab_rows};
use rbms_library::songdb::{SongRow, mode_from_id};
use rbms_store::{ScoreBook, ScoreRecord};

const USAGE: &str = "usage:\n  \
    rbms-cli <chart.bms|.bme|.bml|.pms|.bmson>   chart summary\n  \
    rbms-cli scan <dir>                   scan a song folder\n  \
    rbms-cli config <settings.ron>        read (and migrate) the player configuration\n  \
    rbms-cli scores <scores.ron> [--md5 <md5>]   local score book";

/// Exit code for a usage error, matching the code the bare chart mode has always returned.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    run(std::env::args())
}

fn run(args: impl Iterator<Item = String>) -> ExitCode {
    let args: Vec<String> = args.skip(1).collect();
    match args.first().map(String::as_str) {
        None => usage_error("missing argument"),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("scan") => match args.get(1) {
            Some(dir) => scan_command(Path::new(dir)),
            None => usage_error("scan needs a folder"),
        },
        Some("config") => match args.get(1) {
            Some(path) => config_command(Path::new(path)),
            None => usage_error("config needs a settings.ron path"),
        },
        Some("scores") => match parse_scores_args(&args[1..]) {
            Ok((path, md5)) => scores_command(&path, md5.as_deref()),
            Err(e) => usage_error(&e),
        },
        Some(chart) => chart_command(chart),
    }
}

fn usage_error(reason: &str) -> ExitCode {
    eprintln!("{reason}\n{USAGE}");
    ExitCode::from(EXIT_USAGE)
}

/// `scores <path> [--md5 <md5>]`. The path is required and positional; the filter is optional.
fn parse_scores_args(rest: &[String]) -> Result<(PathBuf, Option<String>), String> {
    let mut path: Option<PathBuf> = None;
    let mut md5: Option<String> = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--md5" => md5 = Some(it.next().ok_or("--md5 needs a value")?.clone()),
            other if other.starts_with("--") => return Err(format!("unknown option: {other}")),
            other if path.is_none() => path = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    Ok((path.ok_or("scores needs a scores.ron path")?, md5))
}

fn mode_name(row: &SongRow) -> &'static str {
    mode_from_id(row.mode).map(|mode| mode.name).unwrap_or("UNKNOWN")
}

/// Charts per mode, most charts first then by mode name, so the output is deterministic.
fn mode_histogram(songs: &[SongRow]) -> Vec<(&'static str, usize)> {
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for row in songs {
        let name = mode_name(row);
        match counts.iter_mut().find(|(existing, _)| *existing == name) {
            Some((_, n)) => *n += 1,
            None => counts.push((name, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    counts
}

fn scan_line(row: &SongRow) -> String {
    format!("{}  {:<9} L{:<3} {}", row.md5, mode_name(row), row.level, row.title)
}

fn record_line(r: &ScoreRecord) -> String {
    format!("{:>7} / {:<7} lamp {:<3} combo {:<6} {}", r.ex_score, r.max_ex, r.clear, r.max_combo, r.title)
}

fn scan_database_path() -> PathBuf {
    std::env::temp_dir().join(format!("rbms-cli-scan-{}.sqlite", std::process::id()))
}

fn remove_scan_database(path: &Path) -> std::io::Result<()> {
    for temporary_path in [path.to_path_buf(), path.with_extension("sqlite-wal"), path.with_extension("sqlite-shm")] {
        match std::fs::remove_file(temporary_path) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn scan_rows(dir: &Path) -> Result<(Vec<SongRow>, usize), String> {
    let db_path = scan_database_path();
    remove_scan_database(&db_path).map_err(|e| format!("scan database not reset: {e}"))?;
    let scan_result = (|| {
        let mut db = rbms_library::songdb::SongDb::open(&db_path).map_err(|e| format!("scan database not opened: {e}"))?;
        db.migrate().map_err(|e| format!("scan database not migrated: {e}"))?;
        let request = rbms_library::scan::ScanRequest::new(vec![dir.to_string_lossy().into_owned()], true);
        rbms_library::scan::scan_into(&mut db, &request).map_err(|e| format!("scan failed: {e}"))?;
        let rows = db.all_songs().map_err(|e| format!("scan results not read: {e}"))?;
        Ok((rows, request.progress.counts().parsed))
    })();
    let cleanup_result = remove_scan_database(&db_path);

    match (scan_result, cleanup_result) {
        (Ok(rows), Ok(())) => Ok(rows),
        (Ok(_), Err(e)) => Err(format!("scan database cleanup failed: {e}")),
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(cleanup_error)) => Err(format!("{e}; scan database cleanup failed: {cleanup_error}")),
    }
}

fn scan_command(dir: &Path) -> ExitCode {
    if !dir.is_dir() {
        eprintln!("not a folder: {}", dir.display());
        return ExitCode::FAILURE;
    }
    let (songs, parsed) = match scan_rows(dir) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    println!("folder     : {}", dir.display());
    println!("charts     : {}", songs.len());
    println!("read       : {parsed}");
    for (mode, n) in mode_histogram(&songs) {
        println!("  {mode:<9} {n}");
    }
    for s in &songs {
        println!("{}", scan_line(s));
    }
    ExitCode::SUCCESS
}

fn scores_command(path: &Path, md5: Option<&str>) -> ExitCode {
    if !path.is_file() {
        eprintln!("no score book at {}", path.display());
        return ExitCode::FAILURE;
    }
    let book = ScoreBook::load(path);
    println!("file       : {}", path.display());
    println!("records    : {}", book.records().len());
    let Some(md5) = md5 else {
        let mut charts: Vec<String> = book.records().iter().map(|r| r.md5.to_ascii_lowercase()).collect();
        charts.sort_unstable();
        charts.dedup();
        println!("charts     : {}", charts.len());
        return ExitCode::SUCCESS;
    };
    let matched = book.for_md5(md5);
    println!("md5        : {md5}");
    println!("plays      : {}", matched.len());
    println!("best ex    : {}", book.best_ex_for_md5(md5).map(|v| v.to_string()).unwrap_or_else(|| "-".into()));
    println!("best lamp  : {}", book.best_clear_for_md5(md5).map(|v| v.to_string()).unwrap_or_else(|| "-".into()));
    for r in matched {
        println!("{}", record_line(r));
    }
    ExitCode::SUCCESS
}

/// Where a configuration came from: the schema it was migrated out of and the copy of the original
/// that migration kept, both as one line each so a migration is visible without diffing files.
fn migration_lines(outcome: &LoadOutcome) -> Vec<String> {
    let mut lines = vec![match outcome.migrated_from {
        Some(from) => format!("migrated   : schema version {from} -> {}", rbms_config::CURRENT_SCHEMA_VERSION),
        None => format!("migrated   : no (already schema version {})", outcome.config.schema_version),
    }];
    if let Some(backup) = &outcome.backup {
        lines.push(format!("kept       : {}", backup.display()));
    }
    lines
}

/// Every settings row of one tab as `label = value`, read straight out of the document.
fn tab_lines(config: &Config, tab: SettingTab) -> Vec<String> {
    tab_rows(tab, config).into_iter().map(|id| format!("  {:<22} {}", descriptor(id).label, display_value(config, id))).collect()
}

fn config_command(path: &Path) -> ExitCode {
    let outcome = match rbms_config::load(path) {
        Ok(outcome) => outcome,
        Err(e) => {
            eprintln!("config not loaded: {e}");
            if let ConfigError::Migrate { .. } = e {
                eprintln!("the file was written by a newer build and has been left untouched");
            }
            return ExitCode::FAILURE;
        }
    };
    println!("file       : {}", path.display());
    for line in migration_lines(&outcome) {
        println!("{line}");
    }
    for tab in SettingTab::ALL {
        println!("[{}]", tab.label());
        for line in tab_lines(&outcome.config, tab) {
            println!("{line}");
        }
    }
    ExitCode::SUCCESS
}

fn chart_command(path: &str) -> ExitCode {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let is_bmson = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(rbms_parser::bmson::EXTENSION));
    let (mode, model, wav_defs, measures) = if is_bmson {
        let chart = match rbms_parser::bmson::parse(&bytes) {
            Ok(chart) => chart,
            Err(e) => {
                eprintln!("bmson parse error: {e}");
                return ExitCode::FAILURE;
            }
        };
        let mode = chart.mode();
        let model = chart.to_model();
        let wav_defs = model.wavmap.len();
        (mode, model, wav_defs, 0)
    } else {
        let src = rbms_parser::parse(&bytes);
        let mode = rbms_chart::detect_mode(&src, path);
        let model = rbms_chart::to_model(&src, mode);
        (mode, model, src.wav.len(), src.measures.len())
    };
    let length_s = model.timelines.last().map(|t| t.time_us).unwrap_or(0) as f64 / 1_000_000.0;

    println!("file       : {path}");
    println!("title      : {}", model.meta.title);
    println!("artist     : {}", model.meta.artist);
    println!("genre      : {}", model.meta.genre);
    println!("level      : {}", model.meta.play_level);
    println!("init bpm   : {}", model.init_bpm);
    println!("md5        : {}", model.md5);
    println!("sha256     : {}", model.sha256);
    println!("wav defs   : {wav_defs}");
    println!("measures   : {measures}");
    println!("timelines  : {}", model.timelines.len());
    println!("mode       : {}", mode.name);
    println!("notes      : {}", rbms_chart::count_playable_notes(&model));
    println!("length     : {length_s:.1}s");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_model::Mode;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn scores_args_take_a_bare_path() {
        let (path, md5) = parse_scores_args(&args(&["/tmp/scores.ron"])).expect("a path alone is enough");
        assert_eq!(path, PathBuf::from("/tmp/scores.ron"));
        assert_eq!(md5, None);
    }

    #[test]
    fn scores_args_take_an_md5_filter_on_either_side_of_the_path() {
        let (path, md5) = parse_scores_args(&args(&["/tmp/s.ron", "--md5", "AABB"])).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/s.ron"));
        assert_eq!(md5.as_deref(), Some("AABB"));
        let (path, md5) = parse_scores_args(&args(&["--md5", "AABB", "/tmp/s.ron"])).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/s.ron"));
        assert_eq!(md5.as_deref(), Some("AABB"), "the option may come first");
    }

    #[test]
    fn scores_args_reject_a_missing_path_value_or_extra_positional() {
        assert!(parse_scores_args(&args(&[])).is_err(), "the path is required");
        assert!(parse_scores_args(&args(&["/tmp/s.ron", "--md5"])).is_err(), "--md5 needs a value");
        assert!(parse_scores_args(&args(&["/a", "/b"])).is_err(), "only one positional path is accepted");
        assert!(parse_scores_args(&args(&["--nope", "/a"])).is_err(), "unknown options are rejected");
    }

    fn song(md5: &str, mode: Mode, title: &str) -> SongRow {
        SongRow { title: title.into(), level: "7".into(), md5: md5.into(), mode: rbms_library::songdb::mode_id(mode), ..SongRow::default() }
    }

    #[test]
    fn mode_histogram_counts_and_orders_by_size_then_name() {
        let songs =
            vec![song("A", Mode::BEAT_7K, "one"), song("B", Mode::BEAT_14K, "two"), song("C", Mode::BEAT_7K, "three"), song("D", Mode::BEAT_5K, "four")];
        let hist = mode_histogram(&songs);
        assert_eq!(hist[0], (Mode::BEAT_7K.name, 2), "the busiest mode comes first");
        assert_eq!(hist.len(), 3);
        assert!(hist[1].0 < hist[2].0, "ties break on the mode name: {hist:?}");
    }

    #[test]
    fn config_is_a_subcommand_and_not_read_as_a_chart_path() {
        assert!(USAGE.contains("rbms-cli config"), "the usage line lists it: {USAGE}");
        let dir = std::env::temp_dir().join(format!("rbms-cli-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join("settings.ron");
        std::fs::write(&path, "(hispeed: 4.5, gauge: \"hard\")").expect("stage a pre-version file");

        assert_eq!(run(args(&["rbms-cli", "config", path.to_str().unwrap()]).into_iter()), ExitCode::SUCCESS);
        assert!(path.with_extension("ron.v0.bak").exists(), "the migration kept the original");
        assert_eq!(run(args(&["rbms-cli", "config"]).into_iter()), ExitCode::from(EXIT_USAGE), "the path is required");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_migrated_configuration_reports_the_schema_it_came_from_and_the_copy_it_kept() {
        let migrated = LoadOutcome { config: Config::default(), migrated_from: Some(0), backup: Some(PathBuf::from("/cfg/settings.ron.v0.bak")) };
        let lines = migration_lines(&migrated);
        assert!(lines[0].contains("schema version 0"), "{lines:?}");
        assert!(lines[1].contains("settings.ron.v0.bak"), "{lines:?}");

        let current = LoadOutcome { config: Config::default(), migrated_from: None, backup: None };
        let lines = migration_lines(&current);
        assert_eq!(lines.len(), 1, "nothing was migrated, so nothing was kept: {lines:?}");
        assert!(lines[0].contains("no"), "{lines:?}");
    }

    /// Every row a tab offers is printed with its value, and each is printed once. A row the table
    /// keeps but does not offer — one nothing acts on yet — is not printed, so the count here is the
    /// offered rows rather than the whole table.
    #[test]
    fn every_settings_tab_prints_its_rows_with_a_value() {
        let config = Config::default();
        let mut rows = 0;
        for tab in SettingTab::ALL {
            let lines = tab_lines(&config, tab);
            assert!(!lines.is_empty(), "{tab:?} has no rows");
            for line in &lines {
                assert!(line.split_whitespace().count() >= 2, "{tab:?} row has no value: {line}");
            }
            rows += lines.len();
        }
        let offered: usize = SettingTab::ALL.iter().map(|tab| rbms_config::tab_rows(*tab, &config).len()).sum();
        assert_eq!(rows, offered, "every row the settings table offers is printed exactly once");
    }

    #[test]
    fn mode_histogram_of_an_empty_scan_is_empty() {
        assert!(mode_histogram(&[]).is_empty());
    }

    #[test]
    fn scan_line_carries_the_md5_mode_and_title() {
        let line = scan_line(&song("DEADBEEF", Mode::BEAT_7K, "A Song"));
        assert!(line.starts_with("DEADBEEF"), "the md5 leads the line: {line}");
        assert!(line.contains(Mode::BEAT_7K.name), "the mode is shown: {line}");
        assert!(line.ends_with("A Song"), "the title closes the line: {line}");
    }

    #[test]
    fn scan_rows_reads_song_rows_from_the_temporary_database() {
        let dir = std::env::temp_dir().join(format!("rbms-cli-scan-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the fixture folder");
        std::fs::write(dir.join("song.bms"), "#TITLE CLI scan\n#BPM 120\n#00111:01\n").expect("write the fixture chart");

        let (rows, parsed) = scan_rows(&dir).expect("scan the fixture folder");
        assert_eq!(parsed, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "CLI scan");
        assert!(!scan_database_path().exists(), "the scan database is removed after reading rows");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chart_command_reads_a_bmson_document() {
        let dir = std::env::temp_dir().join(format!("rbms-cli-bmson-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the fixture folder");
        let path = dir.join("minimal.bmson");
        std::fs::write(&path, include_bytes!("../../../crates/rbms-parser/tests/bmson/minimal.bmson")).expect("write the bmson fixture");

        assert_eq!(chart_command(path.to_str().expect("a UTF-8 temporary path")), ExitCode::SUCCESS);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chart_command_rejects_an_invalid_bmson_document() {
        let dir = std::env::temp_dir().join(format!("rbms-cli-bmson-invalid-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the fixture folder");
        let path = dir.join("broken.bmson");
        std::fs::write(&path, "not JSON").expect("write the invalid bmson fixture");

        assert_eq!(chart_command(path.to_str().expect("a UTF-8 temporary path")), ExitCode::FAILURE);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
