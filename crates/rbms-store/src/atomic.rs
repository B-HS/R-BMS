use std::path::Path;

/// Suffix of the sibling temp file [`write_atomic`] writes before renaming it over the target.
const TEMP_WRITE_SUFFIX: &str = ".tmp";

/// Write `contents` to `path` atomically: create the parent directory, write a sibling temp file,
/// flush it, then rename it over the target. A crash or a full disk mid-write can then never leave a
/// *truncated* config/score/replay file behind — the target is either the old contents or the new
/// ones. The rename itself is not durable (the parent directory is not fsynced), so a crash right
/// after it may still lose the update. The temp name carries the process id so two instances saving
/// the same file do not clobber each other's temp. The temp file is removed if anything fails.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name"));
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_file_name(format!("{name}.{}{TEMP_WRITE_SUFFIX}", std::process::id()));
    let written = std::fs::File::create(&tmp).and_then(|mut f| {
        f.write_all(contents.as_bytes())?;
        f.sync_all()
    });
    if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}
