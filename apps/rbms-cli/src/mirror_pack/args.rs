//! The command line of `mirror-pack`.

use std::path::PathBuf;

/// What `mirror-pack` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorPackArgs {
    /// The folder whose song folders are packed, walked to any depth.
    pub library: PathBuf,
    /// The folder the packs and the manifest are written to. Made when it is not there.
    pub out: PathBuf,
    /// Difficulty table bodies (`data.json`): a song folder that holds a chart any of them lists is packed.
    pub tables: Vec<PathBuf>,
    /// Pack every song folder of the library, whatever the tables list.
    pub all: bool,
    /// Report what would be packed and write nothing.
    pub dry_run: bool,
}

fn value_of<'a>(it: &mut impl Iterator<Item = &'a String>, option: &str) -> Result<&'a String, String> {
    it.next().ok_or_else(|| format!("{option} needs a value"))
}

/// `mirror-pack <library folder> --out <folder> (--table <data.json>)... [--all] [--dry-run]`. The
/// library is positional and the options may stand on either side of it. Which song folders to pack
/// must be said, by a table or by `--all`, so that a library is never packed whole by leaving it out.
pub fn parse(rest: &[String]) -> Result<MirrorPackArgs, String> {
    let mut library: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut tables = Vec::new();
    let mut all = false;
    let mut dry_run = false;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(value_of(&mut it, "--out")?)),
            "--table" => tables.push(PathBuf::from(value_of(&mut it, "--table")?)),
            "--all" => all = true,
            "--dry-run" => dry_run = true,
            other if other.starts_with("--") => return Err(format!("unknown option: {other}")),
            other if library.is_none() => library = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    if tables.is_empty() && !all {
        return Err("mirror-pack needs --table <data.json> or --all".to_string());
    }
    Ok(MirrorPackArgs {
        library: library.ok_or("mirror-pack needs a library folder")?,
        out: out.ok_or("mirror-pack needs --out <folder>")?,
        tables,
        all,
        dry_run,
    })
}
