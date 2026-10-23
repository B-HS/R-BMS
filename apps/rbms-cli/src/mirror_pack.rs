//! `mirror-pack`: packs the song folders of a library as zip archives, one per folder, and writes
//! the manifest that lists them, ready for a song mirror to take.
//!
//! The song folders to pack are those that hold a chart which a difficulty table lists by md5
//! (`--table`, as many as are given), or every one (`--all`). A folder is packed whole: its other
//! charts, its sounds and pictures and what lies in the folders below it, except for the folders
//! below it that are song folders of their own. Each archive is named by the SHA-256 of its bytes,
//! and is the same bytes whenever the folder is the same, so a run that finds an archive already
//! there leaves it alone, and a folder that has not changed since the last run is not read again.
//!
//! The packs are held to what the player's unpacker allows (`rbms_download`), so that a pack made
//! here is one the player can unpack. A folder over those limits is refused and listed; so is one
//! that cannot be read, and the run goes on with the rest. Nothing is fetched or sent: the packs go
//! to a folder the caller chooses, which is not in the library and is not kept in this repository.
//! `--dry-run` reports what would be packed and writes nothing.
//!
//! Files that only a file system makes (`.DS_Store` and the like) and links are left out. The
//! manifest also says, for each pack, which text files speak of redistribution, how many names are
//! written in a form another platform may not read the same, and how many files the charts name
//! from outside the folder, so that they can be looked at before the pack goes anywhere.

mod args;
mod folders;
mod manifest;
mod notice;
mod pack;
mod render;
mod select;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub use args::MirrorPackArgs;
pub use args::parse as parse_args;

use folders::{Failure, PackLimits, Refusal, SongFolder, pack_entries, skipped_by_client, song_folders_under};
use manifest::{
    MANIFEST_FILE_NAME, MANIFEST_FORMAT, MANIFEST_VERSION, ManifestChart, ManifestPack, ManifestRefusal, MirrorManifest, read_manifest, write_manifest,
};
use notice::scan_notices;
use pack::{PACK_EXTENSION, WriteError, WrittenPack, fingerprint, pack_path, write_pack};
use select::{TableCoverage, read_table, select};

/// The folder of the output folder that holds the packs.
const PACKS_FOLDER_NAME: &str = "packs";

/// Song folders between two progress lines.
const PROGRESS_INTERVAL: usize = 50;

/// What a run did.
#[derive(Debug)]
pub struct MirrorPackReport {
    /// Song folders found under the library folder.
    pub folders_found: usize,
    /// Song folders chosen by the tables or by `--all`.
    pub folders_selected: usize,
    /// Packs written, new or changed. Always 0 for a dry run.
    pub written: usize,
    /// Packs that were there already with the same bytes. Always 0 for a dry run.
    pub reused: usize,
    /// The chosen song folders that were not packed, by their folder, and why.
    pub refused: Vec<(String, Refusal)>,
    /// Folders and charts that could not be read while looking for song folders.
    pub failures: Vec<Failure>,
    /// Table charts that more than one song folder holds.
    pub duplicates: usize,
    pub tables: Vec<TableCoverage>,
    /// Files in the packs, or for a dry run in the folders that could be packed.
    pub total_entries: usize,
    /// Of those files, how many the player's unpacker leaves out because of their kind.
    pub skipped_by_client: usize,
    /// The size of the packs written or reused, each counted once. Always 0 for a dry run.
    pub total_bytes: u64,
    /// The size of the files in them.
    pub total_uncompressed_bytes: u64,
}

/// A song folder that was packed.
struct Packed {
    entries: usize,
    uncompressed_bytes: u64,
    skipped_by_client: usize,
    /// What was written and what the manifest says of it; `None` for a dry run.
    stored: Option<(WrittenPack, ManifestPack)>,
}

enum PackFailure {
    /// The folder is not packed; the run goes on.
    Refused(Refusal),
    /// The output cannot be written; the run stops.
    Abort(String),
}

impl From<Refusal> for PackFailure {
    fn from(refusal: Refusal) -> PackFailure {
        PackFailure::Refused(refusal)
    }
}

/// `path` made absolute, with as much of it as exists resolved of links, so that two paths to one
/// place compare as equal even when the place is not made yet.
fn resolved(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut missing = Vec::new();
    let mut base = absolute.clone();
    loop {
        if let Ok(found) = fs::canonicalize(&base) {
            return missing.iter().rev().fold(found, |resolved, part| resolved.join(part));
        }
        match (base.file_name().map(ToOwned::to_owned), base.parent().map(Path::to_path_buf)) {
            (Some(name), Some(parent)) => {
                missing.push(name);
                base = parent;
            }
            _ => return absolute,
        }
    }
}

/// The pack an earlier run made of this very folder, if the manifest says so and the pack is there.
fn reusable(previous: Option<&MirrorManifest>, folder: &SongFolder, fingerprint: &str, packs_dir: &Path) -> Option<WrittenPack> {
    let earlier = previous?.packs.iter().find(|pack| pack.folder == folder.relative && pack.fingerprint == fingerprint)?;
    let path = pack_path(packs_dir, &earlier.sha256);
    fs::metadata(&path).is_ok_and(|kept| kept.len() == earlier.bytes).then(|| WrittenPack {
        sha256: earlier.sha256.clone(),
        bytes: earlier.bytes,
        path,
        reused: true,
    })
}

fn manifest_charts(folder: &SongFolder) -> Vec<ManifestChart> {
    folder
        .charts
        .iter()
        .map(|chart| ManifestChart {
            md5: chart.md5.clone(),
            sha256: chart.sha256.clone(),
            path: format!("{}/{}", folder.top, chart.path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned())),
        })
        .collect()
}

/// Packs one song folder into `packs_dir`, or with no `packs_dir` only checks that it could be.
fn pack_folder(folder: &SongFolder, limits: &PackLimits, previous: Option<&MirrorManifest>, packs_dir: Option<&Path>) -> Result<Packed, PackFailure> {
    let gathered = pack_entries(folder, limits)?;
    let entries = gathered.entries.len();
    let uncompressed_bytes = gathered.entries.iter().map(|entry| entry.bytes).sum::<u64>();
    let skipped_by_client = skipped_by_client(&gathered.entries);
    let Some(packs_dir) = packs_dir else {
        return Ok(Packed { entries, uncompressed_bytes, skipped_by_client, stored: None });
    };
    let fingerprint = fingerprint(&gathered.entries);
    let written = match reusable(previous, folder, &fingerprint, packs_dir) {
        Some(written) => written,
        None => write_pack(&gathered.entries, packs_dir).map_err(|e| match e {
            WriteError::Source { name, reason } => PackFailure::Refused(Refusal::Unreadable { path: PathBuf::from(name), reason }),
            WriteError::Output(reason) => PackFailure::Abort(reason),
        })?,
    };
    let manifest_pack = ManifestPack {
        key: format!("{PACKS_FOLDER_NAME}/{}.{PACK_EXTENSION}", written.sha256),
        sha256: written.sha256.clone(),
        bytes: written.bytes,
        uncompressed_bytes,
        entries,
        skipped_by_client,
        symlinks: gathered.symlinks,
        folder: folder.relative.clone(),
        top: folder.top.clone(),
        fingerprint,
        charts: manifest_charts(folder),
        notices: scan_notices(folder, &gathered.entries),
    };
    Ok(Packed { entries, uncompressed_bytes, skipped_by_client, stored: Some((written, manifest_pack)) })
}

/// Reads the library and writes the packs and the manifest, returning what was done. Fails when the
/// library is not a folder, the output folder is inside it, a table cannot be read, or the output
/// cannot be written; a song folder that cannot be packed is in the report and does not fail it.
pub fn export(args: &MirrorPackArgs) -> Result<MirrorPackReport, String> {
    export_within(args, &PackLimits::default())
}

/// [`export`] with the limits a song folder is held to given, which a test lowers to refuse a small folder.
fn export_within(args: &MirrorPackArgs, limits: &PackLimits) -> Result<MirrorPackReport, String> {
    if !args.library.is_dir() {
        return Err(format!("not a folder: {}", args.library.display()));
    }
    let library = fs::canonicalize(&args.library).map_err(|e| format!("{} not resolved: {e}", args.library.display()))?;
    if resolved(&args.out).starts_with(&library) {
        return Err(format!("{} is inside the library folder, where a later run would pack its own output", args.out.display()));
    }
    let tables = args.tables.iter().map(|path| read_table(path)).collect::<Result<Vec<_>, _>>()?;

    let walked = song_folders_under(&library);
    let selection = select(&walked.folders, &tables, args.all);
    let packs_dir = args.out.join(PACKS_FOLDER_NAME);
    let manifest_path = args.out.join(MANIFEST_FILE_NAME);
    let previous = if args.dry_run {
        None
    } else {
        fs::create_dir_all(&args.out).map_err(|e| format!("{} not created: {e}", args.out.display()))?;
        read_manifest(&manifest_path).unwrap_or_else(|reason| {
            eprintln!("{reason}; every pack is made again");
            None
        })
    };

    let total = selection.indexes.len();
    let mut packs = Vec::new();
    let mut refused = Vec::new();
    let mut stored: BTreeMap<String, u64> = BTreeMap::new();
    let (mut written, mut reused, mut total_entries, mut skipped, mut total_uncompressed_bytes) = (0, 0, 0, 0, 0);
    for (position, &index) in selection.indexes.iter().enumerate() {
        let folder = &walked.folders[index];
        match pack_folder(folder, limits, previous.as_ref(), (!args.dry_run).then_some(packs_dir.as_path())) {
            Ok(packed) => {
                total_entries += packed.entries;
                skipped += packed.skipped_by_client;
                total_uncompressed_bytes += packed.uncompressed_bytes;
                if let Some((pack, manifest_pack)) = packed.stored {
                    if pack.reused {
                        reused += 1;
                    } else {
                        written += 1;
                    }
                    stored.insert(pack.sha256, pack.bytes);
                    packs.push(manifest_pack);
                }
            }
            Err(PackFailure::Refused(refusal)) => refused.push((folder.relative.clone(), refusal)),
            Err(PackFailure::Abort(reason)) => return Err(reason),
        }
        let done = position + 1;
        if done.is_multiple_of(PROGRESS_INTERVAL) || done == total {
            eprintln!("song folders {done}/{total}");
        }
    }

    if !args.dry_run {
        let manifest = MirrorManifest {
            format: MANIFEST_FORMAT.to_string(),
            version: MANIFEST_VERSION,
            generator: format!("rbms-cli {}", env!("CARGO_PKG_VERSION")),
            tables: selection.coverage.clone(),
            packs,
            refused: refused.iter().map(|(folder, refusal)| ManifestRefusal::of(folder, refusal)).collect(),
            duplicates: selection.duplicates.clone(),
        };
        write_manifest(&manifest_path, &manifest)?;
    }
    Ok(MirrorPackReport {
        folders_found: walked.folders.len(),
        folders_selected: total,
        written,
        reused,
        refused,
        failures: walked.failures,
        duplicates: selection.duplicates.len(),
        tables: selection.coverage,
        total_entries,
        skipped_by_client: skipped,
        total_bytes: stored.values().sum(),
        total_uncompressed_bytes,
    })
}

/// Runs `mirror-pack` and prints the report. Fails when the run could not be done; a song folder
/// that was refused or could not be read is in the printed list and does not fail it.
pub fn run(args: &MirrorPackArgs) -> ExitCode {
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
