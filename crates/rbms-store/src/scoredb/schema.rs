use rusqlite::{Connection, OptionalExtension};

use super::ScoreDbError;

/// The DDL every score database is created from. Applied with `CREATE TABLE IF NOT EXISTS`, so
/// running it against an existing database is a no-op.
pub(super) const SCHEMA_SQL: &str = include_str!("schema.sql");

/// Schema generation this build reads and writes. A database stamped with a higher generation was
/// written by a newer build and is refused rather than rewritten, the same policy the settings file
/// follows.
pub const SCORE_DB_SCHEMA_VERSION: u32 = 1;

/// `meta` key holding [`SCORE_DB_SCHEMA_VERSION`].
pub(super) const META_SCHEMA_VERSION: &str = "schema_version";

/// `meta` key holding the epoch second the database was created at.
pub(super) const META_CREATED_AT: &str = "created_at";

/// Write-ahead logging keeps a reader (the select screen) from blocking the writer (the result
/// screen), which is the only concurrency this database has.
const JOURNAL_MODE_WAL: &str = "PRAGMA journal_mode = WAL";

/// One fsync per checkpoint rather than per commit. Losing the last few plays to a power cut is
/// acceptable here; stalling every result screen on a disk flush is not.
const SYNCHRONOUS_NORMAL: &str = "PRAGMA synchronous = NORMAL";

/// Apply the connection-level pragmas. `journal_mode` reports the mode it settled on, which an
/// in-memory database answers as `memory`, so its row is read and discarded rather than treated as
/// a failure.
pub(super) fn apply_pragmas(conn: &Connection) -> Result<(), ScoreDbError> {
    conn.query_row(JOURNAL_MODE_WAL, [], |row| row.get::<_, String>(0))?;
    conn.execute_batch(SYNCHRONOUS_NORMAL)?;
    Ok(())
}

/// Create the tables if they are absent and settle the schema generation, returning the generation
/// the database now carries.
pub(super) fn migrate(conn: &mut Connection) -> Result<u32, ScoreDbError> {
    let found = read_schema_version(conn)?;
    if let Some(found) = found
        && found > SCORE_DB_SCHEMA_VERSION
    {
        return Err(ScoreDbError::FutureSchema { found, supported: SCORE_DB_SCHEMA_VERSION });
    }
    let tx = conn.transaction()?;
    tx.execute_batch(SCHEMA_SQL)?;
    tx.execute("INSERT OR REPLACE INTO meta(key, value) VALUES(?1, ?2)", (META_SCHEMA_VERSION, SCORE_DB_SCHEMA_VERSION.to_string()))?;
    tx.execute("INSERT OR IGNORE INTO meta(key, value) VALUES(?1, ?2)", (META_CREATED_AT, now_epoch_seconds().to_string()))?;
    tx.commit()?;
    Ok(SCORE_DB_SCHEMA_VERSION)
}

/// The generation stamped on the database, or `None` when it has never been migrated (no `meta`
/// table, or no row in it).
fn read_schema_version(conn: &Connection) -> Result<Option<u32>, ScoreDbError> {
    let has_meta: Option<String> = conn.query_row("SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'meta'", [], |row| row.get(0)).optional()?;
    if has_meta.is_none() {
        return Ok(None);
    }
    let raw: Option<String> = conn.query_row("SELECT value FROM meta WHERE key = ?1", [META_SCHEMA_VERSION], |row| row.get(0)).optional()?;
    Ok(raw.and_then(|v| v.parse().ok()))
}

/// Seconds since the epoch, or zero on a system clock set before it.
pub(super) fn now_epoch_seconds() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Play-mode id for a mode that is not one of the known ones, and the id a record whose mode string
/// cannot be parsed is stored under.
pub const MODE_UNKNOWN: i32 = 0;

/// Play mode names and the integer ids they are stored under. The ids are the reference
/// implementation's `Mode` ids, read off the mode-id switch in `PlayerConfig.java:413-430`, so a
/// database written here means the same thing as one written there.
const MODE_IDS: &[(&str, i32)] = &[("BEAT_5K", 5), ("BEAT_7K", 7), ("POPN_9K", 9), ("BEAT_10K", 10), ("BEAT_14K", 14), ("KEYBOARD_24K", 25)];

/// Gauge id for a token that is not one of the known ones.
pub const GAUGE_UNKNOWN: i32 = 0;

/// Gauge tokens and the ids they are stored under, in the reference implementation's
/// `GrooveGauge` index order — the same order `rbms_judge::gauge::GaugeIndex` is declared in. The
/// aliases match what `rbms_config::gauge_from_name` accepts, so a token that crate writes always
/// reads back as the gauge it named.
const GAUGE_IDS: &[(&str, i32)] = &[
    ("assist", 0),
    ("assisteasy", 0),
    ("easy", 1),
    ("normal", 2),
    ("hard", 3),
    ("exhard", 4),
    ("hazard", 5),
    ("class", 6),
    ("exclass", 7),
    ("exhardclass", 8),
];

/// Canonical token per gauge id, indexed by the id itself.
const GAUGE_TOKENS: &[&str] = &["assist", "easy", "normal", "hard", "exhard", "hazard", "class", "exclass", "exhardclass"];

/// Note-shuffle id for a label that is not one of the known ones, which is also the id of OFF: an
/// unreadable option is stored as no option rather than as a shuffle nothing can name.
pub const RANDOM_OFF: i32 = 0;

/// Note-shuffle labels and the ids they are stored under, in `rbms_chart::shuffle::NoteOption::ALL`
/// order. Separators are stripped before the lookup, so `S-RANDOM`, `S_RANDOM` and `srandom` all
/// resolve to the same id.
const RANDOM_IDS: &[(&str, i32)] =
    &[("off", 0), ("mirror", 1), ("random", 2), ("srandom", 3), ("rrandom", 4), ("rotate", 5), ("spiral", 5), ("hrandom", 6), ("allscratch", 7)];

/// Canonical label per shuffle id, indexed by the id itself.
const RANDOM_LABELS: &[&str] = &["OFF", "MIRROR", "RANDOM", "S-RANDOM", "R-RANDOM", "ROTATE", "H-RANDOM", "ALL-SCRATCH"];

/// The id a play mode is stored under. Both separator spellings of a mode name are accepted
/// (`BEAT_7K` and `BEAT-7K`), because records written before the modes had one canonical spelling
/// carry either. An unknown name is [`MODE_UNKNOWN`].
pub fn mode_id(name: &str) -> i32 {
    let key = normalise_mode(name);
    MODE_IDS.iter().find(|(n, _)| *n == key).map(|(_, id)| *id).unwrap_or(MODE_UNKNOWN)
}

/// Canonical name of a stored play-mode id, or an empty string when nothing is stored under it.
pub fn mode_name(id: i32) -> &'static str {
    MODE_IDS.iter().find(|(_, i)| *i == id).map(|(n, _)| *n).unwrap_or("")
}

/// The id a gauge token is stored under, or [`GAUGE_UNKNOWN`] for a token nothing names.
pub fn gauge_id(token: &str) -> i32 {
    let key = token.to_ascii_lowercase();
    GAUGE_IDS.iter().find(|(t, _)| *t == key).map(|(_, id)| *id).unwrap_or(GAUGE_UNKNOWN)
}

/// Canonical token of a stored gauge id, or an empty string when nothing is stored under it.
pub fn gauge_token(id: i32) -> &'static str {
    usize::try_from(id).ok().and_then(|i| GAUGE_TOKENS.get(i)).copied().unwrap_or("")
}

/// The id a note-shuffle label is stored under, or [`RANDOM_OFF`] for a label nothing names.
pub fn random_id(label: &str) -> i32 {
    let key: String = label.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect();
    RANDOM_IDS.iter().find(|(t, _)| *t == key).map(|(_, id)| *id).unwrap_or(RANDOM_OFF)
}

/// Canonical label of a stored shuffle id, or an empty string when nothing is stored under it.
pub fn random_token(id: i32) -> &'static str {
    usize::try_from(id).ok().and_then(|i| RANDOM_LABELS.get(i)).copied().unwrap_or("")
}

/// Upper-case a mode name and fold its separator, so one lookup table covers every spelling a
/// record can carry.
fn normalise_mode(name: &str) -> String {
    name.chars().map(|c| if c == '-' { '_' } else { c.to_ascii_uppercase() }).collect()
}
