//! Course and grade data: the ordered chart list, its constraints and trophies, and the run state
//! that carries gauge and combo across stages.
//!
//! The model mirrors the reference implementation's `CourseData` so a course file written for it
//! loads here unchanged — the same fourteen constraint tokens, the same normalise-rather-than-reject
//! validation, and the same trophy qualification arithmetic. [`Course::hash`] is rbms's own, being
//! the id the IR superset ranks a course under.
//!
//! The crate holds no play, judge or IR types: a finished stage arrives as a [`StageResult`] of
//! plain numbers, and the integration layer is what fills that in.
//!
//! Two sources of courses besides the player's own files live here: the GENOSIDE 2018 grade
//! courses the build carries ([`bundled`]), and the courses a difficulty table header lists
//! ([`table`]), which can be matched against the bundled ones.

#![forbid(unsafe_code)]

pub mod bundled;
pub mod load;
pub mod model;
pub mod run;
pub mod table;

#[cfg(test)]
mod tests;

pub use bundled::{BundledCourse, BundledCourses, BundledGroup, BundledSource, SourceFile, genoside_2018};
pub use load::{COURSE_FILE_EXTENSION, file_name_for, load_dir, load_file, parse, save};
pub use model::{CONSTRAINT_GROUP_COUNT, Course, CourseChart, CourseConstraint, DEFAULT_COURSE_NAME, TrophyRule};
pub use run::{CLEAR_FAILED, CLEAR_NO_PLAY, CourseRun, CourseStep, CourseTotals, StageResult};
pub use table::{TableCourses, bundled_twin, courses_from_table_header, same_charts};
