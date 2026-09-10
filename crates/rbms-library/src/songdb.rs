//! The SQLite song database: the persisted form of the library index, its schema and migrations,
//! and the row types the select screen reads back.
//!
//! The library used to live only in memory, rebuilt by re-parsing every chart in every configured
//! folder on each run. This module is what lets a scan be incremental instead: rows carry the file
//! stamp they were parsed from ([`SongRow::date`] and [`SongRow::size`]), so a rescan re-reads only
//! the charts whose files moved under it, and the browser can open on the stored rows before any
//! scan runs at all.
//!
//! Column names follow the reference implementation's `song` table so a database it wrote can be
//! imported later; see [`rows`] for how the two sets are kept apart.

pub mod rows;
mod schema;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::Connection;

pub use rows::{
    CONTENT_BGA, CONTENT_NO_KEYSOUND, CONTENT_PREVIEW, CONTENT_TEXT, DetailRow, FEATURE_CHARGE_NOTE, FEATURE_HELL_CHARGE_NOTE, FEATURE_LONG_NOTE,
    FEATURE_MINE_NOTE, FEATURE_RANDOM, FEATURE_SCROLL, FEATURE_STOP_SEQUENCE, FEATURE_UNDEFINED_LN, SongRow, mode_from_id, mode_id,
};
pub(crate) use schema::now_secs;
pub use schema::{PARSER_VERSION, SCHEMA_VERSION};

use rows::{SONG_COLUMNS, UPSERT_DETAIL, UPSERT_SONG, numeric_level, pack_density, song_from_row, unpack_density, upsert_params};

/// Everything that can go wrong reading or writing the song database.
#[derive(Debug)]
pub enum SongDbError {
    /// The statement itself failed.
    Sqlite(rusqlite::Error),
    /// The file is there and readable but its layout is not one this build can use.
    Schema(String),
}

impl std::fmt::Display for SongDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SongDbError::Sqlite(e) => write!(f, "{e}"),
            SongDbError::Schema(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SongDbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SongDbError::Sqlite(e) => Some(e),
            SongDbError::Schema(_) => None,
        }
    }
}

impl From<rusqlite::Error> for SongDbError {
    fn from(e: rusqlite::Error) -> SongDbError {
        SongDbError::Sqlite(e)
    }
}

/// The song database. One connection; open a second one for a background scan rather than sharing
/// this one, which is what the write-ahead log is turned on for.
#[derive(Debug)]
pub struct SongDb {
    conn: Connection,
}

impl SongDb {
    /// Open (creating if absent) the database at `path` and apply the connection settings. The
    /// layout is not touched until [`SongDb::migrate`] runs.
    pub fn open(path: &Path) -> Result<SongDb, SongDbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(schema::PRAGMAS)?;
        Ok(SongDb { conn })
    }

    /// A private database that lives only as long as the value, for tests.
    pub fn open_in_memory() -> Result<SongDb, SongDbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(schema::PRAGMAS)?;
        Ok(SongDb { conn })
    }

    /// Create or step up the layout, returning the schema version now stored. Refuses a database
    /// written by a newer build instead of writing rows it would not understand.
    pub fn migrate(&mut self) -> Result<u32, SongDbError> {
        schema::migrate(&self.conn)
    }

    /// The parse generation the last completed scan ran under, or `None` before the first one.
    pub fn parser_version(&self) -> Result<Option<u32>, SongDbError> {
        schema::read_meta_u32(&self.conn, schema::META_PARSER_VERSION)
    }

    /// Stamp the parse generation. A scan calls this when it completes, never when it is cancelled,
    /// so an interrupted upgrade rescan resumes on the next run.
    pub fn set_parser_version(&self, version: u32) -> Result<(), SongDbError> {
        schema::write_meta(&self.conn, schema::META_PARSER_VERSION, &version.to_string())
    }

    /// Whether the stored rows were parsed by a different generation of the parser, which is what
    /// makes the next scan a full one regardless of file stamps.
    pub fn needs_full_rescan(&self) -> Result<bool, SongDbError> {
        Ok(self.parser_version()? != Some(PARSER_VERSION))
    }

    /// Every stored chart, title-ordered so the browser can show them without sorting first.
    pub fn all_songs(&self) -> Result<Vec<SongRow>, SongDbError> {
        let sql = format!("SELECT {SONG_COLUMNS} FROM song ORDER BY title COLLATE NOCASE, path");
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let rows = stmt.query_map([], song_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Every stored chart that lives under one of `roots`, in the same order as [`SongDb::all_songs`].
    /// A chart under two overlapping roots is returned once.
    pub fn songs_in_folders(&self, roots: &[String]) -> Result<Vec<SongRow>, SongDbError> {
        let sql = format!("SELECT {SONG_COLUMNS} FROM song WHERE path >= ?1 AND path < ?2 ORDER BY title COLLATE NOCASE, path");
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<SongRow> = Vec::new();
        for root in roots {
            let prefix = with_trailing_slash(root);
            let upper = prefix_upper_bound(&prefix);
            let rows = stmt.query_map([&prefix, &upper], song_from_row)?;
            for row in rows {
                let row = row?;
                if seen.insert(row.path.clone()) {
                    out.push(row);
                }
            }
        }
        out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()).then_with(|| a.path.cmp(&b.path)));
        Ok(out)
    }

    /// Every stored chart with this md5. More than one row is normal: the same chart file can sit
    /// in two folders.
    pub fn by_md5(&self, md5: &str) -> Result<Vec<SongRow>, SongDbError> {
        let sql = format!("SELECT {SONG_COLUMNS} FROM song WHERE md5 = ?1 ORDER BY path");
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let rows = stmt.query_map([md5.to_ascii_lowercase()], song_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// One stored chart by path.
    pub fn song(&self, path: &str) -> Result<Option<SongRow>, SongDbError> {
        let sql = format!("SELECT {SONG_COLUMNS} FROM song WHERE path = ?1");
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let mut rows = stmt.query([path])?;
        match rows.next()? {
            Some(row) => Ok(Some(song_from_row(row)?)),
            None => Ok(None),
        }
    }

    /// How many charts are stored, which is what a scan in progress is measured against.
    pub fn song_count(&self) -> Result<usize, SongDbError> {
        let mut stmt = self.conn.prepare_cached("SELECT COUNT(*) FROM song")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count.max(0) as usize)
    }

    /// The cached detail panel of one chart, or `None` when it was never computed.
    pub fn detail(&self, path: &str) -> Result<Option<DetailRow>, SongDbError> {
        let mut stmt = self.conn.prepare_cached("SELECT duration_us, peak_density, avg_density, end_density, density_bins FROM song_detail WHERE path = ?1")?;
        let mut rows = stmt.query([path])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let blob: Vec<u8> = row.get(4)?;
        Ok(Some(DetailRow {
            duration_us: row.get(0)?,
            peak_density: row.get(1)?,
            avg_density: row.get(2)?,
            end_density: row.get(3)?,
            density: unpack_density(&blob),
        }))
    }

    /// Cache one chart's detail panel. The chart has to be stored first: the detail row is keyed on
    /// it and goes when it goes.
    pub fn put_detail(&self, path: &str, detail: &DetailRow) -> Result<(), SongDbError> {
        self.conn.prepare_cached(UPSERT_DETAIL)?.execute(rusqlite::params![
            path,
            detail.duration_us,
            detail.peak_density,
            detail.avg_density,
            detail.end_density,
            pack_density(&detail.density)
        ])?;
        Ok(())
    }

    /// Star or unstar one chart. This is user data, so a rescan never writes it.
    pub fn set_favorite(&self, path: &str, favorite: i32) -> Result<(), SongDbError> {
        self.conn.prepare_cached("UPDATE song SET favorite = ?2 WHERE path = ?1")?.execute((path, favorite))?;
        Ok(())
    }

    /// Path to `(mtime seconds, size)` for every stored chart: what an incremental scan compares
    /// the files it walks against.
    pub fn stamp_index(&self) -> Result<HashMap<String, (i64, i64)>, SongDbError> {
        let mut stmt = self.conn.prepare_cached("SELECT path, date, rbms_size FROM song")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))))?;
        let mut out = HashMap::new();
        for row in rows {
            let (path, stamp) = row?;
            out.insert(path, stamp);
        }
        Ok(out)
    }

    /// Insert or refresh a batch of charts in one transaction, so a scan's partial results are
    /// queryable between batches and never half-written inside one.
    pub fn upsert_batch(&mut self, rows: &[SongRow]) -> Result<(), SongDbError> {
        let scanned_at = now_secs();
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(UPSERT_SONG)?;
            for row in rows {
                let level_num = numeric_level(&row.level);
                stmt.execute(upsert_params(row, &level_num, &scanned_at).as_slice())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Insert or refresh a batch of charts together with the detail each scan already computed for
    /// them, in one transaction.
    pub fn upsert_batch_with_details(&mut self, entries: &[(SongRow, Option<DetailRow>)]) -> Result<(), SongDbError> {
        let scanned_at = now_secs();
        let tx = self.conn.transaction()?;
        {
            let mut song = tx.prepare_cached(UPSERT_SONG)?;
            let mut detail = tx.prepare_cached(UPSERT_DETAIL)?;
            for (row, cached) in entries {
                let level_num = numeric_level(&row.level);
                song.execute(upsert_params(row, &level_num, &scanned_at).as_slice())?;
                if let Some(d) = cached {
                    detail.execute(rusqlite::params![row.path, d.duration_us, d.peak_density, d.avg_density, d.end_density, pack_density(&d.density)])?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Drop every stored chart that lives under one of `roots` and was not walked this time, which
    /// is how a deleted file leaves the browser. Charts outside `roots` are left alone: a folder
    /// that is no longer configured keeps its rows rather than losing them to a scan of the others.
    pub fn delete_missing(&mut self, alive: &HashSet<String>, roots: &[String]) -> Result<usize, SongDbError> {
        let prefixes: Vec<String> = roots.iter().map(|r| with_trailing_slash(r)).collect();
        let stored: Vec<String> = {
            let mut stmt = self.conn.prepare_cached("SELECT path FROM song")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let doomed: Vec<String> = stored.into_iter().filter(|p| !alive.contains(p) && prefixes.iter().any(|prefix| p.starts_with(prefix.as_str()))).collect();
        if doomed.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("DELETE FROM song WHERE path = ?1")?;
            for path in &doomed {
                stmt.execute([path])?;
            }
        }
        tx.commit()?;
        Ok(doomed.len())
    }

    /// The column names one table actually has, which is what the schema test reads.
    pub fn columns_of(&self, table: &str) -> Result<Vec<String>, SongDbError> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

/// A folder path that ends in exactly one separator, so a prefix test cannot match a sibling whose
/// name merely starts the same way (`/songs/pack` against `/songs/pack2`).
pub(crate) fn with_trailing_slash(root: &str) -> String {
    let trimmed = root.trim_end_matches('/');
    format!("{trimmed}/")
}

/// The exclusive upper bound of the ordered range holding every string that starts with `prefix`.
fn prefix_upper_bound(prefix: &str) -> String {
    let mut bound = prefix.to_string();
    bound.push(char::MAX);
    bound
}
