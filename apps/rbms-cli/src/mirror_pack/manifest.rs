//! `manifest.json`: what a run packed, for the uploader to read, and for the next run to find what it need not pack again.
//!
//! The file holds no absolute path and no time, so it is the same bytes whenever the packs are the same.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::folders::Refusal;
use super::notice::Notices;
use super::select::{Duplicate, TableCoverage};

/// The `format` tag of the manifest.
pub const MANIFEST_FORMAT: &str = "rbms-song-mirror-manifest";

/// The generation of the manifest's shape.
pub const MANIFEST_VERSION: u32 = 1;

/// The name of the manifest in the output folder.
pub const MANIFEST_FILE_NAME: &str = "manifest.json";

/// The extension a file has while it is being written, before it takes its name.
const TEMPORARY_EXTENSION: &str = "json.tmp";

/// One chart of a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestChart {
    pub md5: String,
    pub sha256: String,
    /// The chart's name in the archive.
    pub path: String,
}

/// One pack: an object to upload and the charts it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestPack {
    /// The object's key: `packs/<sha256>.zip`.
    pub key: String,
    pub sha256: String,
    /// The archive's size.
    pub bytes: u64,
    /// The size of the files in it.
    pub uncompressed_bytes: u64,
    pub entries: usize,
    /// Files in the archive that the player's unpacker leaves out because of their kind.
    pub skipped_by_client: usize,
    /// Links in the folder that were left out of the archive.
    pub symlinks: usize,
    /// The song folder, from the library folder.
    pub folder: String,
    /// The song folder's own name, under which every entry is.
    pub top: String,
    /// What the folder's files were when they were packed; see [`super::pack::fingerprint`].
    pub fingerprint: String,
    /// Every chart of the folder, not only the ones a table lists.
    pub charts: Vec<ManifestChart>,
    pub notices: Notices,
}

/// A song folder that was chosen and not packed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRefusal {
    pub folder: String,
    pub reason: String,
    pub detail: String,
}

impl ManifestRefusal {
    pub fn of(folder: &str, refusal: &Refusal) -> ManifestRefusal {
        ManifestRefusal { folder: folder.to_string(), reason: refusal.code().to_string(), detail: refusal.detail() }
    }
}

/// The whole manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorManifest {
    pub format: String,
    pub version: u32,
    pub generator: String,
    pub tables: Vec<TableCoverage>,
    pub packs: Vec<ManifestPack>,
    pub refused: Vec<ManifestRefusal>,
    pub duplicates: Vec<Duplicate>,
}

/// Reads the manifest at `path`; `None` when there is none. A file that is not a manifest of this
/// generation is an error, so that a run does not take it for one.
pub fn read_manifest(path: &Path) -> Result<Option<MirrorManifest>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{} not read: {e}", path.display())),
    };
    let manifest: MirrorManifest = serde_json::from_slice(&bytes).map_err(|e| format!("{} is not a mirror manifest: {e}", path.display()))?;
    if manifest.format != MANIFEST_FORMAT || manifest.version != MANIFEST_VERSION {
        return Err(format!("{} is not a version {MANIFEST_VERSION} {MANIFEST_FORMAT}", path.display()));
    }
    Ok(Some(manifest))
}

/// Writes the manifest to `path` unless the file is there with these very bytes. The bytes go to a
/// temporary file that is then renamed, so a run that is stopped halfway leaves no half-written manifest.
pub fn write_manifest(path: &Path, manifest: &MirrorManifest) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(manifest).map_err(|e| format!("manifest not serialised: {e}"))?;
    bytes.push(b'\n');
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    let temporary = path.with_extension(TEMPORARY_EXTENSION);
    fs::write(&temporary, &bytes).map_err(|e| format!("{} not written: {e}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|e| format!("{} not written: {e}", path.display()))
}
