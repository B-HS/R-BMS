use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use rbms_library::songdb::PARSER_VERSION;
use rbms_model::Mode;
use serde_json::{Value, json};

use super::args::{DEFAULT_MODE, mode_flag, parse};
use super::document::{Extracted, FORMAT, VERSION, extract};
use super::walk::charts_under;
use super::{COVERAGE_FILE_NAME, PatternArgs, PatternReport, export, run};

const PLAIN: &[&str] =
    &["#TITLE Plain", "#ARTIST Tester", "#BPM 120", "#PLAYLEVEL 5", "#RANK 2", "#TOTAL 200", "#WAV01 a.wav", "#00011:01000100", "#00016:01", "#00019:0001"];
const PLAIN_MD5: &str = "5ad4c29f81605cb0902be8822a189834";
const PLAIN_SHA256: &str = "d02ea2b546cf7d8c262d1b93144e210c92b262986f2d439c2ecead77f22ff93c";

const LONG: &[&str] = &["#TITLE Long", "#BPM 120", "#WAV01 a.wav", "#00151:01000001", "#000D3:01", "#00018:01"];
const LONG_MD5: &str = "08a43deefc828fca5a84b5a5d445e1e6";

const CHARGE: &[&str] = &["#TITLE Long", "#BPM 120", "#LNMODE 2", "#WAV01 a.wav", "#00151:01000001", "#000D3:01", "#00018:01"];
const CHARGE_MD5: &str = "e28fd1e0517572c3f2c324576b0b8dce";

const TEMPO: &[&str] = &[
    "#TITLE Tempo",
    "#BPM 120",
    "#BPM02 240",
    "#STOP03 96",
    "#SCROLL04 2",
    "#WAV01 a.wav",
    "#00011:01",
    "#00019:01",
    "#00208:0002",
    "#00309:03",
    "#00311:01",
    "#00411:01",
    "#004SC:04",
];
const TEMPO_MD5: &str = "9ff9973e4575867cf82cce19e6259be8";

const RANDOM: &[&str] =
    &["#TITLE Random", "#BPM 120", "#WAV01 a.wav", "#RANDOM 1", "#IF 1", "#00011:01", "#00019:01", "#ENDIF", "#IF 2", "#00012:01", "#ENDIF", "#ENDRANDOM"];
const RANDOM_MD5: &str = "142254ac3e67c2b95527741bdd5ce0c1";

const FIVE: &[&str] = &["#TITLE Five", "#BPM 150", "#WAV01 a.wav", "#00011:01", "#00015:01", "#00016:01"];
const FIVE_MD5: &str = "849ef1a3a30f313f33aaff3796af6f55";

const DOUBLE: &[&str] = &["#TITLE Double", "#BPM 150", "#WAV01 a.wav", "#00011:01", "#00021:01", "#00019:01"];
const DOUBLE_MD5: &str = "9b91584ccfc32eb389259e47ff8ab60e";

const MINIMAL_BMSON_MD5: &str = "ae8ae719abd25371124fd5eb6756fa8a";
const MINIMAL_BMSON_SHA256: &str = "e79b84e27e35aecad52b711efa59af084515e93f089677da862a599628c6d348";

const NOT_IN_ANY_LIBRARY_MD5: &str = "ffffffffffffffffffffffffffffffff";
const ZERO_MD5: &str = "00000000000000000000000000000000";

const PATTERNS_OF_THE_STAGED_LIBRARY: [&str; 6] = [PLAIN_MD5, LONG_MD5, CHARGE_MD5, TEMPO_MD5, RANDOM_MD5, MINIMAL_BMSON_MD5];

static SCRATCH_SERIAL: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let serial = SCRATCH_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("rbms-cli-chart-pattern-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch folder should be creatable");
        Self(path)
    }

    fn library(&self) -> PathBuf {
        self.0.join("library")
    }

    fn out(&self) -> PathBuf {
        self.0.join("out")
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.library().join(relative);
        std::fs::create_dir_all(path.parent().expect("a file has a folder")).expect("the library folder should be creatable");
        std::fs::write(&path, bytes).expect("the scratch file should be writable");
        path
    }

    fn write_lines(&self, relative: &str, lines: &[&str]) -> PathBuf {
        self.write(relative, text_of(lines).as_bytes())
    }

    fn args(&self) -> PatternArgs {
        PatternArgs { library: self.library(), out: self.out(), table: None, mode: DEFAULT_MODE }
    }

    fn stage_library(&self) {
        self.write_lines("songs/plain/plain.bms", PLAIN);
        self.write_lines("songs/plain/again/plain-again.bms", PLAIN);
        self.write_lines("songs/long/long.bms", LONG);
        self.write_lines("songs/long/charge.bme", CHARGE);
        self.write_lines("songs/tempo/tempo.bml", TEMPO);
        self.write_lines("songs/random/random.bms", RANDOM);
        self.write_lines("songs/five/five.bms", FIVE);
        self.write_lines("songs/double/double.bms", DOUBLE);
        self.write("songs/bmson/minimal.bmson", &minimal_bmson());
        self.write("songs/broken/broken-b.bmson", b"not JSON");
        self.write("songs/broken/broken-a.bmson", b"{\"info\":");
        self.write("songs/readme.txt", b"not a chart");
    }

    fn pattern(&self, md5: &str) -> Value {
        let bytes = std::fs::read(self.out().join(format!("{md5}.json"))).unwrap_or_else(|e| panic!("no pattern for {md5}: {e}"));
        serde_json::from_slice(&bytes).expect("a pattern file is JSON")
    }

    fn out_file_names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.out())
            .expect("the out folder should be there")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn text_of(lines: &[&str]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn row(label: &str, value: &str) -> String {
    format!("{label:<14}: {value}")
}

fn minimal_bmson() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/rbms-parser/tests/bmson/minimal.bmson")).expect("the bmson fixture should be there")
}

fn args_of(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

fn pattern_bytes(bytes: &[u8], file_name: &str, mode: Mode) -> (String, Vec<u8>) {
    match extract(bytes, Path::new(file_name), mode).expect("the chart should be read") {
        Extracted::Pattern { md5, document } => (md5, document),
        Extracted::OtherMode { md5 } => panic!("{file_name} ({md5}) is of another mode than {}", mode.name),
    }
}

fn pattern_of(lines: &[&str], file_name: &str) -> Value {
    let (_, document) = pattern_bytes(text_of(lines).as_bytes(), file_name, Mode::BEAT_7K);
    serde_json::from_slice(&document).expect("a pattern is JSON")
}

fn run_report(args: &PatternArgs) -> PatternReport {
    export(args).expect("the run should come out")
}

fn write_table(scratch: &Scratch, body: &str) -> PathBuf {
    let path = scratch.0.join("data.json");
    std::fs::write(&path, body).expect("the table should be writable");
    path
}

fn modified(path: &Path) -> SystemTime {
    std::fs::metadata(path).and_then(|meta| meta.modified()).expect("the file should have a modification time")
}

fn set_modified(path: &Path, time: SystemTime) {
    std::fs::File::options().write(true).open(path).and_then(|file| file.set_modified(time)).expect("the modification time should be settable");
}

#[test]
fn a_library_and_an_out_folder_are_enough_and_the_mode_defaults_to_7k() {
    let parsed = parse(&args_of(&["/library", "--out", "/patterns"])).expect("a library and --out are enough");
    assert_eq!(parsed, PatternArgs { library: "/library".into(), out: "/patterns".into(), table: None, mode: Mode::BEAT_7K });
    assert_eq!(DEFAULT_MODE, Mode::BEAT_7K);
}

#[test]
fn every_option_is_read_wherever_it_stands() {
    let parsed = parse(&args_of(&["--out", "/patterns", "--mode", "BEAT-5K", "/library", "--table", "data.json"])).unwrap();
    assert_eq!(parsed.library, Path::new("/library"));
    assert_eq!(parsed.out, Path::new("/patterns"));
    assert_eq!(parsed.table.as_deref(), Some(Path::new("data.json")));
    assert_eq!(parsed.mode, Mode::BEAT_5K, "a mode is read whatever its case");
}

#[test]
fn every_mode_a_chart_can_be_of_is_named_by_its_flag() {
    for mode in Mode::ALL.iter().chain([&Mode::KEYBOARD_24K]) {
        let flag = mode_flag(mode);
        assert_eq!(parse(&args_of(&["/l", "--out", "/o", "--mode", &flag])).map(|parsed| parsed.mode), Ok(*mode), "{flag}");
    }
    assert_eq!(mode_flag(&Mode::BEAT_7K), "beat-7k");
    assert_eq!(mode_flag(&Mode::POPN_9K), "popn-9k");
}

#[test]
fn a_missing_part_a_missing_value_or_an_unknown_option_is_refused() {
    assert!(parse(&args_of(&["--out", "/o"])).is_err(), "the library is required");
    assert!(parse(&args_of(&["/l"])).is_err(), "--out is required");
    assert!(parse(&args_of(&["/l", "--out"])).is_err(), "--out needs a value");
    assert!(parse(&args_of(&["/l", "--out", "/o", "--table"])).is_err(), "--table needs a value");
    assert!(parse(&args_of(&["/l", "--out", "/o", "--mode"])).is_err(), "--mode needs a value");
    assert!(parse(&args_of(&["/l", "--out", "/o", "--nope"])).is_err(), "unknown options are rejected");
    assert!(parse(&args_of(&["/l", "/m", "--out", "/o"])).is_err(), "only one library is accepted");
}

#[test]
fn an_unknown_mode_is_refused_with_the_modes_there_are() {
    let error = parse(&args_of(&["/l", "--out", "/o", "--mode", "beat-9k"])).expect_err("beat-9k is no mode");
    assert!(error.contains("beat-7k") && error.contains("popn-9k") && error.contains("beat-9k"), "{error}");
}

#[test]
fn a_plain_chart_gives_its_header_hashes_and_note_columns() {
    let document = pattern_of(PLAIN, "plain.bms");
    assert_eq!(document["format"], FORMAT);
    assert_eq!(document["version"], VERSION);
    assert_eq!(document["parser_version"], PARSER_VERSION);
    assert_eq!(document["md5"], PLAIN_MD5);
    assert_eq!(document["sha256"], PLAIN_SHA256);
    assert_eq!(document["mode"], "BEAT_7K");
    assert_eq!(document["declared_ln"], Value::Null);
    assert_eq!(document["has_undefined_ln"], false);
    assert_eq!(document["uses_control_flow"], false);
    assert_eq!(document["init_bpm"], 120.0);
    assert_eq!(document["total"], 200.0);
    assert_eq!(document["rank"], 2);
    assert_eq!(document["play_level"], 5);
    assert_eq!(document["playable_notes"], 4);
    assert_eq!(
        document["notes"],
        json!({ "time_us": [0, 0, 1_000_000, 1_000_000], "lane": [0, 7, 0, 6], "kind": [0, 0, 0, 0], "section": [0.0, 0.0, 0.5, 0.5] }),
        "lane 7 is the scratch, and the notes are in time order, a lane to the left first"
    );
    assert_eq!(document["tempo"], json!([]), "a chart that never changes its tempo has no tempo point");
}

#[test]
fn the_document_keeps_the_order_of_the_format_table() {
    let (_, bytes) = pattern_bytes(text_of(PLAIN).as_bytes(), "plain.bms", Mode::BEAT_7K);
    let text = String::from_utf8(bytes).expect("a pattern is UTF-8");
    let keys = [
        "format",
        "version",
        "parser_version",
        "md5",
        "sha256",
        "mode",
        "declared_ln",
        "has_undefined_ln",
        "uses_control_flow",
        "init_bpm",
        "total",
        "rank",
        "play_level",
        "playable_notes",
        "notes",
        "tempo",
    ];
    let positions: Vec<usize> = keys.iter().map(|key| text.find(&format!("\"{key}\":")).unwrap_or_else(|| panic!("{key} is missing"))).collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "the keys are out of order: {positions:?}");
    assert!(text.starts_with('{') && !text.contains('\n'), "a pattern is one compact line");
}

#[test]
fn reading_a_chart_twice_gives_the_same_bytes() {
    let first = pattern_bytes(text_of(TEMPO).as_bytes(), "tempo.bms", Mode::BEAT_7K);
    let second = pattern_bytes(text_of(TEMPO).as_bytes(), "tempo.bms", Mode::BEAT_7K);
    assert_eq!(first, second);
}

#[test]
fn a_long_note_a_mine_and_a_note_are_told_apart_and_an_unstated_flavour_is_null() {
    let document = pattern_of(LONG, "long.bms");
    assert_eq!(document["md5"], LONG_MD5);
    assert_eq!(
        document["notes"],
        json!({ "time_us": [0, 0, 2_000_000, 3_500_000], "lane": [2, 5, 0, 0], "kind": [3, 0, 1, 2], "section": [0.0, 0.0, 1.0, 1.75] }),
        "kind 1 is a long note's head, 2 its end, 3 a mine; the long note is in the second measure"
    );
    assert_eq!(document["declared_ln"], Value::Null, "the chart did not state a long note flavour");
    assert_eq!(document["has_undefined_ln"], true);
    assert_eq!(document["playable_notes"], 2, "the end of a long note is judged only in a charge note, and a mine is no note");
    assert_eq!(document["play_level"], Value::Null, "a chart without #PLAYLEVEL has no level");
    assert_eq!(document["total"], 0.0, "a chart without #TOTAL says 0");
    assert_eq!(document["rank"], 3);
}

#[test]
fn a_stated_flavour_is_named_and_its_end_is_a_note_to_judge() {
    let document = pattern_of(CHARGE, "charge.bms");
    assert_eq!(document["md5"], CHARGE_MD5);
    assert_eq!(document["declared_ln"], "cn");
    assert_eq!(document["has_undefined_ln"], false);
    assert_eq!(document["playable_notes"], 3);
    assert_eq!(document["notes"]["kind"], json!([3, 0, 1, 2]));
}

#[test]
fn tempo_points_are_where_the_bpm_a_stop_or_the_scroll_speed_is_stated() {
    let document = pattern_of(TEMPO, "tempo.bms");
    assert_eq!(document["md5"], TEMPO_MD5);
    assert_eq!(
        document["tempo"],
        json!([
            { "t_us": 5_000_000, "bpm": 240.0, "stop_us": 0, "scroll": 1.0 },
            { "t_us": 5_500_000, "bpm": 240.0, "stop_us": 500_000, "scroll": 1.0 },
            { "t_us": 7_000_000, "bpm": 240.0, "stop_us": 0, "scroll": 2.0 },
        ]),
        "a tempo change, a stop of half a second, and a scroll change; the timelines between them state nothing"
    );
    assert_eq!(
        document["notes"],
        json!({ "time_us": [0, 0, 5_500_000, 7_000_000], "lane": [0, 6, 0, 0], "kind": [0, 0, 0, 0], "section": [0.0, 0.0, 3.0, 4.0] }),
        "the notes are placed by the tempo before them and the stop that was waited out"
    );
}

#[test]
fn control_flow_is_reported_and_the_chart_is_read_as_the_player_reads_it() {
    let document = pattern_of(RANDOM, "random.bms");
    assert_eq!(document["md5"], RANDOM_MD5);
    assert_eq!(document["uses_control_flow"], true);
    assert_eq!(document["notes"]["lane"], json!([0, 6]), "the branch #RANDOM 1 leaves is the only one that can be drawn");
    assert_eq!(pattern_of(PLAIN, "plain.bms")["uses_control_flow"], false);
}

#[test]
fn a_bmson_chart_is_read_through_its_own_parser_with_the_hashes_of_its_bytes() {
    let (md5, document) = pattern_bytes(&minimal_bmson(), "minimal.bmson", Mode::BEAT_7K);
    assert_eq!(md5, MINIMAL_BMSON_MD5);
    let document: Value = serde_json::from_slice(&document).unwrap();
    assert_eq!(document["sha256"], MINIMAL_BMSON_SHA256);
    assert_eq!(document["mode"], "BEAT_7K");
    assert_eq!(document["uses_control_flow"], false);
    assert_eq!(document["play_level"], 5);
    assert_eq!(
        document["notes"],
        json!({ "time_us": [0, 1_000_000], "lane": [0, 7], "kind": [0, 0], "section": [0.0, 0.5] }),
        "x 1 is the first key and x 8 the scratch"
    );
    assert_eq!(extract(b"not JSON", Path::new("broken.bmson"), Mode::BEAT_7K).err().map(|e| e.starts_with("bmson parse error")), Some(true));
}

#[test]
fn a_chart_of_another_mode_gives_only_its_md5() {
    for (lines, md5, mode) in [(FIVE, FIVE_MD5, Mode::BEAT_5K), (DOUBLE, DOUBLE_MD5, Mode::BEAT_14K)] {
        match extract(text_of(lines).as_bytes(), Path::new("chart.bms"), Mode::BEAT_7K).expect("the chart should be read") {
            Extracted::OtherMode { md5: found } => assert_eq!(found, md5, "{}", mode.name),
            Extracted::Pattern { .. } => panic!("{} was written as a 7K chart", mode.name),
        }
        assert_eq!(pattern_bytes(text_of(lines).as_bytes(), "chart.bms", mode).0, md5);
    }
    let extension_decides = extract(text_of(PLAIN).as_bytes(), Path::new("plain.pms"), Mode::BEAT_7K).expect("the chart should be read");
    assert!(matches!(extension_decides, Extracted::OtherMode { ref md5 } if md5 == PLAIN_MD5), "a .pms file is a POPN_9K chart whatever it holds");
}

#[test]
fn the_charts_under_a_folder_are_listed_in_path_order_and_nothing_else_is() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let walked = charts_under(&scratch.library());
    let names: Vec<String> = walked.charts.iter().map(|path| path.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 11, "{names:?}");
    assert!(!names.contains(&"readme.txt".to_string()));
    assert!(walked.charts.windows(2).all(|pair| pair[0] < pair[1]), "the paths are not in order");
    assert!(walked.failures.is_empty());
}

#[cfg(unix)]
#[test]
fn a_link_back_up_the_tree_is_not_followed() {
    let scratch = Scratch::new();
    scratch.write_lines("songs/plain.bms", PLAIN);
    std::os::unix::fs::symlink(scratch.library(), scratch.library().join("songs/loop")).expect("a link should be creatable");
    assert_eq!(charts_under(&scratch.library()).charts.len(), 1);
}

#[test]
fn a_library_is_written_as_one_pattern_per_chart_of_the_mode() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let report = run_report(&scratch.args());

    assert_eq!((report.found, report.written, report.unchanged, report.duplicates, report.other_mode), (11, 6, 0, 1, 2));
    let mut expected: Vec<String> = PATTERNS_OF_THE_STAGED_LIBRARY.iter().map(|md5| format!("{md5}.json")).collect();
    expected.sort();
    assert_eq!(scratch.out_file_names(), expected, "one file per chart of the mode, named by its md5, and no file left half written");
    for md5 in PATTERNS_OF_THE_STAGED_LIBRARY {
        let document = scratch.pattern(md5);
        assert_eq!(document["md5"], md5);
        assert_eq!(document["mode"], "BEAT_7K");
    }
    assert!(report.coverage.is_none(), "no table was given");
}

#[test]
fn files_that_could_not_be_read_are_listed_by_path_and_the_rest_is_written_all_the_same() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let report = run_report(&scratch.args());

    let failed: Vec<&Path> = report.failures.iter().map(|failure| failure.path.as_path()).collect();
    assert_eq!(failed, [scratch.library().join("songs/broken/broken-a.bmson"), scratch.library().join("songs/broken/broken-b.bmson")]);
    assert!(report.failures.iter().all(|failure| failure.reason.starts_with("bmson parse error")), "{:?}", report.failures);
    assert_eq!(report.written, 6);
}

#[test]
fn another_mode_writes_the_charts_of_that_mode_only() {
    let scratch = Scratch::new();
    scratch.stage_library();

    let five = run_report(&PatternArgs { mode: Mode::BEAT_5K, ..scratch.args() });
    assert_eq!((five.written, five.other_mode, five.failures.len()), (1, 8, 2));
    assert_eq!(scratch.out_file_names(), [format!("{FIVE_MD5}.json")]);
    assert_eq!(scratch.pattern(FIVE_MD5)["mode"], "BEAT_5K");
    assert_eq!(scratch.pattern(FIVE_MD5)["notes"]["lane"], json!([0, 4, 5]));

    let double = run_report(&PatternArgs { mode: Mode::BEAT_14K, ..scratch.args() });
    assert_eq!(double.written, 1);
    assert_eq!(scratch.pattern(DOUBLE_MD5)["mode"], "BEAT_14K");
    assert_eq!(scratch.pattern(DOUBLE_MD5)["notes"]["lane"], json!([0, 6, 8]));
}

#[test]
fn a_second_run_touches_no_file_whose_content_would_not_change() {
    let scratch = Scratch::new();
    scratch.stage_library();
    run_report(&scratch.args());

    let long_ago = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400);
    let files: Vec<PathBuf> = PATTERNS_OF_THE_STAGED_LIBRARY.iter().map(|md5| scratch.out().join(format!("{md5}.json"))).collect();
    for file in &files {
        set_modified(file, long_ago);
    }

    let again = run_report(&scratch.args());
    assert_eq!((again.written, again.unchanged, again.duplicates), (0, 6, 1));
    for file in &files {
        assert_eq!(modified(file), long_ago, "{} was written again", file.display());
    }
}

#[test]
fn a_second_run_writes_back_only_what_differs_or_is_gone() {
    let scratch = Scratch::new();
    scratch.stage_library();
    run_report(&scratch.args());
    let long_ago = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400);
    for md5 in PATTERNS_OF_THE_STAGED_LIBRARY {
        set_modified(&scratch.out().join(format!("{md5}.json")), long_ago);
    }

    let changed = scratch.out().join(format!("{PLAIN_MD5}.json"));
    let original = std::fs::read(&changed).unwrap();
    std::fs::write(&changed, b"{}").unwrap();
    set_modified(&changed, long_ago);
    let removed = scratch.out().join(format!("{TEMPO_MD5}.json"));
    std::fs::remove_file(&removed).unwrap();

    let again = run_report(&scratch.args());
    assert_eq!((again.written, again.unchanged), (2, 4));
    assert_eq!(std::fs::read(&changed).unwrap(), original, "the changed pattern is back as it was");
    assert!(removed.is_file(), "the removed pattern is back");
    assert!(modified(&changed) > long_ago, "a pattern that was rewritten has a new modification time");
    assert_eq!(modified(&scratch.out().join(format!("{LONG_MD5}.json"))), long_ago, "the others were left alone");
}

#[test]
fn a_pattern_of_an_earlier_run_that_no_chart_stands_behind_any_more_stays() {
    let scratch = Scratch::new();
    scratch.stage_library();
    run_report(&scratch.args());
    std::fs::remove_file(scratch.library().join("songs/long/long.bms")).unwrap();

    let again = run_report(&scratch.args());
    assert_eq!((again.found, again.written, again.unchanged), (10, 0, 5));
    assert!(scratch.out().join(format!("{LONG_MD5}.json")).is_file());
}

#[test]
fn the_out_folder_is_made_when_it_is_not_there() {
    let scratch = Scratch::new();
    scratch.write_lines("plain.bms", PLAIN);
    let args = PatternArgs { out: scratch.0.join("deeper/than/that"), ..scratch.args() };
    assert_eq!(run_report(&args).written, 1);
    assert!(args.out.join(format!("{PLAIN_MD5}.json")).is_file());
}

#[test]
fn a_table_is_matched_on_md5_and_what_the_library_lacks_is_listed() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let table = write_table(
        &scratch,
        &format!(
            r#"[
                {{"md5":"{upper_plain}","level":"1","title":"upper case is the same md5"}},
                {{"md5":"{LONG_MD5}","level":"2","title":"long"}},
                {{"md5":"{FIVE_MD5}","level":"3","title":"a 5K chart"}},
                {{"md5":"{NOT_IN_ANY_LIBRARY_MD5}","level":"4","title":"not here"}},
                {{"md5":"{ZERO_MD5}","level":"5","title":"not here either"}},
                {{"sha256":"{sha}","level":"6","title":"named by its sha256 alone"}},
                {{"md5":"{PLAIN_MD5}","level":"7","title":"listed twice"}}
            ]"#,
            upper_plain = PLAIN_MD5.to_ascii_uppercase(),
            sha = PLAIN_SHA256,
        ),
    );
    let report = run_report(&PatternArgs { table: Some(table), ..scratch.args() });

    let coverage = report.coverage.as_ref().expect("a table was given");
    assert_eq!(coverage.table_md5s, 5, "a repeated md5 is one");
    assert_eq!(coverage.table_entries_without_md5, 1);
    assert_eq!(coverage.matched, 2);
    assert_eq!(coverage.missing, [ZERO_MD5, NOT_IN_ANY_LIBRARY_MD5], "the md5s no chart has, in order");
    assert_eq!(coverage.other_mode, [FIVE_MD5], "a 5K chart is in the library but not in 7K");
    assert_eq!(coverage.failed_files, 2);
}

#[test]
fn the_coverage_is_kept_next_to_the_patterns_and_printed() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let table = write_table(&scratch, &format!(r#"[{{"md5":"{PLAIN_MD5}","level":"1"}},{{"md5":"{NOT_IN_ANY_LIBRARY_MD5}","level":"2"}}]"#));
    let args = PatternArgs { table: Some(table), ..scratch.args() };
    let report = run_report(&args);

    let summary: Value = serde_json::from_slice(&std::fs::read(scratch.out().join(COVERAGE_FILE_NAME)).expect("the summary should be written")).unwrap();
    assert_eq!(summary["format"], "rbms-chart-pattern-coverage");
    assert_eq!(summary["version"], 1);
    assert_eq!(summary["mode"], "BEAT_7K");
    assert_eq!(summary["parser_version"], PARSER_VERSION);
    assert_eq!(summary["table_md5s"], 2);
    assert_eq!(summary["matched"], 1);
    assert_eq!(summary["missing"], json!([NOT_IN_ANY_LIBRARY_MD5]));
    assert_eq!(summary["other_mode"], json!([]));

    let text = super::render::text(&args, &report);
    assert!(text.contains(&format!("{}\n  {NOT_IN_ANY_LIBRARY_MD5}", row("missing", "1"))), "{text}");
    assert!(text.contains(&row("matched", "1")), "{text}");
    assert!(text.contains(&row("patterns", "6 (written 6, unchanged 0)")), "{text}");
    assert!(text.contains("broken-a.bmson: bmson parse error"), "{text}");
    for (label, value) in [("charts", "11"), ("duplicates", "1"), ("other mode", "2"), ("failed", "2")] {
        assert!(text.contains(&row(label, value)), "{label}: {text}");
    }
}

#[test]
fn without_a_table_there_is_no_coverage_file_and_no_table_lines() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let args = scratch.args();
    let report = run_report(&args);
    assert!(!scratch.out().join(COVERAGE_FILE_NAME).exists());
    assert!(!super::render::text(&args, &report).contains("table"));
}

#[test]
fn the_coverage_is_written_again_only_when_it_changes() {
    let scratch = Scratch::new();
    scratch.stage_library();
    let table = write_table(&scratch, &format!(r#"[{{"md5":"{PLAIN_MD5}"}}]"#));
    let args = PatternArgs { table: Some(table.clone()), ..scratch.args() };
    run_report(&args);

    let summary = scratch.out().join(COVERAGE_FILE_NAME);
    let long_ago = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400);
    set_modified(&summary, long_ago);
    run_report(&args);
    assert_eq!(modified(&summary), long_ago, "the same coverage was written again");

    std::fs::write(&table, format!(r#"[{{"md5":"{PLAIN_MD5}"}},{{"md5":"{ZERO_MD5}"}}]"#)).unwrap();
    run_report(&args);
    assert!(modified(&summary) > long_ago);
    let summary: Value = serde_json::from_slice(&std::fs::read(&summary).unwrap()).unwrap();
    assert_eq!(summary["missing"], json!([ZERO_MD5]));
}

#[test]
fn a_run_that_cannot_be_done_is_an_error_before_anything_is_written() {
    let scratch = Scratch::new();
    scratch.write_lines("plain.bms", PLAIN);

    let no_library = PatternArgs { library: scratch.0.join("not-there"), ..scratch.args() };
    assert!(export(&no_library).expect_err("a missing library is an error").contains("not a folder"));

    let no_table = PatternArgs { table: Some(scratch.0.join("missing.json")), ..scratch.args() };
    assert!(export(&no_table).expect_err("a missing table is an error").contains("table not read"));

    let bad_table = PatternArgs { table: Some(write_table(&scratch, "not a table")), ..scratch.args() };
    assert!(export(&bad_table).expect_err("a table that is not JSON is an error").contains("table not parsed"));

    assert!(!scratch.out().exists(), "nothing was made for a run that could not be done");
}

#[test]
fn a_run_that_comes_out_exits_clean_even_with_files_it_could_not_read() {
    let scratch = Scratch::new();
    scratch.stage_library();
    assert_eq!(run(&scratch.args()), ExitCode::SUCCESS);
    assert_eq!(run(&PatternArgs { library: scratch.0.join("not-there"), ..scratch.args() }), ExitCode::FAILURE);
}
