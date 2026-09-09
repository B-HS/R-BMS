use std::path::{Path, PathBuf};

use rbms_store::write_atomic;

use crate::error::ConfigError;
use crate::legacy::{LEGACY_FOLDERS_FILE, LEGACY_TABLES_FILE, LegacyV0, merge_legacy_lists};
use crate::schema::{CURRENT_SCHEMA_VERSION, Config, LEGACY_SCHEMA_VERSION, SINGLE_JUDGE_WIDTH_SCHEMA_VERSION};

/// Extension a configuration file that could not be parsed is moved aside under, so the next save
/// cannot silently clobber a recoverable file.
const BACKUP_EXTENSION: &str = "ron.bak";

/// Extension a migrated file is copied to before the caller writes the current schema over it.
/// Named after the schema it holds, so downgrading to a build that reads that schema is a rename.
fn migration_backup_extension(from: u32) -> String {
    format!("ron.v{from}.bak")
}

/// What one [`load`] produced: the configuration itself, the schema it was migrated from (`None`
/// when the file already used the current schema), and where an unreadable file was moved to.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadOutcome {
    pub config: Config,
    pub migrated_from: Option<u32>,
    pub backup: Option<PathBuf>,
}

/// Read the configuration at `path`, migrating a pre-version file (and folding in its `folders.ron`
/// / `tables.ron` siblings) on the way.
///
/// A missing file is not an error: the defaults are written out so the first run leaves a file to
/// edit. A file that cannot be parsed is moved aside to `*.ron.bak` and the defaults are used
/// without being written, exactly as the per-file loaders did before the unification.
///
/// The one hard failure is a file that declares a newer schema: it is reported as
/// [`ConfigError::Migrate`] and left untouched, so running an older build never destroys a
/// configuration a newer one wrote.
pub fn load(path: &Path) -> Result<LoadOutcome, ConfigError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(ConfigError::Read(e)),
        Err(_) => {
            let mut config = Config::default();
            merge_legacy_lists(&mut config, sibling(path, LEGACY_FOLDERS_FILE).as_deref(), sibling(path, LEGACY_TABLES_FILE).as_deref());
            if let Err(e) = save(&config, path) {
                eprintln!("settings save failed ({}): {e}", path.display());
            }
            return Ok(LoadOutcome { config, migrated_from: None, backup: None });
        }
    };
    match migrate(&raw) {
        Ok((config, None)) => Ok(LoadOutcome { config, migrated_from: None, backup: None }),
        Ok((mut config, Some(from))) => {
            merge_legacy_lists(&mut config, sibling(path, LEGACY_FOLDERS_FILE).as_deref(), sibling(path, LEGACY_TABLES_FILE).as_deref());
            let backup = keep_migrated_original(path, &raw, from);
            Ok(LoadOutcome { config, migrated_from: Some(from), backup })
        }
        Err(fatal @ ConfigError::Migrate { .. }) => Err(fatal),
        Err(e) => {
            let backup = path.with_extension(BACKUP_EXTENSION);
            let _ = std::fs::rename(path, &backup);
            eprintln!("settings parse failed ({e}); backed up to {} and using defaults", backup.display());
            let mut config = Config::default();
            merge_legacy_lists(&mut config, sibling(path, LEGACY_FOLDERS_FILE).as_deref(), sibling(path, LEGACY_TABLES_FILE).as_deref());
            Ok(LoadOutcome { config, migrated_from: None, backup: Some(backup) })
        }
    }
}

/// Write the configuration to `path` atomically, always under the current schema version.
pub fn save(config: &Config, path: &Path) -> Result<(), ConfigError> {
    let text = ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default())?;
    write_atomic(path, &text).map_err(ConfigError::Write)
}

/// Parse one configuration document, migrating it forward if it predates the current schema, and
/// report which schema it came from (`None` when it needed no migration).
///
/// The result is always sanitised, so no caller has to re-check ranges of a value that came from a
/// file, a hand edit or an account.
pub fn migrate(raw: &str) -> Result<(Config, Option<u32>), ConfigError> {
    let probe: SchemaProbe = ron::from_str(raw)?;
    let mut config = match probe.schema_version {
        LEGACY_SCHEMA_VERSION => Config::from(ron::from_str::<LegacyV0>(raw)?),
        SINGLE_JUDGE_WIDTH_SCHEMA_VERSION => {
            let mut config = ron::from_str::<Config>(raw)?;
            config.judge.spread_uniform_judge_rate(ron::from_str::<SingleJudgeWidthProbe>(raw)?.judge.judge_rate);
            config
        }
        CURRENT_SCHEMA_VERSION => ron::from_str::<Config>(raw)?,
        from => {
            return Err(ConfigError::Migrate { from, reason: format!("this build reads up to schema version {CURRENT_SCHEMA_VERSION}") });
        }
    };
    config.sanitise();
    let migrated_from = (probe.schema_version != CURRENT_SCHEMA_VERSION).then_some(probe.schema_version);
    Ok((config, migrated_from))
}

/// Copy a migrated file aside before the caller writes the current schema over the original, so
/// rolling back to the build that wrote it is a rename rather than a lost configuration. A failed
/// copy is reported and the migration continues: the alternative is refusing to start.
fn keep_migrated_original(path: &Path, raw: &str, from: u32) -> Option<PathBuf> {
    let backup = path.with_extension(migration_backup_extension(from));
    match std::fs::write(&backup, raw) {
        Ok(()) => Some(backup),
        Err(e) => {
            eprintln!("settings backup failed ({}): {e}", backup.display());
            None
        }
    }
}

/// Contents of a pre-migration list file sitting next to the configuration.
fn sibling(path: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(path.parent()?.join(name)).ok()
}

/// Reads only the version out of a configuration document, so the rest can be parsed as the schema
/// that version names. Serde ignores the fields it does not know, which is every other field here.
#[derive(serde::Deserialize)]
#[serde(default)]
struct SchemaProbe {
    schema_version: u32,
}

impl Default for SchemaProbe {
    fn default() -> Self {
        SchemaProbe { schema_version: LEGACY_SCHEMA_VERSION }
    }
}

/// Reads only the one JUDGE WIDTH percentage a schema-1 document held, so the six per-tier rows it
/// became can all start from the width that file asked for. The rest of the document parses as the
/// current schema: nothing else moved between the two.
#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct SingleJudgeWidthProbe {
    judge: SingleJudgeWidthGroup,
}

#[derive(serde::Deserialize)]
#[serde(default)]
struct SingleJudgeWidthGroup {
    judge_rate: i32,
}

impl Default for SingleJudgeWidthGroup {
    fn default() -> Self {
        SingleJudgeWidthGroup { judge_rate: crate::schema::JUDGE_RATE_DEFAULT_PERCENT }
    }
}
