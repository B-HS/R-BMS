//! Schema versioning: the DDL every connection applies, the `meta` rows that stamp it, and the
//! check that stops a database written by a newer build from being opened at all.

use rusqlite::Connection;

use super::SongDbError;

/// Layout generation of the tables in `schema.sql`. A stored database naming a higher one is not
/// opened; a lower one is stepped up here before the accessors run.
pub const SCHEMA_VERSION: u32 = 1;

/// Generation of the parse that filled the rows. Raising it makes the next scan re-read every
/// chart even when its file has not changed, which is how a parser or model change reaches charts
/// that are already stored.
pub const PARSER_VERSION: u32 = 1;

/// Connection settings every open applies: the write-ahead log so a scan writing in one connection
/// does not block the browser reading in another, and the foreign keys that make `song_detail`
/// follow its `song` row into the grave.
pub(crate) const PRAGMAS: &str = "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;";

/// The tables, indexes and their creation order.
pub(crate) const DDL: &str = include_str!("schema.sql");

/// `meta` key holding [`SCHEMA_VERSION`] as written by the build that created the file.
pub(crate) const META_SCHEMA_VERSION: &str = "schema_version";
/// `meta` key holding [`PARSER_VERSION`] as of the last completed scan.
pub(crate) const META_PARSER_VERSION: &str = "parser_version";
/// `meta` key holding the epoch second the database was created.
pub(crate) const META_CREATED_AT: &str = "created_at";

/// Read one `meta` value, or `None` when the key was never written.
pub(crate) fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>, SongDbError> {
    let mut stmt = conn.prepare_cached("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

/// Write one `meta` value, replacing whatever was there.
pub(crate) fn write_meta(conn: &Connection, key: &str, value: &str) -> Result<(), SongDbError> {
    conn.prepare_cached("INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value")?.execute((key, value))?;
    Ok(())
}

/// Read a `meta` value that holds a number, treating an unparsable one as absent.
pub(crate) fn read_meta_u32(conn: &Connection, key: &str) -> Result<Option<u32>, SongDbError> {
    Ok(read_meta(conn, key)?.and_then(|v| v.trim().parse::<u32>().ok()))
}

/// Bring `conn` up to [`SCHEMA_VERSION`], returning the version it now holds.
///
/// A file written by a newer build is refused rather than migrated down: its rows may carry columns
/// this build would silently drop on the next upsert. Every other case is idempotent, so calling
/// this on an already-current database is a no-op and calling it on an empty file creates the whole
/// layout.
pub(crate) fn migrate(conn: &Connection) -> Result<u32, SongDbError> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)")?;
    let stored = read_meta_u32(conn, META_SCHEMA_VERSION)?;
    if let Some(found) = stored
        && found > SCHEMA_VERSION
    {
        return Err(SongDbError::Schema(format!("song database is version {found}, newer than the {SCHEMA_VERSION} this build reads")));
    }
    conn.execute_batch(DDL)?;
    if stored != Some(SCHEMA_VERSION) {
        write_meta(conn, META_SCHEMA_VERSION, &SCHEMA_VERSION.to_string())?;
    }
    if read_meta(conn, META_CREATED_AT)?.is_none() {
        write_meta(conn, META_CREATED_AT, &now_secs().to_string())?;
    }
    Ok(SCHEMA_VERSION)
}

/// The current time in whole seconds since the epoch, which is the unit every stored timestamp
/// uses. A clock before the epoch reads as `0` rather than panicking.
pub(crate) fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
