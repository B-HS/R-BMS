//! Courses as a difficulty table header lists them, and how they relate to the bundled ones.
//!
//! A table header's `course` field is the reference implementation's way of publishing a course
//! set (`TableDataAccessor.java:192-213`). Three shapes are in use and all three are read: a list
//! of groups, each a list of course objects (the shape the reference flattens); a flat list of
//! course objects; and one course object. A course names its charts either as a `charts` list
//! written like a course file's, or as `md5` (and optionally `sha256`) lists, which is what the
//! tables in circulation write.
//!
//! The crate does not depend on `rbms-table`, so the header arrives as the [`serde_json::Value`]
//! the table kept.

use serde::Deserialize;
use serde_json::Value;

use crate::bundled::genoside_2018;
use crate::model::{Course, CourseChart};

/// What a difficulty table header's `course` field lists, read as courses.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableCourses {
    /// Every course that survives [`Course::validate`], groups flattened, in the order written.
    pub courses: Vec<Course>,
    /// Entries left out: not an object, or a course with no chart or a chart with neither hash.
    pub skipped: usize,
}

/// A hash as the text a course file holds it in: a string as it is, a number as its digits (a hash
/// made only of digits may have been written as one). Anything else is no hash.
fn hash_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

fn hashes_of(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().map(hash_text).collect(),
        Some(single) => vec![hash_text(single)],
        None => Vec::new(),
    }
}

/// The charts of a course that wrote `md5` and `sha256` lists instead of a `charts` list: the n-th
/// of each is the n-th stage.
fn charts_of(md5s: Vec<String>, sha256s: Vec<String>) -> Vec<CourseChart> {
    (0..md5s.len().max(sha256s.len()))
        .map(|stage| CourseChart {
            md5: md5s.get(stage).cloned().unwrap_or_default(),
            sha256: sha256s.get(stage).cloned().unwrap_or_default(),
            title: String::new(),
        })
        .collect()
}

/// One course entry, or `None` when it is no course. Chart hashes are lower-cased whichever way the
/// charts were written, as the reference does when it turns a chart into a song
/// (`TableDataAccessor.java:233-237`).
fn course_of(entry: &Value) -> Option<Course> {
    let object = entry.as_object()?;
    let mut course = Course::deserialize(entry).ok()?;
    if course.charts.is_empty() {
        course.charts = charts_of(hashes_of(object.get("md5")), hashes_of(object.get("sha256")));
    }
    for chart in &mut course.charts {
        chart.md5.make_ascii_lowercase();
        chart.sha256.make_ascii_lowercase();
    }
    course.release = true;
    course.validate().then_some(course)
}

/// Read a table header's `course` field (`TableDataAccessor.java:192-213`).
///
/// A value that is neither a list nor an object counts as one skipped entry. The reference's table
/// parser is not in its source tree, so how much it forgives beyond these shapes is not known.
pub fn courses_from_table_header(course: &Value) -> TableCourses {
    let entries: Vec<&Value> = match course {
        Value::Array(items) => items
            .iter()
            .flat_map(|item| match item {
                Value::Array(group) => group.iter().collect(),
                other => vec![other],
            })
            .collect(),
        other => vec![other],
    };
    let courses: Vec<Course> = entries.iter().filter_map(|entry| course_of(entry)).collect();
    TableCourses { skipped: entries.len() - courses.len(), courses }
}

/// Whether two courses play the same charts in the same order, by MD5 (case-insensitive), whatever
/// their names. A chart without an MD5 is never the same as another.
pub fn same_charts(a: &Course, b: &Course) -> bool {
    !a.charts.is_empty()
        && a.charts.len() == b.charts.len()
        && a.charts.iter().zip(&b.charts).all(|(left, right)| !left.md5.is_empty() && left.md5.eq_ignore_ascii_case(&right.md5))
}

/// The bundled course that plays the same charts as `course`, if one does.
///
/// A table can list a course under a name of its own that is a bundled course all the same
/// (`GENOSIDE 2018 初段` for `GENOSIDE 2018 段位認定 初段`). The id of a course includes its name, so
/// the two would be ranked apart; taking the bundled one puts them together.
pub fn bundled_twin(course: &Course) -> Option<&'static Course> {
    genoside_2018().courses().map(|bundled| &bundled.course).find(|bundled| same_charts(bundled, course))
}

#[cfg(test)]
mod tests;
