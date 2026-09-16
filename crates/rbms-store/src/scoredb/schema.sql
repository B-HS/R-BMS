CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS score (
    chart_key    TEXT NOT NULL,
    mode         INTEGER NOT NULL,
    ln_mode      TEXT NOT NULL DEFAULT 'CHART',
    md5          TEXT NOT NULL DEFAULT '',
    sha256       TEXT NOT NULL DEFAULT '',
    clear        INTEGER NOT NULL DEFAULT 0,
    epg          INTEGER NOT NULL DEFAULT 0,
    lpg          INTEGER NOT NULL DEFAULT 0,
    egr          INTEGER NOT NULL DEFAULT 0,
    lgr          INTEGER NOT NULL DEFAULT 0,
    egd          INTEGER NOT NULL DEFAULT 0,
    lgd          INTEGER NOT NULL DEFAULT 0,
    ebd          INTEGER NOT NULL DEFAULT 0,
    lbd          INTEGER NOT NULL DEFAULT 0,
    epr          INTEGER NOT NULL DEFAULT 0,
    lpr          INTEGER NOT NULL DEFAULT 0,
    ems          INTEGER NOT NULL DEFAULT 0,
    lms          INTEGER NOT NULL DEFAULT 0,
    notes        INTEGER NOT NULL DEFAULT 0,
    combo        INTEGER NOT NULL DEFAULT 0,
    minbp        INTEGER NOT NULL DEFAULT 2147483647,
    avgjudge     INTEGER NOT NULL DEFAULT 2147483647,
    playcount    INTEGER NOT NULL DEFAULT 0,
    clearcount   INTEGER NOT NULL DEFAULT 0,
    option       INTEGER NOT NULL DEFAULT 0,
    seed         INTEGER NOT NULL DEFAULT -1,
    random       INTEGER NOT NULL DEFAULT 0,
    date         INTEGER NOT NULL DEFAULT 0,
    state        INTEGER NOT NULL DEFAULT 0,
    gauge        INTEGER NOT NULL DEFAULT 0,
    assist       INTEGER NOT NULL DEFAULT 0,
    rule_version INTEGER NOT NULL DEFAULT 0,
    replay_file  TEXT,
    PRIMARY KEY (chart_key, mode, ln_mode)
);

CREATE TABLE IF NOT EXISTS scorelog (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    chart_key    TEXT NOT NULL,
    mode         INTEGER NOT NULL,
    ln_mode      TEXT NOT NULL DEFAULT 'CHART',
    md5          TEXT NOT NULL DEFAULT '',
    sha256       TEXT NOT NULL DEFAULT '',
    clear        INTEGER NOT NULL DEFAULT 0,
    epg          INTEGER NOT NULL DEFAULT 0,
    lpg          INTEGER NOT NULL DEFAULT 0,
    egr          INTEGER NOT NULL DEFAULT 0,
    lgr          INTEGER NOT NULL DEFAULT 0,
    egd          INTEGER NOT NULL DEFAULT 0,
    lgd          INTEGER NOT NULL DEFAULT 0,
    ebd          INTEGER NOT NULL DEFAULT 0,
    lbd          INTEGER NOT NULL DEFAULT 0,
    epr          INTEGER NOT NULL DEFAULT 0,
    lpr          INTEGER NOT NULL DEFAULT 0,
    ems          INTEGER NOT NULL DEFAULT 0,
    lms          INTEGER NOT NULL DEFAULT 0,
    notes        INTEGER NOT NULL DEFAULT 0,
    combo        INTEGER NOT NULL DEFAULT 0,
    minbp        INTEGER NOT NULL DEFAULT 0,
    avgjudge     INTEGER NOT NULL DEFAULT 2147483647,
    gauge        INTEGER NOT NULL DEFAULT 0,
    gauge_value  REAL NOT NULL DEFAULT 0,
    assist       INTEGER NOT NULL DEFAULT 0,
    option       INTEGER NOT NULL DEFAULT 0,
    seed         INTEGER NOT NULL DEFAULT -1,
    random       INTEGER NOT NULL DEFAULT 0,
    playtime     INTEGER NOT NULL DEFAULT 0,
    date         INTEGER NOT NULL,
    state        INTEGER NOT NULL DEFAULT 0,
    rule_version INTEGER NOT NULL DEFAULT 0,
    ir_submitted INTEGER NOT NULL DEFAULT 0,
    replay_file  TEXT,
    title        TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_log_chart  ON scorelog(chart_key, date DESC);
CREATE INDEX IF NOT EXISTS idx_log_date   ON scorelog(date DESC);
CREATE INDEX IF NOT EXISTS idx_log_replay ON scorelog(replay_file);

CREATE TABLE IF NOT EXISTS player_stat (
    date      INTEGER PRIMARY KEY,
    playcount INTEGER NOT NULL DEFAULT 0,
    clear     INTEGER NOT NULL DEFAULT 0,
    epg       INTEGER NOT NULL DEFAULT 0,
    lpg       INTEGER NOT NULL DEFAULT 0,
    egr       INTEGER NOT NULL DEFAULT 0,
    lgr       INTEGER NOT NULL DEFAULT 0,
    egd       INTEGER NOT NULL DEFAULT 0,
    lgd       INTEGER NOT NULL DEFAULT 0,
    ebd       INTEGER NOT NULL DEFAULT 0,
    lbd       INTEGER NOT NULL DEFAULT 0,
    epr       INTEGER NOT NULL DEFAULT 0,
    lpr       INTEGER NOT NULL DEFAULT 0,
    ems       INTEGER NOT NULL DEFAULT 0,
    lms       INTEGER NOT NULL DEFAULT 0,
    playtime  INTEGER NOT NULL DEFAULT 0,
    maxcombo  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS profile (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    rank TEXT NOT NULL DEFAULT ''
);
