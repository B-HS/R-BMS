use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Suffix of the sibling temp file [`write_atomic`] writes before renaming it over the target.
const TEMP_WRITE_SUFFIX: &str = ".tmp";

/// Unix mode of a file only its owner may read or write: `rw-------`.
#[cfg(unix)]
const OWNER_ONLY_MODE: u32 = 0o600;

/// Numbers every temp file this process opens, so two writes of the same file from two threads of
/// one process never share one.
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Write `contents` to `path` atomically: create the parent directory, write a sibling temp file,
/// flush it, then rename it over the target. A crash or a full disk mid-write can then never leave a
/// *truncated* config/score/replay file behind — the target is either the old contents or the new
/// ones. The rename itself is not durable (the parent directory is not fsynced), so a crash right
/// after it may still lose the update. The temp file is removed if anything fails.
///
/// The temp name carries the process id and a number of this process's own, so neither two
/// instances nor two threads of one instance saving the same file clobber each other's temp: each
/// write renames whole contents over the target, and the last rename wins.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    StagedWrite::create(path)?.finish(contents.as_bytes())
}

/// [`write_atomic`] for a file that holds a credential: on Unix the file is created readable and
/// writable by its owner alone (`0600`) and keeps that mode when it replaces a file that was open
/// to others. Elsewhere it is [`write_atomic`]: a Windows file inherits the access list of the
/// folder it is created in, which under the user profile admits only the user, the system and the
/// administrators.
pub fn write_atomic_private(path: &Path, contents: &str) -> std::io::Result<()> {
    StagedWrite::create_private(path)?.finish(contents.as_bytes())
}

/// Close a file an older build left open to other users: `0600` on Unix, nothing elsewhere.
///
/// Only a regular file is changed. A folder closed to everyone but its owner for reading and
/// writing could not be entered, and a link would change the file it points at, which is not this
/// file's to decide; both are left as they are. A path that is not there is a
/// [`std::io::ErrorKind::NotFound`] error.
pub fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !std::fs::symlink_metadata(path)?.file_type().is_file() {
            return Ok(());
        }
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(OWNER_ONLY_MODE))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn open_default(temp: &Path) -> std::io::Result<File> {
    File::create(temp)
}

#[cfg(unix)]
fn open_owner_only(temp: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(OWNER_ONLY_MODE).open(temp)?;
    file.set_permissions(std::fs::Permissions::from_mode(OWNER_ONLY_MODE))?;
    Ok(file)
}

#[cfg(not(unix))]
fn open_owner_only(temp: &Path) -> std::io::Result<File> {
    File::create(temp)
}

/// A write of [`write_atomic`] split where it costs: the temp file is created first, which is where
/// a missing directory, a file in the way or a refused permission shows, and the contents are
/// written, flushed and renamed over the target later, possibly on another thread.
///
/// Dropped without [`StagedWrite::finish`], it removes its temp file and leaves the target as it was.
#[derive(Debug)]
pub struct StagedWrite {
    target: PathBuf,
    temp: PathBuf,
    file: Option<File>,
}

impl StagedWrite {
    /// Create the parent directory of `path` and the temp file the contents will be written to.
    pub fn create(path: &Path) -> std::io::Result<StagedWrite> {
        StagedWrite::create_opening(path, open_default)
    }

    /// [`StagedWrite::create`] for a file that holds a credential, as [`write_atomic_private`] says.
    pub fn create_private(path: &Path) -> std::io::Result<StagedWrite> {
        StagedWrite::create_opening(path, open_owner_only)
    }

    fn create_opening(path: &Path, open: fn(&Path) -> std::io::Result<File>) -> std::io::Result<StagedWrite> {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name"));
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!("{name}.{}.{sequence}{TEMP_WRITE_SUFFIX}", std::process::id()));
        let file = open(&temp)?;
        Ok(StagedWrite { target: path.to_path_buf(), temp, file: Some(file) })
    }

    /// The file the contents replace.
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// Write `contents`, flush them to the disk and rename them over the target.
    pub fn finish(mut self, contents: &[u8]) -> std::io::Result<()> {
        let Some(mut file) = self.file.take() else {
            return Err(std::io::Error::other("staged write already finished"));
        };
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&self.temp, &self.target)?;
        self.temp = PathBuf::new();
        Ok(())
    }
}

impl Drop for StagedWrite {
    fn drop(&mut self) {
        self.file = None;
        if !self.temp.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.temp);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const OPEN_TO_OTHERS: u32 = 0o644;
    const PERMISSION_BITS: u32 = 0o777;

    /// A temp file left by a crashed run that has the name of the next one keeps the mode it had
    /// when it is opened again, which would let a credential be written into a file others can read.
    #[test]
    fn opening_a_temp_file_that_is_already_there_closes_it_to_other_users() {
        let dir = std::env::temp_dir().join(format!("rbms_store_open_owner_only_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the folder");
        let temp = dir.join("settings.ron.1.0.tmp");
        std::fs::write(&temp, "left by a crash").expect("written");
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(OPEN_TO_OTHERS)).expect("opened");

        drop(open_owner_only(&temp).expect("opened again"));

        let mode = std::fs::metadata(&temp).expect("the file is there").permissions().mode() & PERMISSION_BITS;
        assert_eq!(mode, OWNER_ONLY_MODE);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
