use std::collections::HashSet;

use super::*;
use crate::model::CourseChart;

const COURSES_PER_GROUP: usize = 22;
const CHARTS_PER_COURSE: usize = 4;
const MD5_LENGTH: usize = 32;
const SHA256_LENGTH: usize = 64;
const SINGLE_PLAY_KEYS: u32 = 7;
const DOUBLE_PLAY_KEYS: u32 = 14;

const FIRST_COURSE_NAME: &str = "GENOSIDE 2018 段位認定 初段";
const FIRST_COURSE_MD5S: [&str; CHARTS_PER_COURSE] =
    ["ae11930223a8f24e91633f1a7864e1eb", "cb661ee05c67990012bfd12703ba656c", "328dbf235f3b6cb2791e71bfb62951f8", "7c6ca04aa14d07b2c081f4dd35432844"];
const FIRST_COURSE_ID: &str = "ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1";

const OFFICIAL_FILE_BYTES: u64 = 4084;
const OFFICIAL_FILE_SHA256: &str = "c426719dcf13c279fe003fcd80258fd625c3b796e7c5adda21abec1655df65ca";
const OFFICIAL_ENTRY_SHA256: &str = "9d7981aeab8259d96349cc32237164ba1798344e6372cb6c7ee3bb83b417ffb7";
const OFFICIAL_HASH_PREFIX: &str = "00000000002000000000000000005190";
const OFFICIAL_ENTRY_TYPE: u32 = 2;

/// The grades in the order the data climbs them: the ten ordinary grades, the ten insane ones, the
/// top insane grade and the course above it.
const GRADES: [&str; COURSES_PER_GROUP] = [
    "初段",
    "二段",
    "三段",
    "四段",
    "五段",
    "六段",
    "七段",
    "八段",
    "九段",
    "十段",
    "発狂初段",
    "発狂二段",
    "発狂三段",
    "発狂四段",
    "発狂五段",
    "発狂六段",
    "発狂七段",
    "発狂八段",
    "発狂九段",
    "発狂十段",
    "発狂皆伝",
    "Overjoy",
];

/// Grades from the first that the normal table gives three medals; the rest give two.
const THREE_MEDAL_GRADES: usize = 10;

fn bundled() -> &'static BundledCourses {
    genoside_2018()
}

fn as_written() -> BundledCourses {
    serde_json::from_str(GENOSIDE_2018_JSON).expect("the data file parses")
}

fn medals(course: &Course) -> Vec<(&str, f32, f32)> {
    course.trophies.iter().map(|trophy| (trophy.name.as_str(), trophy.missrate, trophy.scorerate)).collect()
}

#[test]
fn the_bundled_genoside_2018_has_22_sp_and_22_dp_courses() {
    let groups = &bundled().groups;
    assert_eq!(groups.iter().map(|group| group.id.as_str()).collect::<Vec<_>>(), ["genoside-2018-sp", "genoside-2018-dp"]);
    assert_eq!(groups.iter().map(|group| group.keys).collect::<Vec<_>>(), [SINGLE_PLAY_KEYS, DOUBLE_PLAY_KEYS]);
    assert!(groups.iter().all(|group| group.courses.len() == COURSES_PER_GROUP));
    assert_eq!(bundled().courses().count(), 2 * COURSES_PER_GROUP);
}

#[test]
fn every_bundled_course_has_four_lowercase_md5s_and_no_sha256() {
    for bundled in bundled().courses() {
        let course = &bundled.course;
        assert_eq!(course.charts.len(), CHARTS_PER_COURSE, "{}", course.name);
        for chart in &course.charts {
            assert_eq!(chart.md5.len(), MD5_LENGTH, "{}: {}", course.name, chart.md5);
            assert!(chart.md5.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)), "{}: {}", course.name, chart.md5);
            assert!(chart.sha256.is_empty(), "{}: a stated sha256 would change the id", course.name);
        }
    }
}

#[test]
fn every_bundled_course_hash_is_the_one_its_file_states() {
    for bundled in bundled().courses() {
        assert_eq!(bundled.course.hash(), bundled.course_hash, "{}", bundled.course.name);
    }
}

#[test]
fn no_two_bundled_courses_share_an_id() {
    let ids: HashSet<&str> = bundled().courses().map(|bundled| bundled.course_hash.as_str()).collect();
    assert_eq!(ids.len(), 2 * COURSES_PER_GROUP);
}

#[test]
fn the_first_sp_course_keeps_its_id() {
    let first = &bundled().groups[0].courses[0];
    assert_eq!(first.course.name, FIRST_COURSE_NAME);
    assert_eq!(first.course.charts.iter().map(|chart| chart.md5.as_str()).collect::<Vec<_>>(), FIRST_COURSE_MD5S);
    assert_eq!(first.course_hash, FIRST_COURSE_ID);

    let rebuilt = Course {
        name: FIRST_COURSE_NAME.to_string(),
        charts: FIRST_COURSE_MD5S.iter().map(|md5| CourseChart { md5: (*md5).to_string(), ..CourseChart::default() }).collect(),
        ..Course::default()
    };
    assert_eq!(rebuilt.hash(), FIRST_COURSE_ID, "the rule that makes the id still gives it");
}

#[test]
fn every_bundled_course_plays_mirror_lr2_ln() {
    for bundled in bundled().courses() {
        assert_eq!(bundled.course.constraints, [CourseConstraint::Mirror, CourseConstraint::GaugeLr2, CourseConstraint::Ln], "{}", bundled.course.name);
        assert!(bundled.course.release, "{}", bundled.course.name);
    }
}

#[test]
fn bundled_trophies_follow_the_beatoraja_tables() {
    let three = [("bronzemedal", 7.5, 0.0), ("silvermedal", 5.0, 70.0), ("goldmedal", 2.5, 85.0)];
    for group in &bundled().groups {
        for (index, bundled) in group.courses.iter().enumerate() {
            let expected = if index < THREE_MEDAL_GRADES { &three[..] } else { &three[1..] };
            assert_eq!(medals(&bundled.course), expected, "{}", bundled.course.name);
        }
    }
}

#[test]
fn the_bundled_courses_climb_from_shodan_to_overjoy() {
    let prefixes = ["GENOSIDE 2018 段位認定 ", "GENOSIDE 2018 DP段位認定 "];
    for (group, prefix) in bundled().groups.iter().zip(prefixes) {
        let names: Vec<&str> = group.courses.iter().map(|bundled| bundled.course.name.as_str()).collect();
        let expected: Vec<String> = GRADES.iter().map(|grade| format!("{prefix}{grade}")).collect();
        assert_eq!(names, expected.iter().map(String::as_str).collect::<Vec<_>>(), "{}", group.id);
    }
}

#[test]
fn a_double_play_course_has_the_rules_of_the_single_play_course_of_its_grade() {
    let [singles, doubles] = &bundled().groups[..] else { panic!("two groups") };
    for (single, double) in singles.courses.iter().zip(&doubles.courses) {
        assert_eq!(double.course.constraints, single.course.constraints, "{}", double.course.name);
        assert_eq!(double.course.trophies, single.course.trophies, "{}", double.course.name);
    }
}

#[test]
fn every_single_play_chart_has_a_title() {
    for bundled in &as_written().groups[0].courses {
        for chart in &bundled.course.charts {
            assert!(!chart.title.is_empty(), "{}: {} has none", bundled.course.name, chart.md5);
        }
    }
}

#[test]
fn the_bundled_source_names_genocide_and_its_file() {
    let source = &bundled().source;
    assert_eq!(source.maker, "GENOCIDE");
    assert!(source.file_url.ends_with("/genoside/2018_course.zip"), "{}", source.file_url);
    assert_eq!(source.file_bytes, OFFICIAL_FILE_BYTES);
    assert_eq!(source.file_sha256, OFFICIAL_FILE_SHA256);
    assert_eq!(source.entry_name, "test.lr2crs");
    assert_eq!(source.entry_sha256, OFFICIAL_ENTRY_SHA256);
    assert_eq!(source.entry_hash_prefix, OFFICIAL_HASH_PREFIX);
    assert_eq!(source.entry_type, OFFICIAL_ENTRY_TYPE);
    assert_eq!(source.constraint_sources.len(), 2, "the normal table and the insane table");
    assert!(source.constraint_sources.iter().all(|file| file.sha256.len() == SHA256_LENGTH && !file.url.is_empty() && !file.retrieved_on.is_empty()));
}

#[test]
fn every_bundled_course_validates_unchanged() {
    for written in as_written().courses() {
        let mut course = written.course.clone();
        assert!(course.validate(), "{}", course.name);
        assert_eq!(course.name, written.course.name);
        assert_eq!(course.constraints, written.course.constraints);
        assert_eq!(course.trophies, written.course.trophies);
        assert_eq!(course.hash(), written.course_hash, "validation names the untitled charts and nothing that goes into the id");
    }
}

#[test]
fn the_data_file_reads_back_as_the_text_it_is() {
    let mut text = serde_json::to_string_pretty(&as_written()).expect("the data serialises");
    text.push('\n');
    assert_eq!(text, GENOSIDE_2018_JSON, "the generator writes this file with the same serialisation");
}
