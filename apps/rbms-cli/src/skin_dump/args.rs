//! The command line of `skin-dump`.

use std::path::PathBuf;

/// The seed a dump pins its draws with when `--seed` is not given: a random option, a random file
/// slot and a skin's own `math.random` all come out the same on every run.
pub const DEFAULT_SEED: u64 = 1;

/// Frames measured when `--frames` is not given: none, so a dump is a static inspection.
pub const DEFAULT_FRAMES: usize = 0;

/// Most frames `--frames` accepts: over four hours of scene at sixty frames a second, and few
/// enough that the time of every one of them is kept in memory to rank.
pub const MAX_FRAMES: usize = 1_000_000;

/// What `skin-dump` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpArgs {
    /// A skin pack folder, or one skin document.
    pub target: PathBuf,
    /// A scenario file that replaces the built-in scenario of every document.
    pub scenario: Option<PathBuf>,
    /// Where the skins' file writes go. A temporary folder when absent.
    pub overlay: Option<PathBuf>,
    /// How many frames to call every function value for.
    pub frames: usize,
    pub seed: u64,
    /// Machine-readable output instead of the tables.
    pub json: bool,
    /// Search for the smallest Lua instruction budgets each skin runs inside.
    pub probe_budget: bool,
    /// Fail when a document did not come out clean (see [`super::gate`]), so the dump can stand as
    /// a gate rather than only be read.
    pub strict: bool,
    /// The file names of the documents that are expected not to load, which a strict dump holds to
    /// exactly that: one that loads after all is as much a failure as another that does not.
    pub allowed_failures: Vec<String>,
}

fn frames_of(text: &str) -> Result<usize, String> {
    let frames: usize = text.parse().map_err(|e| format!("--frames needs a whole number: {e}"))?;
    if frames > MAX_FRAMES {
        return Err(format!("--frames takes at most {MAX_FRAMES}, not {frames}"));
    }
    Ok(frames)
}

fn value_of<'a>(it: &mut impl Iterator<Item = &'a String>, option: &str) -> Result<&'a String, String> {
    it.next().ok_or_else(|| format!("{option} needs a value"))
}

/// `skin-dump <pack folder or document> [--scenario <json>] [--overlay <folder>] [--frames <N>] [--seed <n>] [--json] [--probe-budget] [--strict]
/// [--allow-failure <document>]...`. Naming a document that may fail asks for a strict dump.
pub fn parse(rest: &[String]) -> Result<DumpArgs, String> {
    let mut target: Option<PathBuf> = None;
    let mut args = DumpArgs {
        target: PathBuf::new(),
        scenario: None,
        overlay: None,
        frames: DEFAULT_FRAMES,
        seed: DEFAULT_SEED,
        json: false,
        probe_budget: false,
        strict: false,
        allowed_failures: Vec::new(),
    };
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--scenario" => args.scenario = Some(PathBuf::from(value_of(&mut it, "--scenario")?)),
            "--overlay" => args.overlay = Some(PathBuf::from(value_of(&mut it, "--overlay")?)),
            "--frames" => args.frames = frames_of(value_of(&mut it, "--frames")?)?,
            "--seed" => args.seed = value_of(&mut it, "--seed")?.parse().map_err(|e| format!("--seed needs a whole number: {e}"))?,
            "--json" => args.json = true,
            "--probe-budget" => args.probe_budget = true,
            "--strict" => args.strict = true,
            "--allow-failure" => {
                args.allowed_failures.push(value_of(&mut it, "--allow-failure")?.clone());
                args.strict = true;
            }
            other if other.starts_with("--") => return Err(format!("unknown option: {other}")),
            other if target.is_none() => target = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    args.target = target.ok_or("skin-dump needs a skin pack folder or a skin document")?;
    Ok(args)
}
