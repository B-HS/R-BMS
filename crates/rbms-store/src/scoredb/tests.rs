use std::path::PathBuf;

use super::*;
use crate::{SCORE_LN_MODE_FROM_CHART, SCORE_RULE_VERSION, ScoreBook, ScoreRecord};

/// A play good enough to be a best: perfect on every note, no breaks.
fn play(chart_key: &str, date: i64, pgreat: u32) -> PlayLog {
    PlayLog {
        chart_key: chart_key.into(),
        mode: mode_id("BEAT_7K"),
        ln_mode: SCORE_LN_MODE_FROM_CHART.into(),
        md5: chart_key.into(),
        sha256: String::new(),
        title: "Chart".into(),
        clear: LAMP_NORMAL,
        judge: JudgeCounts::from_early_late([pgreat, 0, 0, 0, 0, 0], [0; JUDGE_TIER_COUNT]),
        notes: pgreat,
        combo: pgreat,
        minbp: 0,
        avgjudge: 5_000,
        gauge: gauge_id("normal"),
        gauge_value: 80.0,
        assist: 0,
        option: 0,
        seed: 7,
        random: random_id("OFF"),
        playtime_ms: 120_000,
        date,
        state: PLAY_STATE_LIVE,
        rule_version: SCORE_RULE_VERSION,
        ir_submitted: false,
        replay_file: None,
    }
}

/// `ClearType::Normal`, the lamp the fixtures clear with.
const LAMP_NORMAL: u8 = 5;
/// `ClearType::Hard`, one lamp above [`LAMP_NORMAL`].
const LAMP_HARD: u8 = 6;
/// `ClearType::AssistEasy`, the lamp an assisted run is demoted to.
const LAMP_ASSIST_EASY: u8 = 2;
/// `ClearType::Failed`.
const LAMP_FAILED: u8 = 1;

fn fresh() -> ScoreDb {
    let mut db = ScoreDb::open_in_memory().expect("an in-memory database opens");
    db.migrate().expect("the schema applies");
    db
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms_scoredb_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temp dir is creatable");
    dir
}

fn table_columns(db: &ScoreDb, table: &str) -> Vec<String> {
    let mut stmt = db.conn.prepare(&format!("PRAGMA table_info({table})")).expect("table_info prepares");
    let rows = stmt.query_map([], |row| row.get::<_, String>("name")).expect("table_info runs");
    rows.map(|r| r.expect("a column name")).collect()
}

#[test]
fn migrate_creates_every_table_the_spec_names() {
    let db = fresh();
    let mut stmt = db.conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").expect("prepares");
    let tables: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(0)).expect("runs").map(|r| r.expect("a name")).collect();
    for want in ["meta", "player_stat", "profile", "score", "scorelog"] {
        assert!(tables.iter().any(|t| t == want), "{want} exists, got {tables:?}");
    }
}

#[test]
fn the_score_table_carries_every_column_the_spec_lists() {
    let db = fresh();
    let columns = table_columns(&db, "score");
    let want = [
        "chart_key",
        "mode",
        "ln_mode",
        "md5",
        "sha256",
        "clear",
        "epg",
        "lpg",
        "egr",
        "lgr",
        "egd",
        "lgd",
        "ebd",
        "lbd",
        "epr",
        "lpr",
        "ems",
        "lms",
        "notes",
        "combo",
        "minbp",
        "avgjudge",
        "playcount",
        "clearcount",
        "option",
        "seed",
        "random",
        "date",
        "state",
        "gauge",
        "assist",
        "rule_version",
        "replay_file",
    ];
    assert_eq!(columns, want, "the score row is exactly the columns the DDL declares");
}

#[test]
fn the_scorelog_and_player_stat_tables_carry_their_columns() {
    let db = fresh();
    let log = table_columns(&db, "scorelog");
    for want in ["id", "chart_key", "mode", "ln_mode", "gauge_value", "playtime", "ir_submitted", "replay_file", "title"] {
        assert!(log.iter().any(|c| c == want), "scorelog has {want}, got {log:?}");
    }
    let stat = table_columns(&db, "player_stat");
    assert_eq!(stat.first().map(String::as_str), Some("date"), "the day is the key");
    for want in ["playcount", "clear", "playtime", "maxcombo"] {
        assert!(stat.iter().any(|c| c == want), "player_stat has {want}, got {stat:?}");
    }
    for column in JUDGE_COLUMNS {
        assert!(stat.iter().any(|c| c == column), "player_stat accumulates {column}");
    }
}

#[test]
fn migrate_stamps_the_schema_version_and_is_idempotent() {
    let mut db = fresh();
    assert_eq!(db.migrate().expect("a second migrate is a no-op"), SCORE_DB_SCHEMA_VERSION);
    let stored: String = db.conn.query_row("SELECT value FROM meta WHERE key = 'schema_version'", [], |row| row.get(0)).expect("the stamp is there");
    assert_eq!(stored, SCORE_DB_SCHEMA_VERSION.to_string());
}

#[test]
fn a_database_from_a_newer_build_is_refused_rather_than_rewritten() {
    let mut db = fresh();
    let future = SCORE_DB_SCHEMA_VERSION + 1;
    db.conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES('schema_version', ?1)", [future.to_string()]).expect("stamped");
    match db.migrate() {
        Err(ScoreDbError::FutureSchema { found, supported }) => {
            assert_eq!(found, future);
            assert_eq!(supported, SCORE_DB_SCHEMA_VERSION);
        }
        other => panic!("a newer schema is refused, got {other:?}"),
    }
}

#[test]
fn record_play_writes_the_log_the_best_and_the_day() {
    let mut db = fresh();
    let id = db.record_play(&play("aa", 1_700_000_000, 100), true).expect("the play records");
    assert!(id > 0, "the log row has an id");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("a best exists");
    assert_eq!(best.ex_score(), 200, "a hundred PGREATs are two hundred EX");
    assert_eq!(best.playcount, 1);
    assert_eq!(best.clearcount, 1, "a normal clear counts as a clear");
    assert_eq!(db.play_count().expect("counted"), 1);
    let days = db.day_stats(0, i64::MAX).expect("day stats");
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].playcount, 1);
    assert_eq!(days[0].judge.tier(JUDGE_PGREAT), 100);
    assert_eq!(days[0].playtime_ms, 120_000);
}

#[test]
fn a_better_run_replaces_the_best_and_a_worse_one_does_not() {
    let mut db = fresh();
    db.record_play(&play("aa", 10, 100), true).expect("first");
    db.record_play(&play("aa", 20, 50), true).expect("worse");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.ex_score(), 200, "the worse run leaves the EX alone");
    assert_eq!(best.playcount, 2, "but it still counts as a play");
    db.record_play(&play("aa", 30, 150), true).expect("better");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.ex_score(), 300, "the better run takes the EX");
    assert_eq!(best.playcount, 3);
}

#[test]
fn an_assisted_run_raises_the_lamp_and_the_play_count_but_never_the_score() {
    let mut db = fresh();
    db.record_play(&play("aa", 10, 100), true).expect("a clean run");
    let mut assisted = play("aa", 20, 400);
    assisted.clear = LAMP_HARD;
    assisted.combo = 400;
    assisted.minbp = 0;
    assisted.avgjudge = 1;
    db.record_play(&assisted, false).expect("an assisted run");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.ex_score(), 200, "decision 12: the assisted EX is not a best");
    assert_eq!(best.combo, 100, "nor its combo");
    assert_eq!(best.avgjudge, 5_000, "nor its timing");
    assert_eq!(best.clear, LAMP_HARD, "but the demoted lamp still stands");
    assert_eq!(best.playcount, 2, "and the run counted as a play");
    assert_eq!(best.clearcount, 2, "and as a clear");
}

#[test]
fn an_assisted_run_from_an_older_rule_generation_cannot_raise_the_lamp() {
    let mut db = fresh();
    db.record_play(&play("aa", 10, 100), true).expect("a clean run");
    let mut stale = play("aa", 20, 10);
    stale.clear = LAMP_HARD;
    stale.rule_version = 0;
    db.record_play(&stale, false).expect("an old assisted run");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.clear, LAMP_NORMAL, "nothing demoted that lamp, so it may not raise this one");
    assert_eq!(best.playcount, 2, "the play still counted");
}

#[test]
fn the_day_bucket_accumulates_every_play_of_that_day_and_splits_on_the_next() {
    let mut db = fresh();
    let day = 1_700_000_000;
    db.record_play(&play("aa", day, 10), true).expect("one");
    db.record_play(&play("bb", day + 60, 20), true).expect("two");
    let mut failed = play("cc", day + 86_400, 5);
    failed.clear = LAMP_FAILED;
    db.record_play(&failed, true).expect("three");
    let days = db.day_stats(0, i64::MAX).expect("day stats");
    assert_eq!(days.len(), 2, "two calendar days");
    assert_eq!(days[0].date, utc_day_start(day));
    assert_eq!(days[0].playcount, 2);
    assert_eq!(days[0].clear, 2);
    assert_eq!(days[0].judge.tier(JUDGE_PGREAT), 30, "both runs' judgements accumulate");
    assert_eq!(days[0].maxcombo, 20, "the day keeps the largest combo");
    assert_eq!(days[1].playcount, 1);
    assert_eq!(days[1].clear, 0, "a failed run is a play but not a clear");
}

#[test]
fn utc_day_start_floors_before_and_after_the_epoch() {
    assert_eq!(utc_day_start(0), 0);
    assert_eq!(utc_day_start(86_399), 0);
    assert_eq!(utc_day_start(86_400), 86_400);
    assert_eq!(utc_day_start(-1), -86_400, "a stamp before the epoch belongs to the day before it");
}

#[test]
fn the_long_note_flavour_splits_the_bests_of_one_chart() {
    let mut db = fresh();
    let mut charge = play("aa", 20, 400);
    charge.ln_mode = "CN".into();
    db.record_play(&play("aa", 10, 100), true).expect("plain");
    db.record_play(&charge, true).expect("charge");
    let plain = db.best_in_ln_mode("aa", mode_id("BEAT_7K"), SCORE_LN_MODE_FROM_CHART).expect("queried").expect("exists");
    let cn = db.best_in_ln_mode("aa", mode_id("BEAT_7K"), "CN").expect("queried").expect("exists");
    assert_eq!(plain.ex_score(), 200, "the charge-note run has its own EX ceiling and does not touch this one");
    assert_eq!(cn.ex_score(), 800);
    let folded = db.chart_best("aa").expect("queried").expect("exists");
    assert_eq!(folded.ex_score, 800, "the select list sees the chart's strongest run whatever it was played as");
    assert_eq!(folded.playcount, 2);
}

#[test]
fn best_picks_the_highest_lamp_and_breaks_a_tie_on_score() {
    let mut db = fresh();
    let mut weak_but_cleared = play("aa", 10, 50);
    weak_but_cleared.ln_mode = "CN".into();
    weak_but_cleared.clear = LAMP_HARD;
    db.record_play(&play("aa", 20, 400), true).expect("high score, lower lamp");
    db.record_play(&weak_but_cleared, true).expect("high lamp, lower score");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.clear, LAMP_HARD, "the lamp decides first");
    assert_eq!(best.ln_mode, "CN");
}

#[test]
fn chart_best_many_answers_a_screenful_in_one_query_and_omits_unplayed_charts() {
    let mut db = fresh();
    db.record_play(&play("aa", 10, 100), true).expect("one");
    db.record_play(&play("bb", 20, 300), true).expect("two");
    let keys = vec!["aa".to_string(), "bb".to_string(), "cc".to_string()];
    let found = db.chart_best_many(&keys).expect("queried");
    assert_eq!(found.len(), 2, "a chart with no plays has no row");
    assert_eq!(found["aa"].ex_score, 200);
    assert_eq!(found["bb"].ex_score, 600);
    assert!(db.chart_best_many(&[]).expect("queried").is_empty(), "no keys, no query");
}

#[test]
fn history_is_newest_first_and_round_trips_every_field() {
    let mut db = fresh();
    let mut first = play("aa", 10, 100);
    first.replay_file = Some("aa-10.ron".into());
    first.ir_submitted = true;
    first.judge = JudgeCounts::from_early_late([1, 2, 3, 4, 5, 6], [7, 8, 9, 10, 11, 12]);
    db.record_play(&first, true).expect("one");
    db.record_play(&play("aa", 20, 50), true).expect("two");
    let history = db.history("aa", 10).expect("queried");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].date, 20, "newest first");
    assert_eq!(history[1], first, "every field survives the round trip");
    assert_eq!(db.history("aa", 1).expect("queried").len(), 1, "the limit is honoured");
}

#[test]
fn all_plays_is_oldest_first() {
    let mut db = fresh();
    db.record_play(&play("bb", 30, 1), true).expect("one");
    db.record_play(&play("aa", 10, 1), true).expect("two");
    let plays = db.all_plays().expect("queried");
    assert_eq!(plays.iter().map(|p| p.date).collect::<Vec<_>>(), vec![10, 30]);
}

#[test]
fn a_chart_with_no_plays_has_no_best() {
    let db = fresh();
    assert!(db.best("nothing", mode_id("BEAT_7K")).expect("queried").is_none());
    assert!(db.chart_best("nothing").expect("queried").is_none());
    assert!(db.history("nothing", 10).expect("queried").is_empty());
    assert!(db.day_stats(0, i64::MAX).expect("queried").is_empty());
}

#[test]
fn the_profile_row_is_replaced_not_appended() {
    let mut db = fresh();
    assert!(db.profile().expect("queried").is_none());
    db.set_profile(&Profile { id: "one".into(), name: "First".into(), rank: "A".into() }).expect("written");
    db.set_profile(&Profile { id: "two".into(), name: "Second".into(), rank: "B".into() }).expect("replaced");
    let profile = db.profile().expect("queried").expect("exists");
    assert_eq!(profile.id, "two");
    assert_eq!(profile.name, "Second");
    let count: u32 = db.conn.query_row("SELECT COUNT(*) FROM profile", [], |row| row.get(0)).expect("counted");
    assert_eq!(count, 1, "the database holds one identity");
}

#[test]
fn mode_ids_are_the_reference_values_and_round_trip() {
    assert_eq!(mode_id("BEAT_7K"), 7);
    assert_eq!(mode_id("BEAT_5K"), 5);
    assert_eq!(mode_id("BEAT_10K"), 10);
    assert_eq!(mode_id("BEAT_14K"), 14);
    assert_eq!(mode_id("POPN_9K"), 9);
    assert_eq!(mode_id("KEYBOARD_24K"), 25);
    assert_eq!(mode_id("BEAT-7K"), 7, "a record written with the other separator still parses");
    assert_eq!(mode_id("beat_7k"), 7, "and in lower case");
    assert_eq!(mode_id("NOT-A-MODE"), MODE_UNKNOWN);
    for name in ["BEAT_5K", "BEAT_7K", "BEAT_10K", "BEAT_14K", "POPN_9K", "KEYBOARD_24K"] {
        assert_eq!(mode_name(mode_id(name)), name, "{name} round-trips");
    }
    assert_eq!(mode_name(MODE_UNKNOWN), "");
}

#[test]
fn gauge_and_shuffle_tokens_round_trip_through_their_ids() {
    for token in ["assist", "easy", "normal", "hard", "exhard", "hazard", "class", "exclass", "exhardclass"] {
        assert_eq!(gauge_token(gauge_id(token)), token, "{token} round-trips");
    }
    assert_eq!(gauge_id("assisteasy"), gauge_id("assist"), "the alias names the same gauge");
    assert_eq!(gauge_id("HARD"), gauge_id("hard"), "the token is matched case-insensitively");
    assert_eq!(gauge_id("nonsense"), GAUGE_UNKNOWN);
    for label in ["OFF", "MIRROR", "RANDOM", "S-RANDOM", "R-RANDOM", "ROTATE", "H-RANDOM", "ALL-SCRATCH"] {
        assert_eq!(random_token(random_id(label)), label, "{label} round-trips");
    }
    assert_eq!(random_id("S_RANDOM"), random_id("S-RANDOM"), "separators do not change the shuffle");
    assert_eq!(random_id("SPIRAL"), random_id("ROTATE"), "the reference name for the same shuffle");
    assert_eq!(random_id("nonsense"), RANDOM_OFF);
    assert_eq!(random_token(99), "");
}

#[test]
fn ex_score_is_two_per_pgreat_and_one_per_great() {
    let counts = JudgeCounts::from_early_late([3, 5, 1, 2, 0, 4], [4, 5, 0, 0, 1, 0]);
    assert_eq!(counts.ex_score(), (3 + 4) * 2 + (5 + 5));
    assert_eq!(counts.total(), 3 + 5 + 1 + 2 + 4 + 4 + 5 + 1);
    assert_eq!(counts.combo_breaks(), 2 + 1 + 4);
    assert_eq!(JudgeCounts::all_late([1, 2, 3, 4, 5, 6]).ex_score(), 2 + 2, "a run with no split still scores");
}

fn record(md5: &str, played_at_ms: i64, clear: u8, counts: [u32; 6], assisted: bool) -> ScoreRecord {
    ScoreRecord {
        md5: md5.into(),
        title: "Legacy".into(),
        mode: "BEAT-7K".into(),
        clear,
        ex_score: counts[0] * 2 + counts[1],
        max_ex: 1000,
        counts,
        empty_poor: counts[5],
        max_combo: counts[0],
        total_notes: 500,
        gauge: "NORMAL".into(),
        gauge_value: 80.0,
        random: "OFF".into(),
        played_at: played_at_ms,
        replay_file: None,
        rule_version: SCORE_RULE_VERSION,
        ln_mode: SCORE_LN_MODE_FROM_CHART.into(),
        assisted,
    }
}

#[test]
fn migrating_a_score_book_keeps_the_bests_the_book_itself_reports() {
    let mut book = ScoreBook::default();
    book.push(record("AA", 3_000_000, LAMP_NORMAL, [100, 0, 0, 0, 0, 0], false));
    book.push(record("aa", 1_000_000, LAMP_FAILED, [50, 0, 0, 0, 0, 0], false));
    let mut assisted = record("AA", 5_000_000, LAMP_HARD, [400, 0, 0, 0, 0, 0], true);
    assisted.clear = LAMP_ASSIST_EASY;
    book.push(assisted);
    let mut db = fresh();
    let report = migrate_score_book(&mut db, &book).expect("the book migrates");
    assert_eq!(report.records, 3);
    assert_eq!(report.assisted, 1);
    let folded = db.chart_best("aa").expect("queried").expect("exists");
    assert_eq!(u32::from(folded.clear), u32::from(book.best_clear_for_md5("AA").expect("the book has a lamp")));
    assert_eq!(folded.ex_score, book.best_ex_for_md5("AA").expect("the book has an EX"), "the assisted run set no EX in either store");
    assert_eq!(folded.playcount, 3);
}

#[test]
fn migrated_rows_carry_the_marks_that_say_what_they_never_recorded() {
    let mut book = ScoreBook::default();
    book.push(record("AA", 1_700_000_000_000, LAMP_NORMAL, [10, 5, 4, 3, 2, 1], false));
    let mut db = fresh();
    migrate_score_book(&mut db, &book).expect("migrated");
    let history = db.history("aa", 10).expect("queried");
    assert_eq!(history.len(), 1);
    let log = &history[0];
    assert_eq!(log.state, PLAY_STATE_MIGRATED, "the row says where it came from");
    assert_eq!(log.judge.early, [0; JUDGE_TIER_COUNT], "there was no early/late split to carry over");
    assert_eq!(log.judge.late, [10, 5, 4, 3, 2, 1], "so the whole run reads as late");
    assert_eq!(log.avgjudge, UNSET_MINIMUM, "and no mean timing error");
    assert_eq!(log.date, 1_700_000_000, "the millisecond stamp becomes a second");
    assert_eq!(log.chart_key, "aa", "the key is the lower-cased md5");
    assert_eq!(log.minbp, 3 + 2 + 1, "bad, poor and miss");
    assert_eq!(log.mode, 7);
    assert_eq!(log.gauge, gauge_id("normal"), "the gauge token became its id");
}

#[test]
fn an_assisted_legacy_record_is_marked_as_assisted_without_inventing_a_level() {
    let mut book = ScoreBook::default();
    book.push(record("AA", 1_000, LAMP_ASSIST_EASY, [10, 0, 0, 0, 0, 0], true));
    let mut db = fresh();
    migrate_score_book(&mut db, &book).expect("migrated");
    assert_eq!(db.history("aa", 1).expect("queried")[0].assist, ASSIST_UNRECORDED);
}

#[test]
fn an_empty_score_book_migrates_to_an_empty_database() {
    let mut db = fresh();
    let report = migrate_score_book(&mut db, &ScoreBook::default()).expect("migrated");
    assert_eq!(report, MigrationReport::default());
    assert_eq!(db.play_count().expect("counted"), 0);
}

#[test]
fn opening_with_an_import_moves_the_book_aside_and_leaves_the_database_behind() {
    let dir = temp_dir("import");
    let ron = dir.join("scores.ron");
    let db_path = dir.join("scoredb.sqlite");
    let mut book = ScoreBook::default();
    book.push(record("AA", 1_000, LAMP_NORMAL, [100, 0, 0, 0, 0, 0], false));
    book.try_save(&ron).expect("the book is written");

    let (db, report) = open_with_import(&db_path, &ron).expect("the import runs");
    assert_eq!(report.expect("a report").records, 1);
    assert_eq!(db.play_count().expect("counted"), 1);
    assert!(!ron.exists(), "the book is moved aside");
    assert!(dir.join("scores.ron.migrated").exists(), "and kept, not deleted");
    drop(db);

    let (db, report) = open_with_import(&db_path, &ron).expect("a second open");
    assert!(report.is_none(), "an existing database is never re-imported");
    assert_eq!(db.play_count().expect("counted"), 1, "and its rows are not duplicated");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_import_removes_the_half_written_database_and_leaves_the_book_alone() {
    let dir = temp_dir("rollback");
    let ron = dir.join("scores.ron");
    let db_path = dir.join("scoredb.sqlite");
    let mut book = ScoreBook::default();
    book.push(record("AA", 1_000, LAMP_NORMAL, [100, 0, 0, 0, 0, 0], false));
    book.try_save(&ron).expect("the book is written");

    let mut db = ScoreDb::open(&db_path).expect("opened");
    db.migrate().expect("migrated");
    let future = SCORE_DB_SCHEMA_VERSION + 1;
    db.conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES('schema_version', ?1)", [future.to_string()]).expect("stamped");
    drop(db);

    let err = open_with_import(&db_path, &ron).expect_err("a database from the future is refused");
    assert!(matches!(err, ScoreDbError::FutureSchema { .. }), "got {err:?}");
    assert!(ron.exists(), "the book survives so the player can keep reading it");
    assert!(db_path.exists(), "an existing database is refused, not deleted");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn opening_without_a_book_creates_an_empty_database() {
    let dir = temp_dir("empty");
    let db_path = dir.join("scoredb.sqlite");
    let (db, report) = open_with_import(&db_path, &dir.join("nothing.ron")).expect("opened");
    assert!(report.is_none());
    assert_eq!(db.play_count().expect("counted"), 0);
    assert!(db_path.exists());
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Write `count` replays for a chart, newest last, and record a play per replay.
fn seed_replays(db: &mut ScoreDb, dir: &Path, chart: &str, count: usize, bytes: usize) -> Vec<String> {
    let mut names = Vec::new();
    for i in 0..count {
        let name = format!("{chart}-{i}.ron");
        std::fs::write(dir.join(&name), "x".repeat(bytes)).expect("a replay file");
        let mut log = play(chart, 100 + i as i64, 10 + i as u32);
        log.replay_file = Some(name.clone());
        db.record_play(&log, true).expect("recorded");
        names.push(name);
    }
    names
}

#[test]
fn the_retention_plan_keeps_the_recent_replays_and_the_best_one() {
    let dir = temp_dir("gc_recent");
    let mut db = fresh();
    let names = seed_replays(&mut db, &dir, "aa", 5, 10);
    let policy = ReplayPolicy { keep_best_per_chart: true, keep_recent_per_chart: 3, max_total_bytes: 0 };
    let plan = db.replay_gc_plan(&dir, &policy).expect("planned");
    assert_eq!(plan.delete, vec![names[0].clone(), names[1].clone()], "the two oldest go, the three newest stay");
    assert_eq!(plan.kept, 3, "and the best is the newest, which was already kept");
    assert_eq!(plan.freed_bytes, 20);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_retention_plan_spares_an_old_best_the_recent_window_would_have_dropped() {
    let dir = temp_dir("gc_best");
    let mut db = fresh();
    let names = seed_replays(&mut db, &dir, "aa", 5, 10);
    let mut weaker = play("aa", 200, 1);
    weaker.replay_file = Some("aa-late.ron".into());
    std::fs::write(dir.join("aa-late.ron"), "x".repeat(10)).expect("a replay file");
    db.record_play(&weaker, true).expect("recorded");

    let best_file = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists").replay_file.expect("the best has a replay");
    assert_eq!(best_file, names[4], "the strongest run is still the fifth");
    let policy = ReplayPolicy { keep_best_per_chart: true, keep_recent_per_chart: 1, max_total_bytes: 0 };
    let plan = db.replay_gc_plan(&dir, &policy).expect("planned");
    assert!(!plan.delete.contains(&best_file), "the best run's replay is kept however old it is");
    assert_eq!(plan.kept, 2, "the newest and the best");

    let unprotected = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 1, max_total_bytes: 0 };
    let plan = db.replay_gc_plan(&dir, &unprotected).expect("planned");
    assert!(plan.delete.contains(&best_file), "with the best unprotected only the newest survives");
    assert_eq!(plan.kept, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_retention_plan_drops_the_oldest_kept_replays_once_the_directory_is_over_budget() {
    let dir = temp_dir("gc_budget");
    let mut db = fresh();
    let names = seed_replays(&mut db, &dir, "aa", 4, 100);
    let policy = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 4, max_total_bytes: 250 };
    let plan = db.replay_gc_plan(&dir, &policy).expect("planned");
    assert_eq!(plan.kept, 2, "two hundred bytes fit under the ceiling, three hundred do not");
    assert_eq!(plan.delete, vec![names[0].clone(), names[1].clone()], "the oldest go first");
    assert_eq!(plan.freed_bytes, 200);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn each_chart_keeps_its_own_recent_window() {
    let dir = temp_dir("gc_charts");
    let mut db = fresh();
    seed_replays(&mut db, &dir, "aa", 3, 10);
    seed_replays(&mut db, &dir, "bb", 3, 10);
    let policy = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 2, max_total_bytes: 0 };
    let plan = db.replay_gc_plan(&dir, &policy).expect("planned");
    assert_eq!(plan.kept, 4, "two per chart");
    assert_eq!(plan.delete, vec!["aa-0.ron".to_string(), "bb-0.ron".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_database_with_no_replays_plans_nothing() {
    let dir = temp_dir("gc_empty");
    let mut db = fresh();
    db.record_play(&play("aa", 10, 100), true).expect("a play with no replay");
    let plan = db.replay_gc_plan(&dir, &ReplayPolicy::default()).expect("planned");
    assert_eq!(plan, ReplayGcPlan::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clearing_a_replay_reference_leaves_the_history_pointing_at_nothing() {
    let dir = temp_dir("gc_clear");
    let mut db = fresh();
    let names = seed_replays(&mut db, &dir, "aa", 2, 10);
    let cleared = db.clear_replay_refs(&names[..1]).expect("cleared");
    assert_eq!(cleared, 1);
    let history = db.history("aa", 10).expect("queried");
    assert_eq!(history[1].replay_file, None, "the deleted replay is no longer offered");
    assert_eq!(history[0].replay_file, Some(names[1].clone()), "the kept one still is");
    assert_eq!(db.clear_replay_refs(&[]).expect("nothing to clear"), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_default_retention_policy_is_the_one_the_spec_names() {
    let policy = ReplayPolicy::default();
    assert!(policy.keep_best_per_chart);
    assert_eq!(policy.keep_recent_per_chart, DEFAULT_KEEP_RECENT_PER_CHART);
    assert_eq!(policy.keep_recent_per_chart, 3);
    assert_eq!(policy.max_total_bytes, DEFAULT_REPLAY_MAX_TOTAL_BYTES);
    assert_eq!(policy.max_total_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn a_database_survives_a_close_and_reopen() {
    let dir = temp_dir("reopen");
    let db_path = dir.join("scoredb.sqlite");
    {
        let mut db = ScoreDb::open(&db_path).expect("opened");
        db.migrate().expect("migrated");
        db.record_play(&play("aa", 10, 100), true).expect("recorded");
    }
    let mut db = ScoreDb::open(&db_path).expect("reopened");
    assert_eq!(db.migrate().expect("migrated"), SCORE_DB_SCHEMA_VERSION);
    assert_eq!(db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists").ex_score(), 200);
    let backup = dir.join("backup/scoredb.sqlite");
    db.backup_to(&backup).expect("backed up");
    assert!(backup.exists(), "the snapshot is one file");
    let copy = ScoreDb::open(&backup).expect("the snapshot opens");
    assert_eq!(copy.play_count().expect("counted"), 1);
    drop(copy);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_chart_key_is_the_md5_and_falls_back_to_the_sha256() {
    assert_eq!(chart_key("AbCd", "ffff"), "abcd", "the md5 keys the record, lower-cased");
    assert_eq!(chart_key("", "FFFF"), "ffff", "a chart with no md5 is keyed by its sha256");
    assert_eq!(chart_key("", ""), "");
}
