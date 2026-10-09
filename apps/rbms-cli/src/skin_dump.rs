//! `skin-dump`: loads skin documents without a game and reports what they came to.
//!
//! For a skin pack folder every `.luaskin`, `.json` and `.json5` in it is loaded; for a file, that
//! file. Each document is loaded against a scenario (see [`scenario`]), the way the game would load
//! it when entering its screen, and the report says what its header declares, how many objects and
//! function values the body made, whether the files it names are there, what went wrong inside it,
//! and with `--frames` what calling every function value once per frame costs.
//!
//! A skin never writes into its own folder. Its writes go to an overlay folder, a temporary one
//! unless `--overlay` names another, and the pack folder is listed before and after to prove it.
//!
//! Read by a person, a dump always succeeds as long as the pack folder was left alone: a document
//! that cannot load is a finding, not a failure of the tool. With `--strict` the same findings fail
//! the run (see [`gate`]), which is what lets a dump stand as a gate.

mod args;
mod collect;
mod frames;
mod gate;
mod pack;
mod probe;
mod render;
mod report;
mod scenario;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

pub use args::DumpArgs;
pub use args::parse as parse_args;

use collect::{Context, dump_document};
use report::DumpReport;
use scenario::Scenario;

/// The extensions of a skin document.
const DOCUMENT_EXTENSIONS: [&str; 3] = [rbms_skin::loader::LUA_SKIN_EXTENSION, "json", "json5"];

fn is_document(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|extension| DOCUMENT_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known)))
}

/// The folder skins may read from and the documents to dump: a pack folder's documents, or one file
/// and the folder it is in.
fn documents(target: &Path) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let target = std::fs::canonicalize(target).map_err(|e| format!("{} not found: {e}", target.display()))?;
    if target.is_file() {
        let root = target.parent().map(Path::to_path_buf).ok_or_else(|| format!("{} has no folder", target.display()))?;
        return Ok((root, vec![target]));
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(&target)
        .map_err(|e| format!("{} not listed: {e}", target.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_document(path))
        .collect();
    found.sort();
    if found.is_empty() {
        return Err(format!("no .luaskin, .json or .json5 document in {}", target.display()));
    }
    Ok((target, found))
}

/// How many temporary overlays this process has named, so that no two dumps share one.
static TEMPORARY_OVERLAYS: AtomicU64 = AtomicU64::new(0);

/// A folder of this dump's own for the skins' writes. A dump empties its temporary overlay before
/// it starts and removes it when it ends, so the name carries a serial as well as the process id:
/// two dumps running in one process would otherwise delete each other's files.
fn temporary_overlay() -> PathBuf {
    let serial = TEMPORARY_OVERLAYS.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("rbms-skin-dump-{}-{serial}", std::process::id()))
}

/// Dumps every document the arguments name.
pub fn dump(args: &DumpArgs) -> Result<DumpReport, String> {
    let (root, entries) = documents(&args.target)?;
    let scenario = args.scenario.as_deref().map(Scenario::from_file).transpose()?;
    let overlay = args.overlay.clone().unwrap_or_else(temporary_overlay);
    let own_overlay = args.overlay.is_none();
    if own_overlay {
        let _ = std::fs::remove_dir_all(&overlay);
    }

    let before = pack::list(&root);
    let context =
        Context { root: &root, overlay: &overlay, seed: args.seed, frames: args.frames, scenario: scenario.as_ref(), probe_budget: args.probe_budget };
    let documents = entries.iter().map(|entry| dump_document(&context, entry)).collect();
    let pack = pack::compare(&root, &before);

    if own_overlay {
        let _ = std::fs::remove_dir_all(&overlay);
    }
    Ok(DumpReport { target: args.target.display().to_string(), seed: args.seed, frame_micros: frames::FRAME_MICROS, documents, pack })
}

/// Runs `skin-dump` and prints the report. Fails when the target cannot be read, when a skin
/// changed its own folder, or when a strict dump found a document that did not come out clean.
pub fn run(args: &DumpArgs) -> ExitCode {
    let report = match dump(args) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if args.json {
        match serde_json::to_string_pretty(&report) {
            Ok(text) => println!("{text}"),
            Err(e) => {
                eprintln!("report not serialised: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!("{}", render::text(&report));
    }
    let findings = if args.strict { gate::findings(&report, &args.allowed_failures) } else { Vec::new() };
    for finding in &findings {
        eprintln!("strict: {finding}");
    }
    if report.pack.unchanged() && findings.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
