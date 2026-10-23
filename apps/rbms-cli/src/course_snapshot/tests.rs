use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

use encoding_rs::SHIFT_JIS;
use rbms_course::{CourseConstraint, TrophyRule};
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::lr2crs::{HASH_PREFIX_LENGTH, decode, parse};
use super::{EXIT_DIFFERENT, EXIT_FAILED, Inputs, SnapshotArgs, build, parse_args, render, run, sha256_of};

const HASH_PREFIX: &str = "00000000002000000000000000005190";
const MD5_LENGTH: usize = 32;
const SINGLE_PLAY_LINE: u32 = 7;
const DOUBLE_PLAY_LINE: u32 = 14;
const OTHER_LINE: u32 = 9;
const OFFICIAL_TYPE: u32 = 2;

const SINGLE_TOP: &str = "GENOSIDE 2018 段位認定 発狂皆伝";
const SINGLE_FIRST: &str = "GENOSIDE 2018 段位認定 初段";
const DOUBLE_TOP: &str = "GENOSIDE 2018 DP段位認定 発狂皆伝";
const DOUBLE_FIRST: &str = "GENOSIDE 2018 DP段位認定 初段";

const SINGLE_TOP_MD5S: [&str; 2] = ["a1111111111111111111111111111111", "a2222222222222222222222222222222"];
const SINGLE_FIRST_MD5S: [&str; 2] = ["b1111111111111111111111111111111", "b2222222222222222222222222222222"];
const DOUBLE_TOP_MD5S: [&str; 2] = ["c1111111111111111111111111111111", "c2222222222222222222222222222222"];
const DOUBLE_FIRST_MD5S: [&str; 2] = ["d1111111111111111111111111111111", "d2222222222222222222222222222222"];

const RETRIEVED_ON: &str = "2026-10-23";

static SCRATCH_SERIAL: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let serial = SCRATCH_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("rbms-cli-course-snapshot-{}-{serial}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch folder should be creatable");
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).expect("the scratch file should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn course_element(title: &str, line: u32, md5s: &[&str]) -> String {
    format!("<course><title>{title}</title><line>{line}</line><hash>{HASH_PREFIX}{}</hash><type>{OFFICIAL_TYPE}</type></course>\r\n\r\n", md5s.concat())
}

fn course_list(elements: &[String]) -> String {
    format!("<?xml version=\"1.0\" encoding=\"shift_jis\"?>\r\n<courselist>\r\n{}</courselist>\r\n", elements.concat())
}

fn official_list() -> String {
    course_list(&[
        course_element(SINGLE_TOP, SINGLE_PLAY_LINE, &SINGLE_TOP_MD5S),
        course_element(SINGLE_FIRST, SINGLE_PLAY_LINE, &SINGLE_FIRST_MD5S),
        course_element(DOUBLE_TOP, DOUBLE_PLAY_LINE, &DOUBLE_TOP_MD5S),
        course_element(DOUBLE_FIRST, DOUBLE_PLAY_LINE, &DOUBLE_FIRST_MD5S),
    ])
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer.start_file(*name, SimpleFileOptions::default().compression_method(CompressionMethod::Stored)).expect("start the entry");
        writer.write_all(bytes).expect("write the entry");
    }
    writer.finish().expect("finish the archive").into_inner()
}

fn official_zip(list: &str) -> Vec<u8> {
    zip_of(&[("test.lr2crs", &SHIFT_JIS.encode(list).0)])
}

fn table_course(name: &str, md5s: &[&str], medals: Value) -> Value {
    json!({ "name": name, "constraint": ["grade_mirror", "gauge_lr2", "ln"], "trophy": medals, "md5": md5s })
}

fn header_of(courses: &[Value], with_bom: bool) -> Vec<u8> {
    let mut bytes = if with_bom { vec![0xEF, 0xBB, 0xBF] } else { Vec::new() };
    bytes.extend(serde_json::to_vec(&json!({ "name": "table", "course": [courses] })).expect("the header serialises"));
    bytes
}

fn gold() -> Value {
    json!([{ "name": "goldmedal", "missrate": 2.5, "scorerate": 85.0 }])
}

fn three_medals() -> Value {
    json!([
        { "name": "bronzemedal", "missrate": 7.5, "scorerate": 0.0 },
        { "name": "silvermedal", "missrate": 5.0, "scorerate": 70.0 },
        { "name": "goldmedal", "missrate": 2.5, "scorerate": 85.0 },
    ])
}

fn normal_header() -> Vec<u8> {
    header_of(&[table_course("GENOSIDE 2018 初段", &SINGLE_FIRST_MD5S, three_medals())], true)
}

fn insane_header() -> Vec<u8> {
    header_of(&[table_course("GENOSIDE 2018 段位認定 発狂皆伝", &[&SINGLE_TOP_MD5S[0].to_ascii_uppercase(), SINGLE_TOP_MD5S[1]], gold())], false)
}

fn titles() -> Vec<u8> {
    let mut named = serde_json::Map::new();
    for (md5, title) in SINGLE_TOP_MD5S.iter().chain(&SINGLE_FIRST_MD5S).zip(["Top One", "Top Two", "First One", "First Two"]) {
        named.insert((*md5).to_string(), Value::String(title.to_string()));
    }
    serde_json::to_vec(&named).expect("the titles serialise")
}

fn built_from(zip: &[u8], normal: &[u8], insane: &[u8], titles: &[u8]) -> Result<super::Snapshot, String> {
    build(&Inputs { zip, normal_header: normal, insane_header: insane, titles, retrieved_on: RETRIEVED_ON })
}

fn snapshot() -> super::Snapshot {
    built_from(&official_zip(&official_list()), &normal_header(), &insane_header(), &titles()).expect("the fixture inputs should build")
}

fn args_of(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

fn snapshot_args(scratch: &Scratch) -> SnapshotArgs {
    SnapshotArgs {
        zip: scratch.write("2018_course.zip", &official_zip(&official_list())),
        normal_header: scratch.write("header_normal.json", &normal_header()),
        insane_header: scratch.write("header_insane.json", &insane_header()),
        titles: scratch.write("titles.json", &titles()),
        retrieved_on: RETRIEVED_ON.to_string(),
        out: scratch.0.join("out").join("courses.json"),
        check: false,
    }
}

#[test]
fn lr2crs_courses_are_read_in_file_order() {
    let courses = parse(&official_list()).expect("the list should read");
    let titles: Vec<&str> = courses.iter().map(|course| course.title.as_str()).collect();
    assert_eq!(titles, [SINGLE_TOP, SINGLE_FIRST, DOUBLE_TOP, DOUBLE_FIRST]);
    assert_eq!(courses.iter().map(|course| course.line).collect::<Vec<_>>(), [SINGLE_PLAY_LINE, SINGLE_PLAY_LINE, DOUBLE_PLAY_LINE, DOUBLE_PLAY_LINE]);
    assert!(courses.iter().all(|course| course.kind == OFFICIAL_TYPE));
}

#[test]
fn a_shift_jis_list_decodes_to_the_titles_it_was_written_with() {
    let bytes = SHIFT_JIS.encode(&official_list()).0.into_owned();
    assert_ne!(bytes, official_list().into_bytes(), "the fixture is not UTF-8 by accident");
    assert_eq!(decode(&bytes).expect("Shift_JIS decodes"), official_list());
    assert!(decode(&[0x81]).is_err(), "a lead byte with nothing after it is no Shift_JIS");
}

#[test]
fn a_hash_splits_into_its_prefix_and_four_md5s() {
    let md5s: Vec<String> = ["1", "2", "3", "4"].iter().map(|digit| digit.repeat(MD5_LENGTH)).collect();
    let refs: Vec<&str> = md5s.iter().map(String::as_str).collect();
    let courses = parse(&course_list(&[course_element("four", SINGLE_PLAY_LINE, &refs)])).expect("four charts read");
    assert_eq!(courses[0].hash_prefix(), HASH_PREFIX);
    assert_eq!(courses[0].md5s(), md5s);
}

#[test]
fn a_hash_is_lower_cased_and_a_title_keeps_its_spaces() {
    let upper = SINGLE_FIRST_MD5S[0].to_ascii_uppercase();
    let courses = parse(&course_list(&[course_element(" spaced title ", SINGLE_PLAY_LINE, &[&upper])])).expect("upper case hexadecimal reads");
    assert_eq!(courses[0].md5s(), [SINGLE_FIRST_MD5S[0]]);
    assert_eq!(courses[0].title, " spaced title ");
}

#[test]
fn a_hash_of_the_wrong_length_fails() {
    let broken = |hash: &str| course_list(&[format!("<course><title>t</title><line>7</line><hash>{hash}</hash><type>2</type></course>")]);
    assert!(parse(&broken(HASH_PREFIX)).is_err(), "a prefix and no chart");
    assert!(parse(&broken(&format!("{HASH_PREFIX}{}", "a".repeat(MD5_LENGTH - 1)))).is_err(), "a chart one character short");
    assert!(parse(&broken(&format!("{}{}", "0".repeat(HASH_PREFIX_LENGTH - 1), "a".repeat(MD5_LENGTH)))).is_err(), "a prefix one character short");
    assert!(parse(&broken(&format!("{HASH_PREFIX}{}", "g".repeat(MD5_LENGTH)))).is_err(), "not hexadecimal");
}

#[test]
fn a_course_missing_a_tag_or_closing_fails() {
    assert!(parse("<courselist><course><title>t</title><line>7</line><type>2</type></course></courselist>").is_err(), "no hash");
    assert!(parse("<courselist><course><title>t</title><line>seven</line><hash></hash><type>2</type></course></courselist>").is_err(), "line is no number");
    assert!(parse("<courselist><course><title>t</title>").is_err(), "never closed");
    assert!(parse("<courselist></courselist>").is_err(), "no course at all");
}

#[test]
fn a_title_with_markup_fails_rather_than_becoming_another_id() {
    assert!(parse(&course_list(&[course_element("A &amp; B", SINGLE_PLAY_LINE, &SINGLE_FIRST_MD5S)])).is_err());
}

#[test]
fn each_line_is_reversed_to_climb_from_shodan() {
    let groups = snapshot().courses.groups;
    let names = |index: usize| groups[index].courses.iter().map(|bundled| bundled.course.name.as_str()).collect::<Vec<_>>();
    assert_eq!(groups.iter().map(|group| group.id.as_str()).collect::<Vec<_>>(), ["genoside-2018-sp", "genoside-2018-dp"]);
    assert_eq!(groups.iter().map(|group| group.keys).collect::<Vec<_>>(), [SINGLE_PLAY_LINE, DOUBLE_PLAY_LINE]);
    assert_eq!(names(0), [SINGLE_FIRST, SINGLE_TOP]);
    assert_eq!(names(1), [DOUBLE_FIRST, DOUBLE_TOP]);
}

#[test]
fn a_single_play_course_takes_the_rules_of_the_table_course_with_the_same_charts() {
    let groups = snapshot().courses.groups;
    let first = &groups[0].courses[0].course;
    assert_eq!(first.constraints, [CourseConstraint::Mirror, CourseConstraint::GaugeLr2, CourseConstraint::Ln]);
    assert_eq!(first.trophies.len(), 3, "the normal table's three medals, though it names the course otherwise");
    let top = &groups[0].courses[1].course;
    assert_eq!(
        top.trophies,
        [TrophyRule { name: "goldmedal".to_string(), missrate: 2.5, scorerate: 85.0 }],
        "found though the table wrote one MD5 in capitals"
    );
}

#[test]
fn a_double_play_course_takes_the_rules_of_the_single_play_course_of_its_grade() {
    let groups = snapshot().courses.groups;
    assert_eq!(groups[1].courses[0].course.trophies, groups[0].courses[0].course.trophies);
    assert_eq!(groups[1].courses[1].course.trophies, groups[0].courses[1].course.trophies);
    assert_eq!(groups[1].courses[0].course.constraints, groups[0].courses[0].course.constraints);
}

#[test]
fn an_sp_course_without_a_beatoraja_twin_fails_the_snapshot() {
    let only_first = header_of(&[table_course("GENOSIDE 2018 初段", &SINGLE_FIRST_MD5S, three_medals())], true);
    let error = built_from(&official_zip(&official_list()), &only_first, &header_of(&[], false), &titles()).expect_err("the top grade has no table course");
    assert!(error.contains(SINGLE_TOP), "{error}");
}

#[test]
fn a_double_play_grade_with_no_single_play_grade_fails_the_snapshot() {
    let list = course_list(&[
        course_element(SINGLE_FIRST, SINGLE_PLAY_LINE, &SINGLE_FIRST_MD5S),
        course_element("GENOSIDE 2018 DP段位認定 十段", DOUBLE_PLAY_LINE, &DOUBLE_FIRST_MD5S),
    ]);
    let error = built_from(&official_zip(&list), &normal_header(), &insane_header(), &titles()).expect_err("no single-play course is of that grade");
    assert!(error.contains("十段"), "{error}");
}

#[test]
fn a_line_that_is_neither_play_style_fails_the_snapshot() {
    let list = course_list(&[
        course_element(SINGLE_FIRST, SINGLE_PLAY_LINE, &SINGLE_FIRST_MD5S),
        course_element("GENOSIDE 2018 PMS 初段", OTHER_LINE, &DOUBLE_FIRST_MD5S),
    ]);
    assert!(built_from(&official_zip(&list), &normal_header(), &insane_header(), &titles()).is_err());
}

#[test]
fn courses_that_disagree_on_the_hash_prefix_or_type_fail_the_snapshot() {
    let other_prefix = course_element(DOUBLE_FIRST, DOUBLE_PLAY_LINE, &DOUBLE_FIRST_MD5S).replace(HASH_PREFIX, &"1".repeat(HASH_PREFIX_LENGTH));
    let list = course_list(&[course_element(SINGLE_FIRST, SINGLE_PLAY_LINE, &SINGLE_FIRST_MD5S), other_prefix]);
    assert!(built_from(&official_zip(&list), &normal_header(), &insane_header(), &titles()).is_err());

    let other_type = course_element(DOUBLE_FIRST, DOUBLE_PLAY_LINE, &DOUBLE_FIRST_MD5S).replace(&format!("<type>{OFFICIAL_TYPE}</type>"), "<type>3</type>");
    let list = course_list(&[course_element(SINGLE_FIRST, SINGLE_PLAY_LINE, &SINGLE_FIRST_MD5S), other_type]);
    assert!(built_from(&official_zip(&list), &normal_header(), &insane_header(), &titles()).is_err());
}

#[test]
fn an_archive_that_does_not_hold_exactly_one_file_fails_the_snapshot() {
    let list = SHIFT_JIS.encode(&official_list()).0.into_owned();
    for zip in [zip_of(&[]), zip_of(&[("a.lr2crs", &list), ("b.lr2crs", &list)]), b"not an archive".to_vec()] {
        assert!(built_from(&zip, &normal_header(), &insane_header(), &titles()).is_err());
    }
}

#[test]
fn a_header_with_a_bom_reads() {
    assert!(normal_header().starts_with(&[0xEF, 0xBB, 0xBF]), "the fixture starts as the real headers do");
    assert_eq!(snapshot().courses.groups[0].courses.len(), 2);
    let without = header_of(&[table_course("GENOSIDE 2018 初段", &SINGLE_FIRST_MD5S, three_medals())], false);
    let snapshot = built_from(&official_zip(&official_list()), &without, &insane_header(), &titles()).expect("a header without a BOM reads as well");
    assert_eq!(snapshot.courses.groups[0].courses[0].course.trophies.len(), 3);
}

#[test]
fn a_header_that_is_not_json_or_lists_no_course_fails_the_snapshot() {
    assert!(built_from(&official_zip(&official_list()), b"<html>", &insane_header(), &titles()).is_err());
    assert!(built_from(&official_zip(&official_list()), &serde_json::to_vec(&json!({ "name": "no courses" })).unwrap(), &insane_header(), &titles()).is_err());
}

#[test]
fn a_chart_the_titles_do_not_name_is_written_untitled_and_counted() {
    let snapshot = snapshot();
    assert_eq!(snapshot.untitled_charts, DOUBLE_FIRST_MD5S.len() + DOUBLE_TOP_MD5S.len());
    assert_eq!(snapshot.courses.groups[0].courses[0].course.charts[0].title, "First One");
    assert_eq!(snapshot.courses.groups[1].courses[0].course.charts[0].title, "");
}

#[test]
fn a_course_states_the_id_its_name_and_charts_give() {
    for bundled in snapshot().courses.courses() {
        assert_eq!(bundled.course_hash, bundled.course.hash(), "{}", bundled.course.name);
        assert!(bundled.course.charts.iter().all(|chart| chart.sha256.is_empty()));
    }
}

#[test]
fn the_source_names_the_inputs_by_size_and_sha256() {
    let zip = official_zip(&official_list());
    let source = snapshot().courses.source;
    assert_eq!(source.file_bytes, zip.len() as u64);
    assert_eq!(source.file_sha256, sha256_of(&zip));
    assert_eq!(source.entry_name, "test.lr2crs");
    assert_eq!(source.entry_sha256, sha256_of(&SHIFT_JIS.encode(&official_list()).0));
    assert_eq!((source.entry_hash_prefix.as_str(), source.entry_type), (HASH_PREFIX, OFFICIAL_TYPE));
    assert_eq!(source.retrieved_on, RETRIEVED_ON);
    let hashes: Vec<&str> = source.constraint_sources.iter().map(|file| file.sha256.as_str()).collect();
    assert_eq!(hashes, [sha256_of(&normal_header()), sha256_of(&insane_header())], "the files as they were, BOM included");
}

#[test]
fn the_same_inputs_write_the_same_bytes() {
    let first = render(&snapshot().courses).expect("the data renders");
    let second = render(&snapshot().courses).expect("the data renders");
    assert_eq!(first, second);
    assert!(first.ends_with("}\n"), "one final line break");
    assert!(!first.contains("\"sha256\": \"\""), "a chart without a sha256 does not write one");
    assert!(first.contains("\"constraint\"") && first.contains("\"trophy\""), "the spelling of the tables");
}

#[test]
fn running_writes_the_file_and_running_again_changes_nothing() {
    let scratch = Scratch::new();
    let args = snapshot_args(&scratch);
    assert_eq!(run(&args), ExitCode::SUCCESS);
    let written = std::fs::read(&args.out).expect("the data file is written, folder and all");
    assert_eq!(written, render(&snapshot().courses).unwrap().into_bytes());

    let mut permissions = std::fs::metadata(&args.out).expect("the data file").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&args.out, permissions).expect("the data file can be made read-only");
    assert_eq!(run(&args), ExitCode::SUCCESS, "an up to date file is not written again, which a read-only one would refuse");
    assert_eq!(std::fs::read(&args.out).expect("the data file"), written);
}

#[test]
fn check_is_zero_when_the_file_is_what_the_inputs_give() {
    let scratch = Scratch::new();
    let mut args = snapshot_args(&scratch);
    assert_eq!(run(&args), ExitCode::SUCCESS);
    args.check = true;
    assert_eq!(run(&args), ExitCode::SUCCESS);
}

#[test]
fn check_reports_a_difference_with_exit_code_one() {
    let scratch = Scratch::new();
    let mut args = snapshot_args(&scratch);
    assert_eq!(run(&args), ExitCode::SUCCESS);
    let mut stale = std::fs::read(&args.out).expect("the data file");
    stale.insert(0, b' ');
    std::fs::write(&args.out, &stale).expect("the data file is writable");

    args.check = true;
    assert_eq!(run(&args), ExitCode::from(EXIT_DIFFERENT));
    assert_eq!(std::fs::read(&args.out).expect("the data file"), stale, "--check never writes");

    std::fs::remove_file(&args.out).expect("the data file is removable");
    assert_eq!(run(&args), ExitCode::from(EXIT_DIFFERENT), "a missing file differs as well");
}

#[test]
fn inputs_that_cannot_be_read_or_built_exit_with_code_two() {
    let scratch = Scratch::new();
    let mut args = snapshot_args(&scratch);
    args.zip = scratch.0.join("missing.zip");
    assert_eq!(run(&args), ExitCode::from(EXIT_FAILED));

    let mut args = snapshot_args(&scratch);
    args.titles = scratch.write("titles.json", b"[1, 2]");
    args.check = true;
    assert_eq!(run(&args), ExitCode::from(EXIT_FAILED), "--check fails the same way, not as a difference");
}

#[test]
fn every_option_is_read_wherever_it_stands() {
    let parsed = parse_args(&args_of(&[
        "--out",
        "courses.json",
        "--check",
        "--zip",
        "2018_course.zip",
        "--retrieved-on",
        "2026-10-23",
        "--titles",
        "titles.json",
        "--insane-header",
        "insane.json",
        "--normal-header",
        "normal.json",
    ]))
    .expect("every option is given");
    assert_eq!(
        parsed,
        SnapshotArgs {
            zip: "2018_course.zip".into(),
            normal_header: "normal.json".into(),
            insane_header: "insane.json".into(),
            titles: "titles.json".into(),
            retrieved_on: RETRIEVED_ON.to_string(),
            out: "courses.json".into(),
            check: true,
        }
    );
}

#[test]
fn a_missing_option_a_missing_value_a_bad_date_or_an_unknown_option_is_refused() {
    let complete = ["--zip", "z", "--normal-header", "n", "--insane-header", "i", "--titles", "t", "--retrieved-on", RETRIEVED_ON, "--out", "o"];
    assert!(parse_args(&args_of(&complete)).is_ok());
    for skipped in (0..complete.len()).step_by(2) {
        let mut missing = complete.to_vec();
        missing.drain(skipped..skipped + 2);
        assert!(parse_args(&args_of(&missing)).is_err(), "{} is required", complete[skipped]);
    }
    assert!(parse_args(&args_of(&["--zip"])).is_err(), "--zip needs a value");
    for date in ["2026-1-23", "20261023", "yesterday", "2026-10-23x"] {
        let mut with_date = complete.to_vec();
        with_date[9] = date;
        assert!(parse_args(&args_of(&with_date)).is_err(), "{date} is no date");
    }
    assert!(parse_args(&args_of(&[complete.as_slice(), &["--nope"]].concat())).is_err(), "unknown options are rejected");
    assert!(parse_args(&args_of(&[complete.as_slice(), &["stray"]].concat())).is_err(), "no positional argument is taken");
}
