//! Headless companion to the player: inspect a chart, read the player's configuration, scan a song
//! folder, or query the local score book. The config, scan and scores subcommands drive
//! `rbms-config`, `rbms-library` and `rbms-store` without a window, which is how those crates get
//! exercised outside the GUI.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rbms_config::{Config, ConfigError, LoadOutcome, SettingTab, descriptor, display_value, tab_rows};
use rbms_library::{SongEntry, scan_folder};
use rbms_store::{ScoreBook, ScoreRecord};

const USAGE: &str = "usage:\n  \
    rbms-cli <chart.bms|.bme|.bml|.pms>   chart summary\n  \
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

/// Charts per mode, most charts first then by mode name, so the output is deterministic.
fn mode_histogram(songs: &[SongEntry]) -> Vec<(&'static str, usize)> {
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for s in songs {
        match counts.iter_mut().find(|(name, _)| *name == s.mode.name) {
            Some((_, n)) => *n += 1,
            None => counts.push((s.mode.name, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    counts
}

fn scan_line(entry: &SongEntry) -> String {
    format!("{}  {:<9} L{:<3} {}", entry.md5, entry.mode.name, entry.level, entry.title)
}

fn record_line(r: &ScoreRecord) -> String {
    format!("{:>7} / {:<7} lamp {:<3} combo {:<6} {}", r.ex_score, r.max_ex, r.clear, r.max_combo, r.title)
}

fn scan_command(dir: &Path) -> ExitCode {
    if !dir.is_dir() {
        eprintln!("not a folder: {}", dir.display());
        return ExitCode::FAILURE;
    }
    let count = AtomicUsize::new(0);
    let cancel = AtomicBool::new(false);
    let songs = scan_folder(dir, &count, &cancel);
    println!("folder     : {}", dir.display());
    println!("charts     : {}", songs.len());
    println!("read       : {}", count.load(Ordering::Relaxed));
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

    let src = rbms_parser::parse(&bytes);
    let mode = rbms_chart::detect_mode(&src, path);
    let model = rbms_chart::to_model(&src, mode);
    let length_s = model.timelines.last().map(|t| t.time_us).unwrap_or(0) as f64 / 1_000_000.0;

    println!("file       : {path}");
    println!("title      : {}", model.meta.title);
    println!("artist     : {}", model.meta.artist);
    println!("genre      : {}", model.meta.genre);
    println!("level      : {}", model.meta.play_level);
    println!("init bpm   : {}", model.init_bpm);
    println!("md5        : {}", model.md5);
    println!("sha256     : {}", model.sha256);
    println!("wav defs   : {}", src.wav.len());
    println!("measures   : {}", src.measures.len());
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

    fn song(md5: &str, mode: Mode, title: &str) -> SongEntry {
        SongEntry {
            path: PathBuf::from(format!("/songs/{title}.bms")),
            title: title.into(),
            subtitle: String::new(),
            artist: String::new(),
            genre: String::new(),
            maker: String::new(),
            level: "7".into(),
            difficulty: 3,
            init_bpm: 150.0,
            rank: 2,
            total: 300.0,
            mode,
            md5: md5.into(),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        }
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
        assert_eq!(rows, rbms_config::SETTING_COUNT, "every row of the settings table is printed exactly once");
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
}
