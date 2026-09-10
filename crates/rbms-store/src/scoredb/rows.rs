//! Binding one row of the score database and reading it back: the column lists the statements
//! agree on, the inserts, and the day accumulation.
//!
//! Kept apart from the accessors so that the shape of a row and the questions asked of it can be
//! read without each other. Nothing here decides anything — the merge rules live beside
//! `record_play`, and this module only moves values between a struct and a statement.

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{BestScore, CLEAR_FAILED_ID, JUDGE_TIER_COUNT, JudgeCounts, PlayLog, ScoreDbError, utc_day_start};

/// The `e*`/`l*` column pair names in schema order, so a bind list and a read never drift apart.
pub(super) const JUDGE_COLUMNS: [&str; JUDGE_TIER_COUNT * 2] = ["epg", "lpg", "egr", "lgr", "egd", "lgd", "ebd", "lbd", "epr", "lpr", "ems", "lms"];

/// Columns of one `scorelog` row, `id` first.
pub(super) const LOG_COLUMNS: &str = "id, chart_key, mode, ln_mode, md5, sha256, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, \
     notes, combo, minbp, avgjudge, gauge, gauge_value, assist, option, seed, random, playtime, date, state, rule_version, ir_submitted, \
     replay_file, title";

/// Columns of one `score` row.
pub(super) const BEST_COLUMNS: &str = "chart_key, mode, ln_mode, md5, sha256, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, \
     notes, combo, minbp, avgjudge, playcount, clearcount, option, seed, random, date, state, gauge, assist, rule_version, replay_file";

/// Append one play to `scorelog`, returning its id.
pub(super) fn insert_log(conn: &Connection, log: &PlayLog) -> Result<i64, ScoreDbError> {
    let judge = log.judge.columns();
    let sql = format!(
        "INSERT INTO scorelog(chart_key, mode, ln_mode, md5, sha256, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, \
         notes, combo, minbp, avgjudge, gauge, gauge_value, assist, option, seed, random, playtime, date, state, rule_version, ir_submitted, \
         replay_file, title) VALUES({})",
        placeholders(35)
    );
    conn.execute(
        &sql,
        params![
            log.chart_key,
            log.mode,
            log.ln_mode,
            log.md5,
            log.sha256,
            log.clear,
            judge[0],
            judge[1],
            judge[2],
            judge[3],
            judge[4],
            judge[5],
            judge[6],
            judge[7],
            judge[8],
            judge[9],
            judge[10],
            judge[11],
            log.notes,
            log.combo,
            log.minbp,
            log.avgjudge,
            log.gauge,
            log.gauge_value,
            log.assist,
            log.option,
            log.seed,
            log.random,
            log.playtime_ms,
            log.date,
            log.state,
            log.rule_version,
            log.ir_submitted,
            log.replay_file,
            log.title,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// One `scorelog` row as the play it recorded.
pub(super) fn log_from_row(row: &Row<'_>) -> rusqlite::Result<PlayLog> {
    Ok(PlayLog {
        chart_key: row.get("chart_key")?,
        mode: row.get("mode")?,
        ln_mode: row.get("ln_mode")?,
        md5: row.get("md5")?,
        sha256: row.get("sha256")?,
        title: row.get("title")?,
        clear: row.get("clear")?,
        judge: JudgeCounts::from_row(row)?,
        notes: row.get("notes")?,
        combo: row.get("combo")?,
        minbp: row.get("minbp")?,
        avgjudge: row.get("avgjudge")?,
        gauge: row.get("gauge")?,
        gauge_value: row.get("gauge_value")?,
        assist: row.get("assist")?,
        option: row.get("option")?,
        seed: row.get("seed")?,
        random: row.get("random")?,
        playtime_ms: row.get("playtime")?,
        date: row.get("date")?,
        state: row.get("state")?,
        rule_version: row.get("rule_version")?,
        ir_submitted: row.get("ir_submitted")?,
        replay_file: row.get("replay_file")?,
    })
}

/// The stored bests of one chart in one play mode and long-note flavour.
pub(super) fn read_best(conn: &Connection, chart_key: &str, mode: i32, ln_mode: &str) -> Result<Option<BestScore>, ScoreDbError> {
    let sql = format!("SELECT {BEST_COLUMNS} FROM score WHERE chart_key = ?1 AND mode = ?2 AND ln_mode = ?3");
    Ok(conn.query_row(&sql, params![chart_key, mode, ln_mode], BestScore::from_row).optional()?)
}

/// Store a chart's merged bests, replacing whatever stood there.
pub(super) fn write_best(conn: &Connection, best: &BestScore) -> Result<(), ScoreDbError> {
    let judge = best.judge.columns();
    let sql = format!(
        "INSERT OR REPLACE INTO score(chart_key, mode, ln_mode, md5, sha256, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, \
         notes, combo, minbp, avgjudge, playcount, clearcount, option, seed, random, date, state, gauge, assist, rule_version, replay_file) \
         VALUES({})",
        placeholders(33)
    );
    conn.execute(
        &sql,
        params![
            best.chart_key,
            best.mode,
            best.ln_mode,
            best.md5,
            best.sha256,
            best.clear,
            judge[0],
            judge[1],
            judge[2],
            judge[3],
            judge[4],
            judge[5],
            judge[6],
            judge[7],
            judge[8],
            judge[9],
            judge[10],
            judge[11],
            best.notes,
            best.combo,
            best.minbp,
            best.avgjudge,
            best.playcount,
            best.clearcount,
            best.option,
            best.seed,
            best.random,
            best.date,
            best.state,
            best.gauge,
            best.assist,
            best.rule_version,
            best.replay_file,
        ],
    )?;
    Ok(())
}

/// Accumulate one play into its day (`PlayDataAccessor.java:115-136`). Every play counts here,
/// assisted or not: the daily totals are what the player did, not what they scored.
pub(super) fn accumulate_day(conn: &Connection, log: &PlayLog) -> Result<(), ScoreDbError> {
    let judge = log.judge.columns();
    let cleared = u32::from(log.clear > CLEAR_FAILED_ID);
    let judge_columns = JUDGE_COLUMNS.join(", ");
    let judge_sums = JUDGE_COLUMNS.iter().map(|c| format!("{c} = {c} + excluded.{c}")).collect::<Vec<_>>().join(", ");
    let sql = format!(
        "INSERT INTO player_stat(date, playcount, clear, {judge_columns}, playtime, maxcombo) VALUES({}) \
         ON CONFLICT(date) DO UPDATE SET playcount = playcount + 1, clear = clear + excluded.clear, {judge_sums}, \
         playtime = playtime + excluded.playtime, maxcombo = MAX(maxcombo, excluded.maxcombo)",
        placeholders(17)
    );
    conn.execute(
        &sql,
        params![
            utc_day_start(log.date),
            1,
            cleared,
            judge[0],
            judge[1],
            judge[2],
            judge[3],
            judge[4],
            judge[5],
            judge[6],
            judge[7],
            judge[8],
            judge[9],
            judge[10],
            judge[11],
            log.playtime_ms,
            log.combo,
        ],
    )?;
    Ok(())
}

/// `?1, ?2, ...` for `count` bound values.
fn placeholders(count: usize) -> String {
    (1..=count).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ")
}
