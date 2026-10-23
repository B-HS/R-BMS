use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::*;
use crate::model::{CourseConstraint, TrophyRule};

const MD5_A: &str = "0123456789abcdef0123456789abcdef";
const MD5_B: &str = "fedcba9876543210fedcba9876543210";
const MD5_C: &str = "00112233445566778899aabbccddeeff";
const MD5_D: &str = "ffeeddccbbaa99887766554433221100";
const SHA256_A: &str = "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";

const STELLA_NAME: &str = "Stella Skill Simulator 4th st0";

/// The charts of the bundled `GENOSIDE 2018 段位認定 初段`, which a table can list under a name of
/// its own.
const GENOSIDE_FIRST_MD5S: [&str; 4] =
    ["ae11930223a8f24e91633f1a7864e1eb", "cb661ee05c67990012bfd12703ba656c", "328dbf235f3b6cb2791e71bfb62951f8", "7c6ca04aa14d07b2c081f4dd35432844"];
const GENOSIDE_FIRST_NAME: &str = "GENOSIDE 2018 段位認定 初段";

fn entry(name: &str, md5s: &[&str]) -> Value {
    json!({ "name": name, "constraint": ["grade_mirror", "gauge_lr2", "ln"], "md5": md5s })
}

fn course_named(name: &str, md5s: &[&str]) -> Course {
    let charts = md5s.iter().map(|md5| CourseChart { md5: (*md5).to_string(), ..CourseChart::default() }).collect();
    Course { name: name.to_string(), charts, ..Course::default() }
}

fn names(read: &TableCourses) -> Vec<&str> {
    read.courses.iter().map(|course| course.name.as_str()).collect()
}

fn md5s(course: &Course) -> Vec<&str> {
    course.charts.iter().map(|chart| chart.md5.as_str()).collect()
}

#[test]
fn a_header_course_of_groups_reads_as_courses_in_order() {
    let header = json!([[entry("one", &[MD5_A]), entry("two", &[MD5_B])], [entry("three", &[MD5_C, MD5_D])]]);
    let read = courses_from_table_header(&header);
    assert_eq!(names(&read), ["one", "two", "three"], "the groups are flattened in the order written");
    assert_eq!(read.skipped, 0);
    assert_eq!(md5s(&read.courses[2]), [MD5_C, MD5_D]);
}

#[test]
fn a_flat_course_list_reads_too() {
    let read = courses_from_table_header(&json!([entry("one", &[MD5_A]), entry("two", &[MD5_B])]));
    assert_eq!(names(&read), ["one", "two"]);
    assert_eq!(read.skipped, 0);
}

#[test]
fn a_single_course_object_reads_too() {
    let read = courses_from_table_header(&entry("alone", &[MD5_A, MD5_B]));
    assert_eq!(names(&read), ["alone"]);
    assert_eq!(md5s(&read.courses[0]), [MD5_A, MD5_B]);
}

#[test]
fn md5_lists_are_lowercased_and_become_charts() {
    let upper = MD5_A.to_ascii_uppercase();
    let read = courses_from_table_header(&entry("loud", &[&upper, MD5_B]));
    let chart = &read.courses[0].charts[0];
    assert_eq!((chart.md5.as_str(), chart.sha256.as_str(), chart.title.as_str()), (MD5_A, "", "course 1"), "validation titles the chart");
    assert_eq!(md5s(&read.courses[0]), [MD5_A, MD5_B]);
}

#[test]
fn an_md5_and_a_sha256_given_as_single_values_or_numbers_read_as_one_stage() {
    let read = courses_from_table_header(&json!({ "name": "single", "md5": MD5_A, "sha256": SHA256_A.to_ascii_uppercase() }));
    let chart = &read.courses[0].charts[0];
    assert_eq!((chart.md5.as_str(), chart.sha256.as_str()), (MD5_A, SHA256_A));
    assert_eq!(read.courses[0].charts.len(), 1);

    let numbered = courses_from_table_header(&json!({ "name": "digits", "md5": [12345] }));
    assert_eq!(md5s(&numbered.courses[0]), ["12345"], "a hash written as a number is its digits");
}

#[test]
fn a_charts_list_is_read_as_written_with_its_hashes_lowercased() {
    let header = json!([{ "name": "listed", "charts": [{ "md5": MD5_A.to_ascii_uppercase(), "title": "First" }, { "sha256": SHA256_A, "title": "Second" }] }]);
    let read = courses_from_table_header(&header);
    let charts = &read.courses[0].charts;
    assert_eq!((charts[0].md5.as_str(), charts[0].title.as_str()), (MD5_A, "First"));
    assert_eq!((charts[1].md5.as_str(), charts[1].sha256.as_str(), charts[1].title.as_str()), ("", SHA256_A, "Second"));
}

#[test]
fn a_course_with_no_chart_is_skipped_and_counted() {
    let header = json!([[entry("kept", &[MD5_A]), { "name": "empty", "md5": [] }, { "name": "none" }, "not a course", 7, [[]]]]);
    let read = courses_from_table_header(&header);
    assert_eq!(names(&read), ["kept"]);
    assert_eq!(read.skipped, 5, "an empty course, a course without charts, a string, a number and a list of lists");
}

#[test]
fn a_chart_with_neither_hash_skips_its_course() {
    let read = courses_from_table_header(&json!([{ "name": "hole", "md5": [MD5_A, null] }]));
    assert!(read.courses.is_empty());
    assert_eq!(read.skipped, 1);
}

#[test]
fn a_value_that_is_no_list_and_no_object_is_one_skipped_entry() {
    assert_eq!(courses_from_table_header(&json!("courses")), TableCourses { courses: Vec::new(), skipped: 1 });
    assert_eq!(courses_from_table_header(&json!([])), TableCourses::default());
}

#[test]
fn unknown_constraints_are_dropped() {
    let header = json!({ "name": "odd", "md5": [MD5_A], "constraint": ["grade_mirror", "no_such_constraint", "gauge_lr2"] });
    let read = courses_from_table_header(&header);
    assert_eq!(read.courses[0].constraints, [CourseConstraint::Mirror, CourseConstraint::GaugeLr2]);
}

#[test]
fn a_table_course_keeps_its_trophies() {
    let header = json!({
        "name": "medals",
        "md5": [MD5_A],
        "trophy": [{ "name": "goldmedal", "missrate": 2.5, "scorerate": 85.0 }, { "name": "never", "missrate": 0.0, "scorerate": 50.0 }],
    });
    let read = courses_from_table_header(&header);
    assert_eq!(
        read.courses[0].trophies,
        [TrophyRule { name: "goldmedal".to_string(), missrate: 2.5, scorerate: 85.0 }],
        "a trophy that can never be met is dropped"
    );
}

#[test]
fn a_table_course_is_for_release() {
    let read = courses_from_table_header(&json!({ "name": "hidden", "md5": [MD5_A], "release": false }));
    assert!(read.courses[0].release);
}

#[test]
fn a_table_course_id_is_its_name_and_md5s() {
    let read = courses_from_table_header(&entry(STELLA_NAME, &[MD5_A, MD5_B, MD5_C, MD5_D]));
    let mut hasher = Sha256::new();
    for part in [STELLA_NAME, MD5_A, MD5_B, MD5_C, MD5_D] {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }
    assert_eq!(read.courses[0].hash(), format!("{:x}", hasher.finalize()));
}

#[test]
fn a_table_course_id_uses_the_sha256_when_the_header_gives_one() {
    let with_sha = courses_from_table_header(&json!({ "name": "pinned", "md5": [MD5_A], "sha256": [SHA256_A] }));
    let without = courses_from_table_header(&json!({ "name": "pinned", "md5": [MD5_A] }));
    assert_ne!(with_sha.courses[0].hash(), without.courses[0].hash());
}

#[test]
fn same_charts_ignores_name_and_case() {
    let upper = MD5_A.to_ascii_uppercase();
    assert!(same_charts(&course_named("one", &[MD5_A, MD5_B]), &course_named("another name", &[&upper, MD5_B])));
}

#[test]
fn same_charts_needs_the_same_charts_in_the_same_order() {
    let course = course_named("one", &[MD5_A, MD5_B]);
    assert!(!same_charts(&course, &course_named("one", &[MD5_B, MD5_A])), "order");
    assert!(!same_charts(&course, &course_named("one", &[MD5_A])), "length");
    assert!(!same_charts(&course, &course_named("one", &[MD5_A, MD5_C])), "a chart");
}

#[test]
fn courses_without_an_md5_are_never_the_same() {
    let nameless = Course { charts: vec![CourseChart { sha256: SHA256_A.to_string(), ..CourseChart::default() }], ..Course::default() };
    assert!(!same_charts(&nameless, &nameless.clone()));
    assert!(!same_charts(&Course::default(), &Course::default()));
}

#[test]
fn bundled_twin_finds_the_genoside_course_a_table_lists_under_another_name() {
    let listed = course_named("GENOSIDE 2018 初段", &GENOSIDE_FIRST_MD5S);
    let twin = bundled_twin(&listed).expect("the bundled first grade plays these charts");
    assert_eq!(twin.name, GENOSIDE_FIRST_NAME);
    assert_ne!(twin.hash(), listed.hash(), "the name is in the id, which is why the twin is taken");
}

#[test]
fn bundled_twin_finds_nothing_for_other_charts() {
    assert!(bundled_twin(&course_named("Stella", &[MD5_A, MD5_B, MD5_C, MD5_D])).is_none());
}
