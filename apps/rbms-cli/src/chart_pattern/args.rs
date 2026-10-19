//! The command line of `chart-pattern`.

use std::path::PathBuf;

use rbms_model::Mode;

/// The mode `--mode` takes when it is not given: the one the difficulty tables this was made for
/// are all in.
pub const DEFAULT_MODE: Mode = Mode::BEAT_7K;

/// What `chart-pattern` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternArgs {
    /// The folder whose charts are read, walked to any depth.
    pub library: PathBuf,
    /// The folder the pattern files are written to. Made when it is not there.
    pub out: PathBuf,
    /// A difficulty table body (`data.json`) to report coverage against.
    pub table: Option<PathBuf>,
    /// Only the charts of this mode are written.
    pub mode: Mode,
}

/// How a mode is spelled on the command line: its name in lower case with hyphens, `beat-7k`.
pub fn mode_flag(mode: &Mode) -> String {
    mode.name.to_ascii_lowercase().replace('_', "-")
}

fn selectable_modes() -> impl Iterator<Item = &'static Mode> {
    Mode::ALL.iter().chain(std::iter::once(&Mode::KEYBOARD_24K))
}

fn mode_of(text: &str) -> Result<Mode, String> {
    let wanted = text.to_ascii_lowercase();
    selectable_modes().find(|mode| mode_flag(mode) == wanted).copied().ok_or_else(|| {
        let names: Vec<String> = selectable_modes().map(mode_flag).collect();
        format!("--mode takes one of {}, not {text}", names.join(", "))
    })
}

fn value_of<'a>(it: &mut impl Iterator<Item = &'a String>, option: &str) -> Result<&'a String, String> {
    it.next().ok_or_else(|| format!("{option} needs a value"))
}

/// `chart-pattern <library folder> --out <folder> [--table <data.json>] [--mode <mode>]`. The library
/// is positional and the options may stand on either side of it.
pub fn parse(rest: &[String]) -> Result<PatternArgs, String> {
    let mut library: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut table: Option<PathBuf> = None;
    let mut mode = DEFAULT_MODE;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(value_of(&mut it, "--out")?)),
            "--table" => table = Some(PathBuf::from(value_of(&mut it, "--table")?)),
            "--mode" => mode = mode_of(value_of(&mut it, "--mode")?)?,
            other if other.starts_with("--") => return Err(format!("unknown option: {other}")),
            other if library.is_none() => library = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    Ok(PatternArgs { library: library.ok_or("chart-pattern needs a library folder")?, out: out.ok_or("chart-pattern needs --out <folder>")?, table, mode })
}
