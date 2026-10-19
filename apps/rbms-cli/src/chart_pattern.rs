//! `chart-pattern`: writes the note layout of the charts of a library folder, one JSON file per
//! chart named by its md5 (`<md5>.json`), for the analysis the website does of a chart.
//!
//! Every chart under the folder is read once, through the parser and model the player uses, and a
//! chart of the mode asked for (BEAT_7K unless `--mode` says another) is written. Nothing but the
//! chart files is opened: no keysound and no picture. Two files with one md5 are one chart, and the
//! first of them in path order is the one read. A file that cannot be read is listed and left out;
//! the rest are written all the same.
//!
//! Running it again over the same library writes nothing that is already there: a file whose
//! content would not change is not touched. Files of earlier runs that no chart stands behind any
//! more are left in place. With `--table <data.json>` the md5s of a difficulty table are matched
//! against what was read, and the match is printed and kept as `_coverage.json` next to the
//! patterns.
//!
//! The patterns are the charts' note layouts, which is a work of their authors: they go to a folder
//! the caller chooses and are not kept in this repository.

mod args;
mod coverage;
mod document;
mod render;
mod walk;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use rbms_model::Mode;

pub use args::PatternArgs;
pub use args::parse as parse_args;

use coverage::{Coverage, coverage_of, read_table};
use document::{Extracted, extract};

/// Charts read at once. A batch is read by a pool of workers and then stored in path order, so the
/// documents held at any moment are at most this many.
const BATCH_SIZE: usize = 64;

/// Ceiling on the workers that read charts. Beyond this the read is bound by the disk rather than
/// by the CPU.
const MAX_WORKERS: usize = 8;

/// Charts between two progress lines.
const PROGRESS_INTERVAL: usize = 500;

/// The extension of a pattern file.
const PATTERN_EXTENSION: &str = "json";

/// The name of the coverage summary. The underscore keeps it from reading as a chart's md5.
const COVERAGE_FILE_NAME: &str = "_coverage.json";

/// The extension a file has while it is being written, before it takes its name.
const TEMPORARY_EXTENSION: &str = "json.tmp";

/// A file or folder that could not be read, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub path: PathBuf,
    pub reason: String,
}

/// What a run did.
#[derive(Debug)]
pub struct PatternReport {
    pub mode: Mode,
    /// Chart files found under the library folder.
    pub found: usize,
    /// Patterns written, new or changed.
    pub written: usize,
    /// Patterns that were there already with the same content.
    pub unchanged: usize,
    /// Charts of the mode whose md5 an earlier file had.
    pub duplicates: usize,
    /// Charts of another mode.
    pub other_mode: usize,
    pub failures: Vec<Failure>,
    /// Present when a table was given.
    pub coverage: Option<Coverage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stored {
    Written,
    Unchanged,
}

/// Writes `bytes` to `path` unless the file is there with these very bytes. The bytes go to a
/// temporary file that is then renamed, so a run that is stopped halfway leaves no half-written
/// pattern under a chart's name.
fn store(path: &Path, bytes: &[u8]) -> std::io::Result<Stored> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(Stored::Unchanged);
    }
    let temporary = path.with_extension(TEMPORARY_EXTENSION);
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)?;
    Ok(Stored::Written)
}

fn read_chart(path: &Path, mode: Mode) -> Result<Extracted, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read error: {e}"))?;
    extract(&bytes, path, mode)
}

/// Workers to read with: one per core, capped, and never more than there is work for.
fn worker_count(jobs: usize) -> usize {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    cores.clamp(1, MAX_WORKERS).min(jobs.max(1))
}

/// Reads `paths` across a pool of workers and returns what came of each, in the order of `paths`.
fn read_batch(paths: &[PathBuf], mode: Mode) -> Vec<Result<Extracted, String>> {
    let cursor = AtomicUsize::new(0);
    let mut read: Vec<(usize, Result<Extracted, String>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..worker_count(paths.len()))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let index = cursor.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(index) else {
                            break done;
                        };
                        done.push((index, read_chart(path, mode)));
                    }
                })
            })
            .collect();
        workers.into_iter().flat_map(|worker| worker.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))).collect()
    });
    read.sort_by_key(|(index, _)| *index);
    read.into_iter().map(|(_, outcome)| outcome).collect()
}

/// Reads the library and writes the patterns, returning what was done. Fails when the library is
/// not a folder, the table cannot be read, or a file cannot be written; a chart that cannot be read
/// is a [`Failure`] in the report and not an error.
pub fn export(args: &PatternArgs) -> Result<PatternReport, String> {
    if !args.library.is_dir() {
        return Err(format!("not a folder: {}", args.library.display()));
    }
    let table = args.table.as_deref().map(read_table).transpose()?;
    std::fs::create_dir_all(&args.out).map_err(|e| format!("{} not created: {e}", args.out.display()))?;

    let walked = walk::charts_under(&args.library);
    let total = walked.charts.len();
    let mut failures = walked.failures;
    let mut patterned: HashSet<String> = HashSet::new();
    let mut elsewhere: HashSet<String> = HashSet::new();
    let (mut written, mut unchanged, mut duplicates, mut other_mode, mut done) = (0, 0, 0, 0, 0);

    for batch in walked.charts.chunks(BATCH_SIZE) {
        for (path, read) in batch.iter().zip(read_batch(batch, args.mode)) {
            match read {
                Err(reason) => failures.push(Failure { path: path.clone(), reason }),
                Ok(Extracted::OtherMode { md5 }) => {
                    elsewhere.insert(md5);
                    other_mode += 1;
                }
                Ok(Extracted::Pattern { md5, document }) => {
                    if !patterned.insert(md5.clone()) {
                        duplicates += 1;
                        continue;
                    }
                    let target = args.out.join(format!("{md5}.{PATTERN_EXTENSION}"));
                    match store(&target, &document).map_err(|e| format!("{} not written: {e}", target.display()))? {
                        Stored::Written => written += 1,
                        Stored::Unchanged => unchanged += 1,
                    }
                }
            }
        }
        let before = done;
        done += batch.len();
        if done / PROGRESS_INTERVAL > before / PROGRESS_INTERVAL || done == total {
            eprintln!("read {done}/{total} charts");
        }
    }
    failures.sort_by(|a, b| a.path.cmp(&b.path));

    let coverage = match &table {
        Some(table) => {
            let coverage = coverage_of(table, args.mode.name, &patterned, &elsewhere, failures.len());
            let summary = serde_json::to_vec_pretty(&coverage).map_err(|e| format!("coverage not serialised: {e}"))?;
            let target = args.out.join(COVERAGE_FILE_NAME);
            store(&target, &summary).map_err(|e| format!("{} not written: {e}", target.display()))?;
            Some(coverage)
        }
        None => None,
    };
    Ok(PatternReport { mode: args.mode, found: total, written, unchanged, duplicates, other_mode, failures, coverage })
}

/// Runs `chart-pattern` and prints the report. Fails when the run could not be done; a chart that
/// could not be read is in the printed list and does not fail it.
pub fn run(args: &PatternArgs) -> ExitCode {
    match export(args) {
        Ok(report) => {
            println!("{}", render::text(args, &report));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
