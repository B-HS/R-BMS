//! The courses the build carries: the GENOSIDE 2018 grade courses, single-player and double-player.
//!
//! The data is `web/data/courses/genoside-2018/courses.json`, written by `rbms-cli course-snapshot`
//! from the maker's official course file and read here, by the website and by nothing else. The
//! file names where every part of it came from, so a course can be traced to its source and taken
//! out of the build when its maker asks.
//!
//! A course is identified by the chart hashes the official file gives (MD5 only, so
//! [`Course::hash`] is computed from them) and by its name as the official file spells it. The
//! file states that id beside each course so the website can register a course without
//! reimplementing the rule, and a test keeps the stated id equal to the computed one.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize, Serializer};

use crate::model::{Course, CourseConstraint, TrophyRule};

const GENOSIDE_2018_JSON: &str = include_str!("../../../web/data/courses/genoside-2018/courses.json");

/// The courses a build carries, with the provenance of the file they were read from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BundledCourses {
    pub schema: u32,
    pub source: BundledSource,
    pub groups: Vec<BundledGroup>,
}

impl BundledCourses {
    /// Every course, group after group, in the order the file lists them.
    pub fn courses(&self) -> impl Iterator<Item = &BundledCourse> {
        self.groups.iter().flat_map(|group| group.courses.iter())
    }
}

/// Where the data was taken from: the maker's course file and the difficulty tables whose courses
/// supplied the constraints and trophies the course file does not carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundledSource {
    pub maker: String,
    pub page_url: String,
    pub file_url: String,
    pub retrieved_on: String,
    pub file_bytes: u64,
    pub file_sha256: String,
    pub entry_name: String,
    pub entry_sha256: String,
    /// The first 32 characters every `<hash>` of the course file starts with, before the chart MD5s.
    pub entry_hash_prefix: String,
    /// The `<type>` every course of the course file states.
    pub entry_type: u32,
    pub constraint_sources: Vec<SourceFile>,
    pub title_source: String,
}

/// One downloaded file the data was derived from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub url: String,
    pub sha256: String,
    pub retrieved_on: String,
}

/// Courses listed together: one play style of one grade set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BundledGroup {
    pub id: String,
    pub name: String,
    /// How many keys the charts of the group have: 7 for single play, 14 for double play.
    pub keys: u32,
    pub courses: Vec<BundledCourse>,
}

/// A course and the id the data file states for it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BundledCourse {
    pub course_hash: String,
    #[serde(flatten)]
    pub course: Course,
}

#[derive(Serialize)]
struct ChartWire<'a> {
    md5: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    sha256: &'a str,
    title: &'a str,
}

#[derive(Serialize)]
struct CourseWire<'a> {
    course_hash: &'a str,
    name: &'a str,
    charts: Vec<ChartWire<'a>>,
    constraint: &'a [CourseConstraint],
    trophy: &'a [TrophyRule],
    release: bool,
}

impl Serialize for BundledCourse {
    /// Writes the spelling difficulty tables publish a course in (`constraint`, `trophy`, and a
    /// chart without a `sha256` when it has none), which [`Course`]'s own serialisation does not.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let course = &self.course;
        CourseWire {
            course_hash: &self.course_hash,
            name: &course.name,
            charts: course.charts.iter().map(|chart| ChartWire { md5: &chart.md5, sha256: &chart.sha256, title: &chart.title }).collect(),
            constraint: &course.constraints,
            trophy: &course.trophies,
            release: course.release,
        }
        .serialize(serializer)
    }
}

/// The GENOSIDE 2018 grade courses, single play then double play, parsed once from the file the
/// build carries. Every course has been through [`Course::validate`], so a chart the file leaves
/// untitled is named `course {n}`.
pub fn genoside_2018() -> &'static BundledCourses {
    static PARSED: OnceLock<BundledCourses> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut parsed: BundledCourses = serde_json::from_str(GENOSIDE_2018_JSON).expect("bundled GENOSIDE 2018 courses parse");
        for bundled in parsed.groups.iter_mut().flat_map(|group| group.courses.iter_mut()) {
            assert!(bundled.course.validate(), "bundled course {:?} validates", bundled.course.name);
        }
        parsed
    })
}

#[cfg(test)]
mod tests;
