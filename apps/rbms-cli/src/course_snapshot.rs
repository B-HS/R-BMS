//! `course-snapshot`: writes the data file of the GENOSIDE 2018 grade courses the player carries,
//! `web/data/courses/genoside-2018/courses.json`, from the maker's course archive.
//!
//! Nothing is fetched. The maintainer downloads the inputs, and the data names which files they
//! were (their sizes and SHA-256) and where they came from, so the same inputs always give the same
//! bytes and `--check` can tell whether the committed file still is what its inputs give.
//!
//! The course archive holds one `.lr2crs` list of the single-play and double-play courses, with
//! each course's name and chart MD5s and nothing else. The constraints and trophies of the
//! single-play courses are those of the courses that play the same charts in beatoraja's default
//! difficulty tables; the double-play courses have no such source and take those of the
//! single-play course of the same grade. The list runs from the hardest grade down and the data
//! from the first grade up, as the tables show them.
//!
//! The titles of the charts are not in the archive. They come from a JSON object from MD5 to title
//! that the maintainer makes, and a chart the object does not name is written without a title.
//! Titles are shown to the player and are not part of a course's id.
//!
//! Exit code: 0 when written, or with `--check` when the file is what the inputs give; 1 with
//! `--check` when it is not; 2 when the inputs could not be read or the data could not be made.

mod args;
mod lr2crs;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;
use std::process::ExitCode;

use rbms_course::bundled::{BundledCourse, BundledCourses, BundledGroup, BundledSource, SourceFile};
use rbms_course::{Course, CourseChart, CourseConstraint, TrophyRule, courses_from_table_header, same_charts};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

pub use args::SnapshotArgs;
pub use args::parse as parse_args;

use lr2crs::{Lr2crsCourse, decode, parse as parse_lr2crs};

/// The version of the data file's layout.
const SCHEMA_VERSION: u32 = 1;

const MAKER: &str = "GENOCIDE";
const PAGE_URL: &str = "https://nekokan.dyndns.info/~lobsak/genocide/grade.html";
const FILE_URL: &str = "http://nekokan.dyndns.info/~lobsak/genoside/2018_course.zip";
const NORMAL_HEADER_URL: &str = "https://miraiscarlet.github.io/bms/table/genocide_normal/header_normal.json";
const INSANE_HEADER_URL: &str = "https://miraiscarlet.github.io/bms/table/genocide_insane/header_insane.json";
const TITLE_SOURCE: &str = "https://www.bms-ir.org/new/course?hash= (chart titles only, for display)";

/// Bytes a difficulty table header may start with that are not JSON.
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// The most bytes the course list may unpack to. The real one is a few kilobytes.
const ENTRY_MAX_BYTES: u64 = 1 << 20;

const EXIT_DIFFERENT: u8 = 1;
const EXIT_FAILED: u8 = 2;

/// One play style of the grade set, as the course list marks it by the number of keys.
struct GroupSpec {
    id: &'static str,
    name: &'static str,
    line: u32,
}

const SINGLE_PLAY: GroupSpec = GroupSpec { id: "genoside-2018-sp", name: "GENOSIDE 2018 段位認定", line: 7 };
const DOUBLE_PLAY: GroupSpec = GroupSpec { id: "genoside-2018-dp", name: "GENOSIDE 2018 DP段位認定", line: 14 };

/// The files a snapshot is made from, as bytes.
pub struct Inputs<'a> {
    pub zip: &'a [u8],
    pub normal_header: &'a [u8],
    pub insane_header: &'a [u8],
    pub titles: &'a [u8],
    pub retrieved_on: &'a str,
}

/// The data and what the run found worth saying about it.
#[derive(Debug)]
pub struct Snapshot {
    pub courses: BundledCourses,
    /// Charts the titles file does not name, written with an empty title.
    pub untitled_charts: usize,
}

type Rules = (Vec<CourseConstraint>, Vec<TrophyRule>);

fn sha256_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn without_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes)
}

/// The only file in the archive, with its name.
fn sole_entry(zip: &[u8]) -> Result<(String, Vec<u8>), String> {
    let mut archive = ZipArchive::new(Cursor::new(zip)).map_err(|e| format!("the course archive does not open: {e}"))?;
    if archive.len() != 1 {
        return Err(format!("the course archive holds {} entries, not one", archive.len()));
    }
    let entry = archive.by_index(0).map_err(|e| format!("the course archive's entry does not open: {e}"))?;
    let name = entry.name().map_err(|e| format!("the course archive's entry has no readable name: {e}"))?.into_owned();
    if !entry.is_file() || entry.size() > ENTRY_MAX_BYTES {
        return Err(format!("the course archive's entry {name:?} is not a file of at most {ENTRY_MAX_BYTES} bytes"));
    }
    let mut bytes = Vec::new();
    entry.take(ENTRY_MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| format!("the course archive's entry {name:?} is not read: {e}"))?;
    Ok((name, bytes))
}

/// The courses a difficulty table header lists.
fn header_courses(bytes: &[u8], name: &str) -> Result<Vec<Course>, String> {
    let header: Value = serde_json::from_slice(without_bom(bytes)).map_err(|e| format!("the {name} header is not JSON: {e}"))?;
    let listed = header.get("course").ok_or_else(|| format!("the {name} header lists no course"))?;
    Ok(courses_from_table_header(listed).courses)
}

fn titles_of(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let titles: BTreeMap<String, String> = serde_json::from_slice(bytes).map_err(|e| format!("the titles file is not an object from MD5 to title: {e}"))?;
    Ok(titles.into_iter().map(|(md5, title)| (md5.to_ascii_lowercase(), title)).collect())
}

/// What a grade is called: the last word of a course's name.
fn grade_of(name: &str) -> &str {
    name.rsplit_once(' ').map_or(name, |(_, grade)| grade)
}

fn rules_from_tables(tables: &[Course], played: &Course) -> Result<Rules, String> {
    tables
        .iter()
        .find(|table| same_charts(table, played))
        .map(|table| (table.constraints.clone(), table.trophies.clone()))
        .ok_or_else(|| format!("no course of the difficulty tables plays the charts of {:?}", played.name))
}

fn group_of(
    spec: &GroupSpec,
    listed: &[Lr2crsCourse],
    titles: &BTreeMap<String, String>,
    rules_for: &dyn Fn(&Course) -> Result<Rules, String>,
) -> Result<BundledGroup, String> {
    let mut courses = Vec::new();
    for official in listed.iter().filter(|course| course.line == spec.line).rev() {
        let charts =
            official.md5s().into_iter().map(|md5| CourseChart { title: titles.get(&md5).cloned().unwrap_or_default(), sha256: String::new(), md5 }).collect();
        let mut course = Course { name: official.title.clone(), charts, ..Course::default() };
        (course.constraints, course.trophies) = rules_for(&course)?;
        courses.push(BundledCourse { course_hash: course.hash(), course });
    }
    Ok(BundledGroup { id: spec.id.to_string(), name: spec.name.to_string(), keys: spec.line, courses })
}

/// Makes the data from its input files.
pub fn build(inputs: &Inputs) -> Result<Snapshot, String> {
    let (entry_name, entry) = sole_entry(inputs.zip)?;
    let listed = parse_lr2crs(&decode(&entry)?)?;
    if let Some(other) = listed.iter().find(|course| course.line != SINGLE_PLAY.line && course.line != DOUBLE_PLAY.line) {
        return Err(format!("{:?} is for {} keys, and only {} and {} are known", other.title, other.line, SINGLE_PLAY.line, DOUBLE_PLAY.line));
    }
    let (entry_hash_prefix, entry_type) = (listed[0].hash_prefix().to_string(), listed[0].kind);
    if let Some(other) = listed.iter().find(|course| course.hash_prefix() != entry_hash_prefix || course.kind != entry_type) {
        return Err(format!("{:?} does not share the hash prefix {entry_hash_prefix} and type {entry_type} of the first course", other.title));
    }

    let mut tables = header_courses(inputs.normal_header, "normal")?;
    tables.extend(header_courses(inputs.insane_header, "insane")?);
    let titles = titles_of(inputs.titles)?;

    let singles = group_of(&SINGLE_PLAY, &listed, &titles, &|course| rules_from_tables(&tables, course))?;
    let grade_rules: Vec<(&str, Rules)> =
        singles.courses.iter().map(|single| (grade_of(&single.course.name), (single.course.constraints.clone(), single.course.trophies.clone()))).collect();
    let doubles = group_of(&DOUBLE_PLAY, &listed, &titles, &|course| {
        grade_rules
            .iter()
            .find(|(grade, _)| *grade == grade_of(&course.name))
            .map(|(_, rules)| rules.clone())
            .ok_or_else(|| format!("no single-play course is of the grade of {:?}", course.name))
    })?;

    let groups = vec![singles, doubles];
    let untitled_charts =
        groups.iter().flat_map(|group| &group.courses).flat_map(|bundled| &bundled.course.charts).filter(|chart| chart.title.is_empty()).count();
    let source = BundledSource {
        maker: MAKER.to_string(),
        page_url: PAGE_URL.to_string(),
        file_url: FILE_URL.to_string(),
        retrieved_on: inputs.retrieved_on.to_string(),
        file_bytes: inputs.zip.len() as u64,
        file_sha256: sha256_of(inputs.zip),
        entry_name,
        entry_sha256: sha256_of(&entry),
        entry_hash_prefix,
        entry_type,
        constraint_sources: [(NORMAL_HEADER_URL, inputs.normal_header), (INSANE_HEADER_URL, inputs.insane_header)]
            .into_iter()
            .map(|(url, bytes)| SourceFile { url: url.to_string(), sha256: sha256_of(bytes), retrieved_on: inputs.retrieved_on.to_string() })
            .collect(),
        title_source: TITLE_SOURCE.to_string(),
    };
    Ok(Snapshot { courses: BundledCourses { schema: SCHEMA_VERSION, source, groups }, untitled_charts })
}

/// The text of the data file: pretty JSON in the order the types declare their fields, and a final
/// line break.
pub fn render(courses: &BundledCourses) -> Result<String, String> {
    let mut text = serde_json::to_string_pretty(courses).map_err(|e| format!("the data is not serialised: {e}"))?;
    text.push('\n');
    Ok(text)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{} is not read: {e}", path.display()))
}

fn execute(args: &SnapshotArgs) -> Result<ExitCode, String> {
    let (zip, normal_header, insane_header, titles) = (read(&args.zip)?, read(&args.normal_header)?, read(&args.insane_header)?, read(&args.titles)?);
    let snapshot =
        build(&Inputs { zip: &zip, normal_header: &normal_header, insane_header: &insane_header, titles: &titles, retrieved_on: &args.retrieved_on })?;
    let text = render(&snapshot.courses)?;
    if snapshot.untitled_charts > 0 {
        eprintln!("{} charts are not named by the titles file and are written without a title", snapshot.untitled_charts);
    }
    let is_current = std::fs::read(&args.out).is_ok_and(|existing| existing == text.as_bytes());
    let count: usize = snapshot.courses.groups.iter().map(|group| group.courses.len()).sum();
    if args.check {
        if is_current {
            println!("{} is what the inputs give ({count} courses)", args.out.display());
            return Ok(ExitCode::SUCCESS);
        }
        eprintln!("{} is not what the inputs give", args.out.display());
        return Ok(ExitCode::from(EXIT_DIFFERENT));
    }
    if is_current {
        println!("{} is already what the inputs give ({count} courses)", args.out.display());
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(folder) = args.out.parent() {
        std::fs::create_dir_all(folder).map_err(|e| format!("{} is not made: {e}", folder.display()))?;
    }
    std::fs::write(&args.out, text).map_err(|e| format!("{} is not written: {e}", args.out.display()))?;
    println!("wrote {} ({count} courses)", args.out.display());
    Ok(ExitCode::SUCCESS)
}

/// Runs `course-snapshot`, writing or checking the data file.
pub fn run(args: &SnapshotArgs) -> ExitCode {
    execute(args).unwrap_or_else(|reason| {
        eprintln!("{reason}");
        ExitCode::from(EXIT_FAILED)
    })
}
