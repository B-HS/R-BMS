use std::path::{Path, PathBuf};

use crate::load::{file_name_for, load_dir, load_file, parse, save};
use crate::model::{CONSTRAINT_GROUP_COUNT, Course, CourseChart, CourseConstraint, DEFAULT_COURSE_NAME, TrophyRule};
use crate::run::{CLEAR_FAILED, CLEAR_NO_PLAY, CourseRun, CourseStep, StageResult};

const ARRAY_FIXTURE: &str = include_str!("../tests/fixtures/courses_array.json");
const SINGLE_FIXTURE: &str = include_str!("../tests/fixtures/course_single.json");
const NORMALISE_FIXTURE: &str = include_str!("../tests/fixtures/course_normalise.json");

/// Notes every run in these tests is measured over, chosen so a whole-percent rate is exact.
const TEST_NOTES: u32 = 1_000;

/// Bad-poor count giving a miss rate of exactly 5%.
const TEST_MIN_BP: u32 = 50;

/// EX score giving a score rate of exactly 70% over [`TEST_NOTES`].
const TEST_EX: u32 = 1_400;

const TEST_LAMP_NORMAL: u8 = 5;
const TEST_LAMP_HARD: u8 = 6;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms_course_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn chart(md5: &str, sha256: &str, title: &str) -> CourseChart {
    CourseChart { md5: md5.to_string(), sha256: sha256.to_string(), title: title.to_string() }
}

fn course_of(charts: Vec<CourseChart>) -> Course {
    Course { charts, ..Course::default() }
}

/// A stage that scored `ex` over [`TEST_NOTES`] notes and ended on `gauge`.
fn stage(ex: u32, gauge: f32, survived: bool) -> StageResult {
    StageResult {
        ex_score: ex,
        max_ex_score: TEST_NOTES * 2,
        notes: TEST_NOTES,
        counts: [0, 0, 0, 10, 20, 20],
        empty_poor: 3,
        fast: 40,
        slow: 60,
        combo_breaks: 30,
        max_combo: 300,
        combo_at_end: 120,
        gauge_value: gauge,
        clear: if survived { TEST_LAMP_HARD } else { CLEAR_FAILED },
        survived,
    }
}

#[test]
fn every_constraint_token_round_trips() {
    for constraint in CourseConstraint::ALL {
        let token = constraint.token();
        assert!(!token.is_empty(), "{constraint:?} has a token");
        assert_eq!(CourseConstraint::from_token(token), Some(constraint), "{token} parses back");
    }
    assert_eq!(CourseConstraint::ALL.len(), CourseConstraint::COUNT);
}

#[test]
fn constraint_tokens_are_unique_and_groups_are_in_range() {
    let mut tokens: Vec<&str> = CourseConstraint::ALL.iter().map(|c| c.token()).collect();
    tokens.sort_unstable();
    let before = tokens.len();
    tokens.dedup();
    assert_eq!(tokens.len(), before, "no two constraints share a token");
    for constraint in CourseConstraint::ALL {
        assert!(usize::from(constraint.group()) < CONSTRAINT_GROUP_COUNT, "{constraint:?} is in a known group");
    }
}

#[test]
fn constraint_tokens_match_the_reference_table() {
    assert_eq!(CourseConstraint::Class.token(), "grade");
    assert_eq!(CourseConstraint::Mirror.token(), "grade_mirror");
    assert_eq!(CourseConstraint::Random.token(), "grade_random");
    assert_eq!(CourseConstraint::NoSpeed.token(), "no_speed");
    assert_eq!(CourseConstraint::NoGood.token(), "no_good");
    assert_eq!(CourseConstraint::NoGreat.token(), "no_great");
    assert_eq!(CourseConstraint::GaugeLr2.token(), "gauge_lr2");
    assert_eq!(CourseConstraint::Gauge5Keys.token(), "gauge_5k");
    assert_eq!(CourseConstraint::Gauge7Keys.token(), "gauge_7k");
    assert_eq!(CourseConstraint::Gauge9Keys.token(), "gauge_9k");
    assert_eq!(CourseConstraint::Gauge24Keys.token(), "gauge_24k");
    assert_eq!(CourseConstraint::Ln.token(), "ln");
    assert_eq!(CourseConstraint::Cn.token(), "cn");
    assert_eq!(CourseConstraint::Hcn.token(), "hcn");
}

#[test]
fn constraint_groups_match_the_reference_table() {
    let grouped: Vec<u8> = CourseConstraint::ALL.iter().map(|c| c.group()).collect();
    assert_eq!(grouped, vec![0, 0, 0, 1, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4]);
}

#[test]
fn a_constraint_also_parses_from_the_reference_enum_name() {
    assert_eq!(CourseConstraint::from_any("CLASS"), Some(CourseConstraint::Class));
    assert_eq!(CourseConstraint::from_any("GAUGE_24KEYS"), Some(CourseConstraint::Gauge24Keys));
    assert_eq!(CourseConstraint::from_any("HCN"), Some(CourseConstraint::Hcn));
    assert_eq!(CourseConstraint::from_any("nothing_like_it"), None);
    assert_eq!(CourseConstraint::from_token("CLASS"), None, "the enum name is not a token");
}

#[test]
fn a_constraint_serialises_as_its_token() {
    let json = serde_json::to_string(&CourseConstraint::Gauge7Keys).unwrap();
    assert_eq!(json, "\"gauge_7k\"");
    let back: CourseConstraint = serde_json::from_str(&json).unwrap();
    assert_eq!(back, CourseConstraint::Gauge7Keys);
}

#[test]
fn the_array_fixture_loads_every_course_with_its_reference_field_names() {
    let courses = parse(ARRAY_FIXTURE);
    assert_eq!(courses.len(), 2, "both courses survive validation");

    let grade = &courses[0];
    assert_eq!(grade.name, "GENOSIDE 2018 段位認定 五段");
    assert_eq!(grade.charts.len(), 4, "the reference spells the chart list `hash`");
    assert_eq!(grade.charts[0].title, "Anthem Landing");
    assert_eq!(grade.charts[0].md5, "0d4f7bd1a9a5a1dd6a56b1b1b8a2f001");
    assert_eq!(grade.constraints, vec![CourseConstraint::Class, CourseConstraint::NoSpeed, CourseConstraint::Ln]);
    assert_eq!(grade.trophies.len(), 3);
    assert!(grade.release);
    assert!(grade.is_class());

    let mirror = &courses[1];
    assert_eq!(mirror.name, "Mirror Grade");
    assert_eq!(mirror.charts.len(), 1, "the older spelling `song` reads as the chart list too");
    assert_eq!(mirror.constraints, vec![CourseConstraint::Mirror, CourseConstraint::GaugeLr2]);
    assert!(!mirror.release, "a course kept out of release stays loadable");
}

#[test]
fn the_single_object_fixture_loads_as_a_one_course_list() {
    let courses = parse(SINGLE_FIXTURE);
    assert_eq!(courses.len(), 1, "a file holding one course object is a one-course list");
    let course = &courses[0];
    assert_eq!(course.name, "Overjoy 発狂段位");
    assert_eq!(course.charts.len(), 2);
    assert!(course.release, "release defaults to true when the file leaves it out");
}

#[test]
fn an_unknown_constraint_is_dropped_without_failing_the_course() {
    let course = &parse(SINGLE_FIXTURE)[0];
    assert_eq!(
        course.constraints,
        vec![CourseConstraint::Class, CourseConstraint::NoGood, CourseConstraint::Hcn],
        "the unknown token is gone and what is left is ordered by group"
    );
}

#[test]
fn unqualifiable_trophies_are_removed() {
    let course = &parse(SINGLE_FIXTURE)[0];
    assert_eq!(course.trophies.len(), 1, "a zero miss rate and a full score rate are both unreachable");
    assert_eq!(course.trophies[0].name, "bronzemedal");
}

#[test]
fn a_course_without_a_name_gets_the_default_title() {
    let mut course = course_of(vec![chart("abc", "", "Stage")]);
    assert!(course.validate(), "an empty name is normalised, not rejected");
    assert_eq!(course.name, DEFAULT_COURSE_NAME);
}

#[test]
fn a_chart_without_a_title_gets_its_stage_number() {
    let mut course = course_of(vec![chart("abc", "", ""), chart("def", "", "")]);
    assert!(course.validate());
    assert_eq!(course.charts[0].title, "course 1");
    assert_eq!(course.charts[1].title, "course 2");
}

#[test]
fn constraints_from_the_same_group_collapse_to_the_first_declared() {
    let mut course = Course {
        charts: vec![chart("abc", "", "Stage")],
        constraints: vec![CourseConstraint::Class, CourseConstraint::Mirror, CourseConstraint::Random],
        ..Course::default()
    };
    assert!(course.validate(), "a duplicated group is normalised, not rejected");
    assert_eq!(course.constraints, vec![CourseConstraint::Class], "only the first of the group survives");
}

#[test]
fn collapsed_constraints_come_out_in_group_order() {
    let mut course = Course {
        charts: vec![chart("abc", "", "Stage")],
        constraints: vec![CourseConstraint::Hcn, CourseConstraint::NoSpeed, CourseConstraint::Mirror, CourseConstraint::Ln],
        ..Course::default()
    };
    assert!(course.validate());
    assert_eq!(course.constraints, vec![CourseConstraint::Mirror, CourseConstraint::NoSpeed, CourseConstraint::Hcn]);
}

#[test]
fn a_course_with_no_charts_is_rejected() {
    let mut course = Course { name: "Empty".to_string(), ..Course::default() };
    assert!(!course.validate(), "an empty chart list is the course-level failure");
}

#[test]
fn a_chart_carrying_neither_hash_rejects_the_course() {
    let mut course = course_of(vec![chart("abc", "", "Playable"), chart("", "", "Unresolvable")]);
    assert!(!course.validate());
}

#[test]
fn the_normalise_fixture_keeps_only_the_course_that_can_be_normalised() {
    let courses = parse(NORMALISE_FIXTURE);
    assert_eq!(courses.len(), 1, "the chartless course and the unidentified one are both dropped");
    let course = &courses[0];
    assert_eq!(course.name, DEFAULT_COURSE_NAME);
    assert_eq!(course.charts[0].title, "course 1");
    assert_eq!(course.charts[1].title, "Named Stage");
    assert_eq!(course.constraints, vec![CourseConstraint::Class, CourseConstraint::NoSpeed, CourseConstraint::Cn]);
    assert_eq!(course.trophies.len(), 1, "the nameless trophy is removed");
    assert_eq!(course.trophies[0].name, "keeper");
}

#[test]
fn is_class_answers_for_every_group_zero_constraint() {
    for constraint in CourseConstraint::ALL {
        let course = Course { charts: vec![chart("abc", "", "Stage")], constraints: vec![constraint], ..Course::default() };
        assert_eq!(course.is_class(), constraint.group() == 0, "{constraint:?}");
    }
}

#[test]
fn the_hash_is_sensitive_to_chart_order() {
    let first = course_of(vec![chart("aa", "11", "One"), chart("bb", "22", "Two")]);
    let swapped = course_of(vec![chart("bb", "22", "Two"), chart("aa", "11", "One")]);
    assert_ne!(first.hash(), swapped.hash(), "reordering the stages is a different course");
    assert_eq!(first.hash(), course_of(vec![chart("aa", "11", "One"), chart("bb", "22", "Two")]).hash(), "the same course hashes alike");
}

#[test]
fn the_hash_is_sensitive_to_the_name_and_to_the_chart_set() {
    let base = course_of(vec![chart("aa", "11", "One")]);
    let renamed = Course { name: "Renamed".to_string(), ..base.clone() };
    let extended = course_of(vec![chart("aa", "11", "One"), chart("bb", "22", "Two")]);
    assert_ne!(base.hash(), renamed.hash());
    assert_ne!(base.hash(), extended.hash());
    assert_eq!(base.hash().len(), 64, "a hex SHA-256");
}

#[test]
fn the_hash_falls_back_to_md5_when_a_chart_has_no_sha256() {
    let with_sha = course_of(vec![chart("aa", "11", "One")]);
    let md5_only = course_of(vec![chart("aa", "", "One")]);
    assert_ne!(with_sha.hash(), md5_only.hash());
    assert_eq!(md5_only.charts[0].hash_key(), "aa");
    assert_eq!(with_sha.charts[0].hash_key(), "11");
}

#[test]
fn a_chart_hash_boundary_cannot_be_moved_without_changing_the_hash() {
    let split = course_of(vec![chart("", "aa", "One"), chart("", "bb", "Two")]);
    let joined = course_of(vec![chart("", "aabb", "One")]);
    assert_ne!(split.hash(), joined.hash(), "the separator keeps the concatenation unambiguous");
}

#[test]
fn saving_then_loading_a_course_round_trips_it() {
    let dir = temp_dir("save_round_trip");
    let mut course = Course {
        name: "Round Trip".to_string(),
        charts: vec![chart("aa", "11", "One"), chart("bb", "22", "Two")],
        constraints: vec![CourseConstraint::Class, CourseConstraint::Gauge7Keys],
        trophies: vec![TrophyRule { name: "bronzemedal".to_string(), missrate: 10.0, scorerate: 55.0 }],
        release: false,
    };
    assert!(course.validate());
    save(&dir, &course).expect("the course is written");

    let loaded = load_dir(&dir);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0], course);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_saved_file_is_named_after_the_course() {
    assert_eq!(file_name_for("Round Trip"), "Round_Trip.json");
    assert_eq!(file_name_for("段位認定/五段"), "段位認定_五段.json");
    assert_eq!(file_name_for("///"), "course.json");
}

#[test]
fn load_dir_reads_json_files_in_name_order_and_ignores_the_rest() {
    let dir = temp_dir("load_dir_order");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("b.json"), single_course_json("Second")).unwrap();
    std::fs::write(dir.join("a.json"), single_course_json("First")).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a course").unwrap();
    std::fs::write(dir.join("broken.json"), "{ this is not json").unwrap();

    let loaded = load_dir(&dir);
    assert_eq!(loaded.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["First", "Second"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_dir_of_a_directory_that_is_not_there_is_empty() {
    let dir = temp_dir("load_dir_missing");
    assert!(load_dir(&dir).is_empty());
}

#[test]
fn load_file_reports_a_read_failure_but_not_a_parse_failure() {
    let dir = temp_dir("load_file_errors");
    std::fs::create_dir_all(&dir).unwrap();
    let broken = dir.join("broken.json");
    std::fs::write(&broken, "{ nope").unwrap();
    assert!(load_file(&broken).expect("the file reads").is_empty(), "unparseable text is no courses");
    assert!(load_file(&dir.join("absent.json")).is_err(), "a missing file is an error");
    let _ = std::fs::remove_dir_all(&dir);
}

fn single_course_json(name: &str) -> String {
    format!("{{ \"name\": \"{name}\", \"hash\": [{{ \"md5\": \"aa\", \"title\": \"Stage\" }}] }}")
}

#[test]
fn a_run_carries_the_gauge_and_the_combo_across_stages() {
    let mut run = three_stage_run();
    assert_eq!(run.carry_gauge, 100.0);
    assert_eq!(run.clear, CLEAR_NO_PLAY);

    assert_eq!(run.advance(&stage(TEST_EX, 72.0, true)), CourseStep::Next);
    assert_eq!(run.index, 1);
    assert_eq!(run.carry_gauge, 72.0);
    assert_eq!(run.totals.combo_carry, 120, "the next stage starts from the combo the last one ended on");
    assert_eq!(run.totals.max_combo, 300);
    assert_eq!(run.clear, TEST_LAMP_HARD);

    let mut second = stage(TEST_EX, 44.0, true);
    second.max_combo = 200;
    second.combo_at_end = 5;
    assert_eq!(run.advance(&second), CourseStep::Next);
    assert_eq!(run.carry_gauge, 44.0);
    assert_eq!(run.totals.max_combo, 300, "the best combo of the whole course is kept");
    assert_eq!(run.totals.combo_carry, 5);
    assert_eq!(run.totals.notes, TEST_NOTES * 2);
    assert_eq!(run.totals.ex, TEST_EX * 2);
    assert_eq!(run.totals.max_ex, TEST_NOTES * 4);
}

#[test]
fn a_failed_stage_ends_the_course_and_the_stages_after_it_are_never_counted() {
    let mut run = three_stage_run();
    assert_eq!(run.advance(&stage(TEST_EX, 60.0, true)), CourseStep::Next);
    assert_eq!(run.advance(&stage(200, 0.0, false)), CourseStep::Failed);

    assert_eq!(run.failed_at, Some(1), "the run failed on the second stage");
    assert_eq!(run.clear, CLEAR_FAILED);
    assert!(run.is_finished());
    assert_eq!(run.totals.notes, TEST_NOTES * 2, "the failed stage still counts towards the totals");

    let before = run.totals;
    assert_eq!(run.advance(&stage(TEST_EX, 90.0, true)), CourseStep::Failed, "a finished run stays finished");
    assert_eq!(run.totals, before, "nothing after the failure is counted");
    assert_eq!(run.index, 1, "the run never reached the third stage");
}

#[test]
fn a_run_that_survives_every_stage_is_cleared() {
    let mut run = CourseRun::new(two_stage_course(), 100.0);
    assert_eq!(run.advance(&stage(TEST_EX, 80.0, true)), CourseStep::Next);
    let mut last = stage(TEST_EX, 88.0, true);
    last.clear = TEST_LAMP_NORMAL;
    assert_eq!(run.advance(&last), CourseStep::Cleared);
    assert_eq!(run.index, 2);
    assert_eq!(run.failed_at, None);
    assert_eq!(run.clear, TEST_LAMP_NORMAL, "the lamp is the one the course finished on");
    assert!(run.is_finished());
    assert_eq!(run.current_chart(), None);
}

#[test]
fn the_run_reports_which_chart_is_next() {
    let mut run = three_stage_run();
    assert_eq!(run.current_chart().map(|c| c.title.as_str()), Some("One"));
    assert_eq!(run.stage_number(), 1);
    run.advance(&stage(TEST_EX, 70.0, true));
    assert_eq!(run.current_chart().map(|c| c.title.as_str()), Some("Two"));
    assert_eq!(run.stage_number(), 2);
}

#[test]
fn the_totals_report_the_bad_poor_count_and_the_two_rates() {
    let mut run = CourseRun::new(two_stage_course(), 100.0);
    let mut only = stage(TEST_EX, 50.0, true);
    only.counts = [0, 0, 0, 10, 20, 20];
    run.advance(&only);
    assert_eq!(run.totals.min_bp(), TEST_MIN_BP, "bad plus poor plus miss");
    assert!((run.totals.miss_rate() - 5.0).abs() < 1e-9);
    assert!((run.totals.score_rate() - 70.0).abs() < 1e-9);
}

#[test]
fn the_highest_qualifying_trophy_is_the_one_awarded() {
    let mut run = trophy_run(vec![
        TrophyRule { name: "bronzemedal".to_string(), missrate: 10.0, scorerate: 55.0 },
        TrophyRule { name: "silvermedal".to_string(), missrate: 7.0, scorerate: 70.0 },
        TrophyRule { name: "goldmedal".to_string(), missrate: 5.0, scorerate: 85.0 },
    ]);
    run.advance(&trophy_stage());
    assert_eq!(run.trophy().map(|t| t.name.as_str()), Some("silvermedal"), "gold needs a score rate this run did not reach");
}

#[test]
fn a_trophy_qualifies_exactly_at_both_boundaries() {
    let mut run = trophy_run(vec![TrophyRule { name: "exact".to_string(), missrate: 5.0, scorerate: 70.0 }]);
    run.advance(&trophy_stage());
    assert_eq!(run.trophy().map(|t| t.name.as_str()), Some("exact"), "both bounds are inclusive");
}

#[test]
fn a_trophy_one_step_inside_either_boundary_does_not_qualify() {
    let mut tighter_miss = trophy_run(vec![TrophyRule { name: "tight".to_string(), missrate: 4.999, scorerate: 70.0 }]);
    tighter_miss.advance(&trophy_stage());
    assert_eq!(tighter_miss.trophy(), None, "the run missed one too many");

    let mut tighter_score = trophy_run(vec![TrophyRule { name: "tight".to_string(), missrate: 5.0, scorerate: 70.001 }]);
    tighter_score.advance(&trophy_stage());
    assert_eq!(tighter_score.trophy(), None, "the run scored one too few");
}

#[test]
fn a_run_with_no_notes_earns_no_trophy() {
    let run = trophy_run(vec![TrophyRule { name: "anything".to_string(), missrate: 100.0, scorerate: 0.0 }]);
    assert_eq!(run.totals.notes, 0);
    assert_eq!(run.trophy(), None);
}

fn three_stage_run() -> CourseRun {
    let course = course_of(vec![chart("aa", "11", "One"), chart("bb", "22", "Two"), chart("cc", "33", "Three")]);
    CourseRun::new(course, 100.0)
}

fn two_stage_course() -> Course {
    course_of(vec![chart("aa", "11", "One"), chart("bb", "22", "Two")])
}

fn trophy_run(trophies: Vec<TrophyRule>) -> CourseRun {
    let course = Course { charts: vec![chart("aa", "11", "One")], trophies, ..Course::default() };
    CourseRun::new(course, 100.0)
}

/// One stage scoring exactly 70% with exactly a 5% miss rate.
fn trophy_stage() -> StageResult {
    let mut s = stage(TEST_EX, 60.0, true);
    s.counts = [0, 0, 0, 10, 20, 20];
    s
}

#[test]
fn a_validated_course_reports_its_stage_count_and_group_constraints() {
    let mut course = Course {
        charts: vec![chart("aa", "11", "One"), chart("bb", "22", "Two")],
        constraints: vec![CourseConstraint::Random, CourseConstraint::Gauge9Keys],
        ..Course::default()
    };
    assert!(course.validate());
    assert_eq!(course.stage_count(), 2);
    assert_eq!(course.constraint_in_group(0), Some(CourseConstraint::Random));
    assert_eq!(course.constraint_in_group(3), Some(CourseConstraint::Gauge9Keys));
    assert_eq!(course.constraint_in_group(1), None);
}

#[test]
fn the_fixture_directory_is_where_the_tests_read_from() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    assert!(fixtures.join("courses_array.json").exists(), "the array fixture is committed");
}
