//! Reading and writing course files.
//!
//! A course file is JSON holding either an array of courses or a single course, with any field this
//! build does not know ignored, and every course normalised by [`Course::validate`] before it is
//! kept (`CourseDataAccessor.java:44-68`). Reading the array shape first and falling back to the
//! single-object shape is what lets both hand-written and table-published files load.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::Course;

/// Extension a course file carries (`CourseDataAccessor.java:45`).
pub const COURSE_FILE_EXTENSION: &str = "json";

/// File stem used when a course name has no character a file name may keep.
const FALLBACK_FILE_STEM: &str = "course";

/// Character substituted for anything a file stem may not carry.
const FILE_STEM_REPLACEMENT: char = '_';

/// Parse one course file's text, normalising every course it holds and keeping the ones that
/// survive [`Course::validate`].
///
/// The array shape is tried first and the single-object shape second, so a file holding one course
/// object loads as a one-course list. Text that is neither yields no courses.
pub fn parse(text: &str) -> Vec<Course> {
    if let Ok(list) = serde_json::from_str::<Vec<Course>>(text) {
        return list.into_iter().filter_map(validated).collect();
    }
    match serde_json::from_str::<Course>(text) {
        Ok(course) => validated(course).into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

/// Read one course file. A file that cannot be read is an error; a file that cannot be parsed is an
/// empty list, the same way the reference treats one.
pub fn load_file(path: &Path) -> io::Result<Vec<Course>> {
    Ok(parse(&fs::read_to_string(path)?))
}

/// Read every course file in `dir`, in file-name order so the browser lists them the same way
/// twice running. A directory that is not there yet — the normal state before the player has any
/// courses — is an empty list, not an error.
pub fn load_dir(dir: &Path) -> Vec<Course> {
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case(COURSE_FILE_EXTENSION)))
            .collect(),
        Err(_) => return Vec::new(),
    };
    files.sort();
    files.iter().filter_map(|path| load_file(path).ok()).flatten().collect()
}

/// Write `course` into `dir` as its own file, named after the course, in the array shape the
/// reference reads first. The directory is created when it is not there yet.
pub fn save(dir: &Path, course: &Course) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(std::slice::from_ref(course)).map_err(io::Error::other)?;
    fs::write(dir.join(file_name_for(&course.name)), text)
}

/// The file name a course of this name is saved under: the name with everything a path may not
/// carry replaced, plus the extension.
pub fn file_name_for(name: &str) -> String {
    let stem: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { FILE_STEM_REPLACEMENT }).collect();
    let stem = stem.trim_matches(FILE_STEM_REPLACEMENT);
    let stem = if stem.is_empty() { FALLBACK_FILE_STEM } else { stem };
    format!("{stem}.{COURSE_FILE_EXTENSION}")
}

/// Normalise a course and keep it only if it survives.
fn validated(mut course: Course) -> Option<Course> {
    course.validate().then_some(course)
}
