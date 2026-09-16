//! The row types the song database stores and the column lists that read and write them.
//!
//! Column names mirror the reference implementation's `song` table (`SQLiteSongDatabaseAccessor.java`,
//! `SongData.java`) so a database written by it can be imported later without renaming anything.
//! Columns rbms adds for its own use carry an `rbms_` prefix, which is what keeps the two sets
//! apart.

use rbms_model::Mode;

/// A long note whose flavour the chart left unstated, so the player's LN MODE decides it.
pub const FEATURE_UNDEFINED_LN: i32 = 1;
/// The chart places mines.
pub const FEATURE_MINE_NOTE: i32 = 2;
/// The chart is built with `#RANDOM`/`#SWITCH` control flow.
pub const FEATURE_RANDOM: i32 = 4;
/// The chart states plain long notes.
pub const FEATURE_LONG_NOTE: i32 = 8;
/// The chart states charge notes.
pub const FEATURE_CHARGE_NOTE: i32 = 16;
/// The chart states hell charge notes.
pub const FEATURE_HELL_CHARGE_NOTE: i32 = 32;
/// The chart stops the scroll at least once.
pub const FEATURE_STOP_SEQUENCE: i32 = 64;
/// The chart changes the scroll speed at least once.
pub const FEATURE_SCROLL: i32 = 128;

/// A text file sits next to the chart.
pub const CONTENT_TEXT: i32 = 1;
/// The chart names background images.
pub const CONTENT_BGA: i32 = 2;
/// A preview clip was resolved for the chart.
pub const CONTENT_PREVIEW: i32 = 4;
/// The chart is long but names almost no samples, so it plays as one streamed track rather than
/// from keysounds.
pub const CONTENT_NO_KEYSOUND: i32 = 128;

/// Length, in ms, a chart has to reach before it can be called keysound-less at all.
pub const NO_KEYSOUND_MIN_LENGTH_MS: i64 = 30_000;
/// Milliseconds of chart per allowed sample in the keysound-less test.
pub const NO_KEYSOUND_MS_PER_SAMPLE: i64 = 50_000;
/// Samples a keysound-less chart may name on top of the per-length allowance.
pub const NO_KEYSOUND_SAMPLE_SLACK: i64 = 3;

/// Mode ids as the reference implementation's `Mode` enum numbers them, which is what its `song.mode`
/// column holds. Read out of `Mode.class` (`bms/model/Mode`): `BEAT_5K(5)`, `BEAT_7K(7)`,
/// `BEAT_10K(10)`, `BEAT_14K(14)`, `POPN_9K(9)`, `KEYBOARD_24K(25)`.
const MODE_IDS: &[(Mode, i32)] =
    &[(Mode::BEAT_5K, 5), (Mode::BEAT_7K, 7), (Mode::BEAT_10K, 10), (Mode::BEAT_14K, 14), (Mode::POPN_9K, 9), (Mode::KEYBOARD_24K, 25)];

/// The stored id of a play mode. An unknown mode stores `0`, the value the reference implementation's
/// mode filter reads as "any mode".
pub fn mode_id(mode: Mode) -> i32 {
    MODE_IDS.iter().find(|(m, _)| *m == mode).map(|(_, id)| *id).unwrap_or(0)
}

/// The play mode a stored id names, or `None` for an id no mode of this build claims.
pub fn mode_from_id(id: i32) -> Option<Mode> {
    MODE_IDS.iter().find(|(_, stored)| *stored == id).map(|(m, _)| *m)
}

/// One chart as the database holds it: everything the browser lists a song by, plus the stamp
/// ([`SongRow::date`], [`SongRow::size`]) an incremental scan compares against the file on disk.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SongRow {
    /// Absolute path with `/` separators, which is the primary key.
    pub path: String,
    pub md5: String,
    pub sha256: String,
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub subartist: String,
    pub genre: String,
    pub maker: String,
    /// `#PLAYLEVEL` as authored. The numeric `level` column is derived from it on write.
    pub level: String,
    pub difficulty: i32,
    /// [`mode_id`] of the mode the chart was detected as.
    pub mode: i32,
    /// `#RANK`.
    pub judge: i32,
    pub total: f64,
    pub init_bpm: f64,
    pub min_bpm: i32,
    pub max_bpm: i32,
    pub length_ms: i64,
    pub notes: i32,
    pub long_notes: i32,
    pub stagefile: String,
    pub banner: String,
    pub backbmp: String,
    pub preview: String,
    /// Directory holding the chart, normalised like [`SongRow::path`].
    pub folder: String,
    /// User data: never overwritten by a rescan.
    pub favorite: i32,
    /// File modification time in whole seconds, half of the incremental-scan stamp.
    pub date: i64,
    /// When the chart first entered the database. Kept across rescans.
    pub adddate: i64,
    /// File size in bytes, the other half of the incremental-scan stamp.
    pub size: i64,
    /// `FEATURE_*` bits.
    pub feature: i32,
    /// `CONTENT_*` bits.
    pub content: i32,
}

/// The part of a chart's detail panel that only a full timing integration can answer, cached so the
/// browser does not re-integrate the chart every time the focus lands on it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DetailRow {
    pub duration_us: i64,
    pub peak_density: f64,
    pub avg_density: f64,
    pub end_density: f64,
    /// Notes per second, one bin per second of chart.
    pub density: Vec<u32>,
}

/// Columns [`song_from_row`] reads, in its own order.
pub(crate) const SONG_COLUMNS: &str = "path, md5, sha256, title, subtitle, artist, subartist, genre, rbms_maker, rbms_level_text, difficulty, mode, \
     judge, rbms_total, rbms_init_bpm, minbpm, maxbpm, length, notes, rbms_long_notes, stagefile, banner, backbmp, preview, folder, favorite, date, \
     adddate, rbms_size, feature, content";

/// Insert or refresh one chart. `favorite` and `adddate` are user and history data, so a rescan
/// leaves whatever is already stored for them alone.
pub(crate) const UPSERT_SONG: &str = "INSERT INTO song (path, md5, sha256, title, subtitle, artist, subartist, genre, rbms_maker, level, \
     rbms_level_text, difficulty, mode, judge, rbms_total, rbms_init_bpm, minbpm, maxbpm, length, notes, rbms_long_notes, stagefile, banner, backbmp, \
     preview, folder, favorite, date, adddate, rbms_size, feature, content, rbms_scanned_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, \
     ?30, ?31, ?32, ?33) \
     ON CONFLICT(path) DO UPDATE SET md5 = excluded.md5, sha256 = excluded.sha256, title = excluded.title, subtitle = excluded.subtitle, \
     artist = excluded.artist, subartist = excluded.subartist, genre = excluded.genre, rbms_maker = excluded.rbms_maker, level = excluded.level, \
     rbms_level_text = excluded.rbms_level_text, difficulty = excluded.difficulty, mode = excluded.mode, judge = excluded.judge, \
     rbms_total = excluded.rbms_total, rbms_init_bpm = excluded.rbms_init_bpm, minbpm = excluded.minbpm, maxbpm = excluded.maxbpm, \
     length = excluded.length, notes = excluded.notes, rbms_long_notes = excluded.rbms_long_notes, stagefile = excluded.stagefile, \
     banner = excluded.banner, backbmp = excluded.backbmp, preview = excluded.preview, folder = excluded.folder, date = excluded.date, \
     rbms_size = excluded.rbms_size, feature = excluded.feature, content = excluded.content, rbms_scanned_at = excluded.rbms_scanned_at";

/// Insert or replace one chart's cached detail.
pub(crate) const UPSERT_DETAIL: &str = "INSERT INTO song_detail (path, duration_us, peak_density, avg_density, end_density, density_bins) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
     ON CONFLICT(path) DO UPDATE SET duration_us = excluded.duration_us, peak_density = excluded.peak_density, avg_density = excluded.avg_density, \
     end_density = excluded.end_density, density_bins = excluded.density_bins";

/// Read one [`SongRow`] out of a row selected with [`SONG_COLUMNS`].
pub(crate) fn song_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SongRow> {
    Ok(SongRow {
        path: row.get(0)?,
        md5: row.get(1)?,
        sha256: row.get(2)?,
        title: row.get(3)?,
        subtitle: row.get(4)?,
        artist: row.get(5)?,
        subartist: row.get(6)?,
        genre: row.get(7)?,
        maker: row.get(8)?,
        level: row.get(9)?,
        difficulty: row.get(10)?,
        mode: row.get(11)?,
        judge: row.get(12)?,
        total: row.get(13)?,
        init_bpm: row.get(14)?,
        min_bpm: row.get(15)?,
        max_bpm: row.get(16)?,
        length_ms: row.get(17)?,
        notes: row.get(18)?,
        long_notes: row.get(19)?,
        stagefile: row.get(20)?,
        banner: row.get(21)?,
        backbmp: row.get(22)?,
        preview: row.get(23)?,
        folder: row.get(24)?,
        favorite: row.get(25)?,
        date: row.get(26)?,
        adddate: row.get(27)?,
        size: row.get(28)?,
        feature: row.get(29)?,
        content: row.get(30)?,
    })
}

/// The values [`UPSERT_SONG`] binds, in statement order. The derived numeric level and the scan
/// timestamp are passed in rather than stored on the row, so binding borrows and never allocates.
pub(crate) fn upsert_params<'a>(row: &'a SongRow, level_num: &'a i32, scanned_at: &'a i64) -> [&'a dyn rusqlite::ToSql; UPSERT_PARAM_COUNT] {
    [
        &row.path,
        &row.md5,
        &row.sha256,
        &row.title,
        &row.subtitle,
        &row.artist,
        &row.subartist,
        &row.genre,
        &row.maker,
        level_num,
        &row.level,
        &row.difficulty,
        &row.mode,
        &row.judge,
        &row.total,
        &row.init_bpm,
        &row.min_bpm,
        &row.max_bpm,
        &row.length_ms,
        &row.notes,
        &row.long_notes,
        &row.stagefile,
        &row.banner,
        &row.backbmp,
        &row.preview,
        &row.folder,
        &row.favorite,
        &row.date,
        &row.adddate,
        &row.size,
        &row.feature,
        &row.content,
        scanned_at,
    ]
}

/// How many values [`UPSERT_SONG`] binds.
pub(crate) const UPSERT_PARAM_COUNT: usize = 33;

/// `#PLAYLEVEL` as a number for the numeric `level` column, mirroring the reference implementation's
/// `Integer.parseInt` with its failure left at zero (`SongData.java:167-171`). Charts that decorate
/// the level (`★12`) keep the authored text in `rbms_level_text` and sort as unlevelled here.
pub(crate) fn numeric_level(level: &str) -> i32 {
    level.trim().parse::<i32>().unwrap_or(0)
}

/// Pack per-second density bins for the `density_bins` blob: little-endian `u32` each.
pub(crate) fn pack_density(bins: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bins.len() * DENSITY_BIN_BYTES);
    for bin in bins {
        out.extend_from_slice(&bin.to_le_bytes());
    }
    out
}

/// Unpack a `density_bins` blob. A trailing partial bin is dropped rather than read past.
pub(crate) fn unpack_density(blob: &[u8]) -> Vec<u32> {
    blob.chunks_exact(DENSITY_BIN_BYTES).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

/// Bytes one packed density bin takes.
const DENSITY_BIN_BYTES: usize = 4;
