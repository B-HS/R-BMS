//! The default skin pack: carried inside the binary, and written out beside the settings file so
//! it is read like any other pack.
//!
//! The table of what is carried is built by the package's build script from
//! `assets/skins/rbms-default` and is never written by hand. Every run settles the installed folder
//! against it ([`install_default_pack`]):
//!
//! - a file that is not there is written;
//! - a file that is still the one an earlier build wrote is replaced when this build ships a
//!   different one, and removed when this build no longer ships it;
//! - a file the player changed, or put there themselves, is left exactly as it is.
//!
//! What tells the three apart is a record of what was installed ([`MANIFEST_FILE`]): the digest of
//! every file as it was written. An installed file is read, to be hashed, only when the record does
//! not say it is this build's: on the run after an update, on the run after the record was lost,
//! and on every run for a file the player changed, whose record is kept at what was last installed
//! so that putting the file back makes it the installer's again. A run that finds the record naming
//! this build's digest for every file reads the record, looks each file up, and reads and lists
//! nothing else. The digests of the shipped files are worked out when the binary is built, so
//! nothing carried in the binary is hashed while the player waits for a window.
//!
//! That is the cost of settling the folder, not of drawing with the pack: the skin library reads
//! the header of each of the pack's documents when it is made, as it does for any pack.
//!
//! The pack's folder may itself be a link -- a player who keeps it on another disk -- and is
//! followed. Nothing below it is: a folder inside the pack that is a link, or is not a folder, is
//! the player's arrangement, and no file is written or removed through it.
//!
//! Nothing here fails the run. A folder that cannot be written leaves the pack missing or partly
//! there, the count of what could not be done is handed back for the caller to report, and the
//! screens whose documents are not there are drawn by their built-in layouts.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rbms_config::DEFAULT_SKIN_FOLDER;
use sha2::{Digest, Sha256};

use super::hex;

/// The folder the default pack is installed into, below the skin folder beside the settings file.
pub(crate) const DEFAULT_SKIN_DIRECTORY: &str = "rbms-default";

/// The folder inside the default pack that holds the system sound set, read when the player
/// configured no sound folder of their own.
pub(crate) const DEFAULT_SOUND_DIRECTORY: &str = "sound";

/// The record of what was installed, kept in the pack's folder. Not a name a skin document or one
/// of its files is found under.
pub(crate) const MANIFEST_FILE: &str = "install-manifest.txt";

/// The first line of the record. A record that begins with anything else is one this build cannot
/// read and is taken for no record at all.
const MANIFEST_HEADER: &str = "rbms-installed-files 1";

/// What separates a file's digest from its name on one line of the record.
const MANIFEST_SEPARATOR: char = ' ';

/// The longest record that is read. A pack of a few hundred files is some tens of kilobytes; a
/// file larger than this is not a record this build wrote.
const MANIFEST_MAX_BYTES: u64 = 1024 * 1024;

/// How many hexadecimal digits a digest is written with.
const DIGEST_HEX_CHARS: usize = 64;

/// The largest installed file that is read to find out whether it is still the one that was
/// written. The whole pack is held to a fraction of this, so a larger file is the player's own.
const INSTALLED_FILE_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// What a file is written under until it is whole, beside the name it is then given: the name, the
/// number of the process writing it, and this.
const PARTIAL_SUFFIX: &str = "rbms-partial";

/// What separates the parts of the name a file is written under until it is whole.
const PARTIAL_SEPARATOR: char = '.';

/// What separates the parts of a file's path inside the pack.
const NAME_SEPARATOR: char = '/';

/// The characters no part of an installed file's path may hold: a separator or a drive of the
/// platform's own, either of which would take the path somewhere the name does not say.
const NAME_FORBIDDEN: [char; 2] = ['\\', ':'];

/// One file shipped inside the binary.
pub(crate) struct EmbeddedFile {
    /// The file's path inside the pack, with `/` between its parts.
    pub(crate) name: &'static str,
    /// The SHA-256 of `bytes`, in lowercase hexadecimal.
    pub(crate) digest: &'static str,
    pub(crate) bytes: &'static [u8],
}

/// Every file of the default pack, in the byte order of their names.
pub(crate) static DEFAULT_PACK_FILES: &[EmbeddedFile] = include!(concat!(env!("OUT_DIR"), "/default_pack.rs"));

/// What one run's settling of an installed pack came to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PackInstall {
    /// Files written: ones that were not there, and ones an earlier build wrote that this build
    /// ships differently.
    pub(crate) written: usize,
    /// Files an earlier build wrote that this build no longer ships, taken away.
    pub(crate) removed: usize,
    /// Files left as the player has them, where this build would have written something else.
    pub(crate) kept: usize,
    /// Files, and the record itself, that could not be written.
    pub(crate) failed: usize,
}

impl PackInstall {
    /// Whether everything this build ships is on disk, or the player's own file is in its place.
    pub(crate) fn is_complete(&self) -> bool {
        self.failed == 0
    }
}

/// Where the default pack is installed: always below the default skin folder beside the settings
/// file, whichever folder the SKIN tab is looking for documents in.
pub(crate) fn default_pack_directory(settings_path: &Path) -> PathBuf {
    settings_path.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER).join(DEFAULT_SKIN_DIRECTORY)
}

/// The installed default pack, which is the pack a run draws when the player named none. `None`
/// while the folder is not there, which leaves every screen to its built-in layout.
pub(crate) fn installed_default_pack(settings_path: &Path) -> Option<PathBuf> {
    let directory = default_pack_directory(settings_path);
    directory.is_dir().then_some(directory)
}

/// The installed system sound set, for a player who configured no folder of their own. `None` while
/// the folder is not there, which leaves the set silent rather than naming a directory nothing can
/// be read from.
pub(crate) fn default_sound_folder(settings_path: &Path) -> Option<PathBuf> {
    let directory = default_pack_directory(settings_path).join(DEFAULT_SOUND_DIRECTORY);
    directory.is_dir().then_some(directory)
}

/// Settle the installed default pack against the one this build ships.
///
/// Every file is tried even after one fails, so a single unwritable file costs that one file rather
/// than the rest of the pack. Nothing else under the skin folder is read, moved or removed.
pub(crate) fn install_default_pack(settings_path: &Path) -> PackInstall {
    install_pack(&default_pack_directory(settings_path), DEFAULT_PACK_FILES)
}

/// Settle the pack installed in `directory` against `files`, and bring its record up to date.
///
/// What an earlier build installed and this one does not ship is taken away before anything is
/// settled. A build that only changes how a name is cased ships a new name for what a file system
/// that ignores case holds as the same file, and taking the old name away afterwards would take
/// this build's file with it.
///
/// A run that was cut short leaves a file under the name it was being written with, and leaves the
/// folder unsettled, so the next run has something to write. A run that wrote, removed or failed at
/// anything therefore ends by taking such files away ([`sweep_partials`]); one that found
/// everything in place lists no folder.
pub(crate) fn install_pack(directory: &Path, files: &[EmbeddedFile]) -> PackInstall {
    let manifest = directory.join(MANIFEST_FILE);
    let before = read_manifest(&manifest);
    let recorded = parse_manifest(before.as_deref().unwrap_or_default());
    let mut outcome = PackInstall::default();
    let mut record = BTreeMap::new();

    for (name, digest) in &recorded {
        if files.iter().any(|file| file.name == name) {
            continue;
        }
        match retire_file(directory, name, digest) {
            Retired::Removed => outcome.removed += 1,
            Retired::NotOurs => {}
            Retired::Failed => {
                outcome.failed += 1;
                record.insert(name.clone(), digest.clone());
            }
        }
    }

    for file in files {
        let known = recorded.get(file.name).map(String::as_str);
        let settled = settle_file(directory, file, known);
        match settled {
            Settled::Current => {}
            Settled::Written => outcome.written += 1,
            Settled::Kept => outcome.kept += 1,
            Settled::Failed => outcome.failed += 1,
        }
        let digest = if matches!(settled, Settled::Current | Settled::Written) { Some(file.digest) } else { known };
        if let Some(digest) = digest {
            record.insert(file.name.to_owned(), digest.to_owned());
        }
    }

    let after = manifest_text(&record);
    let record_moved = before.as_deref() != Some(after.as_str()) && !(record.is_empty() && before.is_none());
    if record_moved && !replace_file(&manifest, after.as_bytes()) {
        outcome.failed += 1;
    }
    if record_moved || outcome.written + outcome.removed + outcome.failed > 0 {
        let names = files.iter().map(|file| file.name).chain(recorded.keys().map(String::as_str)).chain([MANIFEST_FILE]);
        sweep_partials(directory, &names.collect());
    }
    outcome
}

/// What became of one shipped file.
enum Settled {
    /// The file on disk is the one this build ships, and nothing was written.
    Current,
    /// The file was written.
    Written,
    /// Something of the player's is at the path, and it was left alone.
    Kept,
    /// The file is not there and could not be written.
    Failed,
}

/// Bring one shipped file to its place in the pack installed in `directory`. `recorded` is the
/// digest of what an earlier run wrote there, when one did.
///
/// Whatever is at the path that is not a regular file -- a link, a folder -- is the player's and is
/// never written through, and neither is a folder on the way to it that is not a folder of the
/// pack's own ([`is_inside_own_folders`]). A regular file is read only when the record does not
/// already say it is this build's.
fn settle_file(directory: &Path, file: &EmbeddedFile, recorded: Option<&str>) -> Settled {
    let path = directory.join(file.name);
    let theirs = || if path.is_file() { Settled::Kept } else { Settled::Failed };
    if !is_inside_own_folders(directory, file.name) {
        return theirs();
    }
    let found = match std::fs::symlink_metadata(&path) {
        Ok(found) => found,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return written(replace_file(&path, file.bytes)),
        Err(_) => return Settled::Failed,
    };
    if !found.file_type().is_file() {
        return theirs();
    }
    if recorded == Some(file.digest) {
        return Settled::Current;
    }
    let on_disk = installed_digest(&path, found.len());
    if on_disk.as_deref() == Some(file.digest) {
        return Settled::Current;
    }
    if on_disk.is_some() && on_disk.as_deref() == recorded {
        return written(replace_file(&path, file.bytes));
    }
    Settled::Kept
}

/// What a write came to.
fn written(done: bool) -> Settled {
    if done { Settled::Written } else { Settled::Failed }
}

/// What became of a file an earlier build installed and this one does not ship.
enum Retired {
    Removed,
    /// The file is gone, or is no longer the one that was installed.
    NotOurs,
    Failed,
}

/// Take away the file `name` of the pack installed in `directory` when it is still the one an
/// earlier run wrote, whose digest was `recorded`, and is reached through the pack's own folders.
fn retire_file(directory: &Path, name: &str, recorded: &str) -> Retired {
    if !is_inside_own_folders(directory, name) {
        return Retired::NotOurs;
    }
    let path = directory.join(name);
    let Ok(found) = std::fs::symlink_metadata(&path) else {
        return Retired::NotOurs;
    };
    if !found.file_type().is_file() || installed_digest(&path, found.len()).as_deref() != Some(recorded) {
        return Retired::NotOurs;
    }
    if std::fs::remove_file(&path).is_ok() { Retired::Removed } else { Retired::Failed }
}

/// Whether every folder between `directory` and the file `name` inside it is a folder of the
/// pack's own, or is not there yet and will be made as one.
///
/// A folder that is a link leads wherever the player pointed it, which may be outside the pack, and
/// the record -- a file the player can edit -- would then name files to remove there. `directory`
/// itself is not looked at: the pack may be kept somewhere else behind a link.
fn is_inside_own_folders(directory: &Path, name: &str) -> bool {
    let mut folder = directory.to_path_buf();
    let mut parts = name.split(NAME_SEPARATOR).peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            break;
        }
        folder.push(part);
        match std::fs::symlink_metadata(&folder) {
            Ok(found) if found.file_type().is_dir() => {}
            Ok(_) => return false,
            Err(error) => return error.kind() == std::io::ErrorKind::NotFound,
        }
    }
    true
}

/// Take away what a run that was cut short left under the name a file is written with
/// ([`replace_file`]), for every file in `names` and whichever process was writing it.
///
/// Only the folders the named files are in are listed, only when they are the pack's own, and only
/// a regular file whose name is exactly one this installer would have written with is removed: a
/// file the player keeps under a name that merely ends the same way is theirs. This is called once
/// a run's own writes are done, so nothing it takes away is this run's.
fn sweep_partials(directory: &Path, names: &BTreeSet<&str>) {
    let folders: BTreeSet<&str> = names.iter().map(|name| name.rsplit_once(NAME_SEPARATOR).map_or("", |(folder, _)| folder)).collect();
    for folder in folders {
        let inside = |file: &str| if folder.is_empty() { file.to_owned() } else { format!("{folder}{NAME_SEPARATOR}{file}") };
        if !is_inside_own_folders(directory, &inside(MANIFEST_FILE)) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(directory.join(folder)) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let written_for = file_name.to_str().and_then(partial_target);
            let is_ours = written_for.is_some_and(|target| names.contains(inside(target).as_str()));
            if is_ours && entry.file_type().is_ok_and(|kind| kind.is_file()) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// The name of the file that `file_name` is the half-written form of, or `None` when it is not a
/// name a file is written under: the target's name, a process number, and [`PARTIAL_SUFFIX`].
fn partial_target(file_name: &str) -> Option<&str> {
    let (rest, suffix) = file_name.rsplit_once(PARTIAL_SEPARATOR)?;
    let (target, process) = rest.rsplit_once(PARTIAL_SEPARATOR)?;
    let is_process = !process.is_empty() && process.bytes().all(|byte| byte.is_ascii_digit());
    (suffix == PARTIAL_SUFFIX && is_process && !target.is_empty()).then_some(target)
}

/// The digest of the regular file at `path`, which is `length` bytes long. `None` for a file that
/// cannot be read, and for one too large to be anything this player installed.
fn installed_digest(path: &Path, length: u64) -> Option<String> {
    if length > INSTALLED_FILE_MAX_BYTES {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(hex(&hasher.finalize()))
}

/// Put `bytes` at `path`, in place of whatever regular file is there, and report whether they are
/// there afterwards.
///
/// The bytes are written under another name beside the path and moved onto it once they are whole,
/// so a run that is cut short never leaves half a file under a name that is read.
fn replace_file(path: &Path, bytes: &[u8]) -> bool {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return false;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let mut partial_name = name.to_os_string();
    partial_name.push(format!("{PARTIAL_SEPARATOR}{}{PARTIAL_SEPARATOR}{PARTIAL_SUFFIX}", std::process::id()));
    let partial = parent.join(partial_name);
    let whole = std::fs::File::create(&partial).and_then(|mut file| file.write_all(bytes)).is_ok();
    if whole && std::fs::rename(&partial, path).is_ok() {
        return true;
    }
    let _ = std::fs::remove_file(&partial);
    false
}

/// The record at `path`, or `None` when there is none that can be read or it is longer than any
/// record this build writes.
fn read_manifest(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut text = String::new();
    let read = file.take(MANIFEST_MAX_BYTES + 1).read_to_string(&mut text).ok()?;
    (read as u64 <= MANIFEST_MAX_BYTES).then_some(text)
}

/// The digest each installed file was written with, by the file's name, out of a record's text.
///
/// The record is a file in a folder the player may edit, so a line is believed only as far as it
/// can do no harm: a name that would leave the pack's folder is passed over, as is anything that is
/// not a digest.
fn parse_manifest(text: &str) -> BTreeMap<String, String> {
    let mut lines = text.lines();
    if lines.next() != Some(MANIFEST_HEADER) {
        return BTreeMap::new();
    }
    lines
        .filter_map(|line| line.split_once(MANIFEST_SEPARATOR))
        .filter(|(digest, name)| is_digest(digest) && is_pack_relative(name))
        .map(|(digest, name)| (name.to_owned(), digest.to_owned()))
        .collect()
}

/// A record's text: the header, then one line per installed file in the order of their names.
fn manifest_text(record: &BTreeMap<String, String>) -> String {
    let mut text = format!("{MANIFEST_HEADER}\n");
    for (name, digest) in record {
        text.push_str(digest);
        text.push(MANIFEST_SEPARATOR);
        text.push_str(name);
        text.push('\n');
    }
    text
}

/// Whether `text` is a digest as the record writes one.
fn is_digest(text: &str) -> bool {
    text.len() == DIGEST_HEX_CHARS && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `name` is a path that stays inside the pack's folder: parts separated by `/`, none of
/// them empty, a step up or down, or holding a separator or a drive of the platform's own.
fn is_pack_relative(name: &str) -> bool {
    name.split(NAME_SEPARATOR).all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(NAME_FORBIDDEN))
}

#[cfg(test)]
mod tests;
