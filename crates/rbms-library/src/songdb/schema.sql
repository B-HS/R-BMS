CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS folder (
    path     TEXT PRIMARY KEY,
    title    TEXT NOT NULL DEFAULT '',
    subtitle TEXT NOT NULL DEFAULT '',
    command  TEXT NOT NULL DEFAULT '',
    banner   TEXT NOT NULL DEFAULT '',
    parent   TEXT NOT NULL DEFAULT '',
    type     INTEGER NOT NULL DEFAULT 0,
    date     INTEGER NOT NULL DEFAULT 0,
    adddate  INTEGER NOT NULL DEFAULT 0,
    max      INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS song (
    path       TEXT PRIMARY KEY,
    md5        TEXT NOT NULL DEFAULT '',
    sha256     TEXT NOT NULL DEFAULT '',
    title      TEXT NOT NULL DEFAULT '',
    subtitle   TEXT NOT NULL DEFAULT '',
    genre      TEXT NOT NULL DEFAULT '',
    artist     TEXT NOT NULL DEFAULT '',
    subartist  TEXT NOT NULL DEFAULT '',
    tag        TEXT NOT NULL DEFAULT '',
    folder     TEXT NOT NULL DEFAULT '',
    parent     TEXT NOT NULL DEFAULT '',
    stagefile  TEXT NOT NULL DEFAULT '',
    banner     TEXT NOT NULL DEFAULT '',
    backbmp    TEXT NOT NULL DEFAULT '',
    preview    TEXT NOT NULL DEFAULT '',
    level      INTEGER NOT NULL DEFAULT 0,
    difficulty INTEGER NOT NULL DEFAULT 0,
    maxbpm     INTEGER NOT NULL DEFAULT 0,
    minbpm     INTEGER NOT NULL DEFAULT 0,
    length     INTEGER NOT NULL DEFAULT 0,
    mode       INTEGER NOT NULL DEFAULT 0,
    judge      INTEGER NOT NULL DEFAULT 0,
    feature    INTEGER NOT NULL DEFAULT 0,
    content    INTEGER NOT NULL DEFAULT 0,
    date       INTEGER NOT NULL DEFAULT 0,
    favorite   INTEGER NOT NULL DEFAULT 0,
    adddate    INTEGER NOT NULL DEFAULT 0,
    notes      INTEGER NOT NULL DEFAULT 0,
    charthash  TEXT NOT NULL DEFAULT '',
    rbms_size       INTEGER NOT NULL DEFAULT 0,
    rbms_total      REAL    NOT NULL DEFAULT 0,
    rbms_init_bpm   REAL    NOT NULL DEFAULT 0,
    rbms_long_notes INTEGER NOT NULL DEFAULT 0,
    rbms_maker      TEXT    NOT NULL DEFAULT '',
    rbms_level_text TEXT    NOT NULL DEFAULT '',
    rbms_scanned_at INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_song_md5    ON song(md5);
CREATE INDEX IF NOT EXISTS idx_song_sha256 ON song(sha256);
CREATE INDEX IF NOT EXISTS idx_song_folder ON song(folder);
CREATE INDEX IF NOT EXISTS idx_song_title  ON song(title COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_song_level  ON song(mode, level);

CREATE TABLE IF NOT EXISTS song_detail (
    path         TEXT PRIMARY KEY REFERENCES song(path) ON DELETE CASCADE,
    duration_us  INTEGER NOT NULL,
    peak_density REAL NOT NULL,
    avg_density  REAL NOT NULL,
    end_density  REAL NOT NULL,
    density_bins BLOB NOT NULL
);
