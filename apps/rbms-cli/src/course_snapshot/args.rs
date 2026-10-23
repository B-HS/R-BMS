//! The command line of `course-snapshot`.

use std::path::PathBuf;

/// What a date given to `--retrieved-on` looks like once every digit is written as `D`.
const DATE_SHAPE: &str = "DDDD-DD-DD";

/// What `course-snapshot` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotArgs {
    /// The maker's course archive.
    pub zip: PathBuf,
    /// The header of the difficulty table whose courses carry the normal grades' constraints and trophies.
    pub normal_header: PathBuf,
    /// The same for the insane grades.
    pub insane_header: PathBuf,
    /// A JSON object from chart MD5 to the chart's title.
    pub titles: PathBuf,
    /// The day the input files were downloaded, written into the data as it is.
    pub retrieved_on: String,
    /// The data file.
    pub out: PathBuf,
    /// Compare instead of writing: the exit code says whether the file is what the inputs give.
    pub check: bool,
}

fn value_of<'a>(it: &mut impl Iterator<Item = &'a String>, option: &str) -> Result<&'a String, String> {
    it.next().ok_or_else(|| format!("{option} needs a value"))
}

fn is_date(text: &str) -> bool {
    text.chars().map(|c| if c.is_ascii_digit() { 'D' } else { c }).eq(DATE_SHAPE.chars())
}

/// `course-snapshot --zip <file> --normal-header <file> --insane-header <file> --titles <file>
/// --retrieved-on <YYYY-MM-DD> --out <file> [--check]`. Every option but `--check` is required, and
/// they may come in any order.
pub fn parse(rest: &[String]) -> Result<SnapshotArgs, String> {
    let mut zip: Option<PathBuf> = None;
    let mut normal_header: Option<PathBuf> = None;
    let mut insane_header: Option<PathBuf> = None;
    let mut titles: Option<PathBuf> = None;
    let mut retrieved_on: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut check = false;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--zip" => zip = Some(PathBuf::from(value_of(&mut it, "--zip")?)),
            "--normal-header" => normal_header = Some(PathBuf::from(value_of(&mut it, "--normal-header")?)),
            "--insane-header" => insane_header = Some(PathBuf::from(value_of(&mut it, "--insane-header")?)),
            "--titles" => titles = Some(PathBuf::from(value_of(&mut it, "--titles")?)),
            "--retrieved-on" => {
                let date = value_of(&mut it, "--retrieved-on")?;
                if !is_date(date) {
                    return Err(format!("--retrieved-on takes a date written {DATE_SHAPE}, not {date}"));
                }
                retrieved_on = Some(date.clone());
            }
            "--out" => out = Some(PathBuf::from(value_of(&mut it, "--out")?)),
            "--check" => check = true,
            other if other.starts_with("--") => return Err(format!("unknown option: {other}")),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    Ok(SnapshotArgs {
        zip: zip.ok_or("course-snapshot needs --zip <file>")?,
        normal_header: normal_header.ok_or("course-snapshot needs --normal-header <file>")?,
        insane_header: insane_header.ok_or("course-snapshot needs --insane-header <file>")?,
        titles: titles.ok_or("course-snapshot needs --titles <file>")?,
        retrieved_on: retrieved_on.ok_or("course-snapshot needs --retrieved-on <YYYY-MM-DD>")?,
        out: out.ok_or("course-snapshot needs --out <file>")?,
        check,
    })
}
