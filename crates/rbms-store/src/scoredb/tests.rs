use std::path::PathBuf;

use super::*;
use crate::{SCORE_LN_MODE_FROM_CHART, SCORE_RULE_VERSION, ScoreBook, ScoreRecord};

/// A play good enough to be a best: perfect on every note, no breaks.
fn play(chart_key: &str, date: i64, pgreat: u32) -> PlayLog {
    PlayLog {
        id: 0,
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

/// A SCRATCH AUTO run counts no turntable note, so its EX is out of fewer notes than a run of the
/// whole chart: the best row keeps the note count of the run whose EX it keeps.
#[test]
fn the_best_note_count_goes_with_the_ex_it_keeps() {
    let mut db = fresh();
    let mut whole = play("aa", 10, 300);
    whole.notes = 500;
    db.record_play(&whole, true).expect("a run of the whole chart");
    let mut fewer = play("aa", 20, 200);
    fewer.notes = 450;
    db.record_play(&fewer, true).expect("a worse SCRATCH AUTO run");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!((best.ex_score(), best.notes), (600, 500), "the worse run left the notes with the EX they belong to");
    let mut better = play("aa", 30, 440);
    better.notes = 450;
    db.record_play(&better, true).expect("a better SCRATCH AUTO run");
    let best = db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!((best.ex_score(), best.notes), (880, 450), "the better run brought its own note count");
}

/// A row an assisted run left with a lamp and no score takes its notes from the first run that may
/// set a best, whatever its EX.
#[test]
fn a_lamp_only_row_takes_the_notes_of_the_first_run_that_scores() {
    let mut db = fresh();
    db.record_play(&play("aa", 10, 300), false).expect("an assisted run");
    assert_eq!(db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists").notes, 0);
    db.record_play(&play("aa", 20, 0), true).expect("a run that scores nothing");
    assert_eq!(db.best("aa", mode_id("BEAT_7K")).expect("queried").expect("exists").notes, 0, "a run of no EX is out of its own notes");
}

#[test]
fn which_runs_set_bests_by_assist_and_rule_generation() {
    use crate::{SCORE_RULE_VERSION, SCRATCH_AUTO_ASSIST, is_stale_rule_version, sets_best};
    let wave16 = 2;
    assert!(sets_best(0, SCORE_RULE_VERSION) && sets_best(0, wave16) && sets_best(0, 0), "a run with no assist always does");
    assert!(sets_best(SCRATCH_AUTO_ASSIST, SCORE_RULE_VERSION), "SCRATCH AUTO does now");
    assert!(!sets_best(SCRATCH_AUTO_ASSIST, wave16), "a SCRATCH AUTO run that counted the turntable does not");
    assert!(!sets_best(2, SCORE_RULE_VERSION), "a widened judge never does");
    assert!(!sets_best(ASSIST_UNRECORDED, SCORE_RULE_VERSION), "an assist an imported record never named does not");
    assert!(!is_stale_rule_version(wave16), "a record of the judge-parity pass is judged as this build judges it");
    assert!(is_stale_rule_version(1));
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
    let first_id = db.record_play(&first, true).expect("one");
    db.record_play(&play("aa", 20, 50), true).expect("two");
    let history = db.history("aa", 10).expect("queried");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].date, 20, "newest first");
    assert_eq!(history[1], PlayLog { id: first_id, ..first }, "every field survives the round trip");
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
        log_id: None,
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
    let plan = db.replay_gc_plan(&dir, &policy, None).expect("planned");
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
    let plan = db.replay_gc_plan(&dir, &policy, None).expect("planned");
    assert!(!plan.delete.contains(&best_file), "the best run's replay is kept however old it is");
    assert_eq!(plan.kept, 2, "the newest and the best");

    let unprotected = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 1, max_total_bytes: 0 };
    let plan = db.replay_gc_plan(&dir, &unprotected, None).expect("planned");
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
    let plan = db.replay_gc_plan(&dir, &policy, None).expect("planned");
    assert_eq!(plan.kept, 2, "two hundred bytes fit under the ceiling, three hundred do not");
    assert_eq!(plan.delete, vec![names[0].clone(), names[1].clone()], "the oldest go first");
    assert_eq!(plan.freed_bytes, 200);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The server and account the hold fixtures sync to.
const HOLD_SERVER: &str = "https://bms.example";
const HOLD_ACCOUNT: &str = "dj";

/// A replay that has not gone up to the server the player syncs to is kept until it has: the
/// recent window would drop the two oldest of five, and the two whose plays the server has not
/// taken, or whose replay is still to go up, stay. One that went up goes as it always did.
#[test]
fn replay_gc_keeps_a_replay_still_to_go_up() {
    let dir = temp_dir("gc_hold");
    let mut db = fresh();
    let names = seed_replays(&mut db, &dir, "aa", 5, 10);
    let ids: Vec<i64> = db.history("aa", 10).expect("reads").iter().rev().map(|log| log.id).collect();
    for (id, replay_state) in
        [(ids[1], ReplayState::Pending), (ids[2], ReplayState::Uploaded), (ids[3], ReplayState::Uploaded), (ids[4], ReplayState::Uploaded)]
    {
        let row = IrSyncRow { replay_state, ..IrSyncRow::new(id, HOLD_SERVER, HOLD_ACCOUNT, SyncState::Accepted) };
        db.put_sync_state(&row).expect("written");
    }
    let policy = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 3, max_total_bytes: 0 };
    assert_eq!(db.replay_gc_plan(&dir, &policy, None).expect("planned").delete, [names[0].clone(), names[1].clone()], "nobody to upload to holds nothing");

    let hold = UploadHold { server: HOLD_SERVER.into(), login_id: HOLD_ACCOUNT.into() };
    let plan = db.replay_gc_plan(&dir, &policy, Some(&hold)).expect("planned");
    assert!(plan.delete.is_empty(), "the unsent play's replay and the pending upload are both held: {:?}", plan.delete);

    let other = UploadHold { server: HOLD_SERVER.into(), login_id: "someone-else".into() };
    db.put_submission(ids[0], "{}", Some(HOLD_ACCOUNT)).expect("kept");
    let plan = db.replay_gc_plan(&dir, &policy, Some(&other)).expect("planned");
    assert_eq!(plan.delete, [names[0].clone()], "a play of another account is not held for this one");

    let tight = ReplayPolicy { max_total_bytes: 30, ..policy };
    let plan = db.replay_gc_plan(&dir, &tight, Some(&hold)).expect("planned");
    assert_eq!(plan.delete, [names[0].clone(), names[1].clone()], "the byte ceiling still lets the oldest go");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A play kept on the side while the database could not take it is read in the next time it opens,
/// with its body, and its file is removed; a file that cannot be read is left alone.
#[test]
fn plays_kept_on_the_side_are_read_in_and_their_files_removed() {
    let dir = temp_dir("pending");
    let mut db = fresh();
    let run = PendingRun { log: play("aa", 10, 100), updates_best: true, body: Some("{\"kept\":1}".into()), played_as: Some(HOLD_ACCOUNT.into()) };
    std::fs::write(dir.join(format!("10000-aa{PENDING_RUN_SUFFIX}")), run.to_ron().expect("serialises")).expect("written");
    std::fs::write(dir.join(format!("20000-bad{PENDING_RUN_SUFFIX}")), "<<< not ron >>>").expect("written");
    std::fs::write(dir.join("30000-aa.ron.1.2.tmp"), "a temp file").expect("written");
    let report = db.import_pending_runs(&dir).expect("imported");
    assert_eq!(report.imported, 1);
    assert_eq!(report.unreadable, [dir.join(format!("20000-bad{PENDING_RUN_SUFFIX}"))]);
    assert!(report.stuck.is_empty());
    let history = db.history("aa", 10).expect("reads");
    assert_eq!(history.len(), 1);
    assert_eq!(PlayLog { id: 0, ..history[0].clone() }, run.log);
    let walk = SyncWalk { server: HOLD_SERVER, login_id: HOLD_ACCOUNT, ..SyncWalk::default() };
    assert_eq!(db.sync_candidates(&walk).expect("reads")[0].body.as_deref(), Some("{\"kept\":1}"));
    assert!(!dir.join(format!("10000-aa{PENDING_RUN_SUFFIX}")).exists());
    assert_eq!(db.import_pending_runs(&dir.join("absent")).expect("nothing").imported, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A kept play the database refuses is reported and its file left, and the plays kept after it
/// are still read in: one file that always fails does not hold back every later play.
#[test]
fn a_kept_play_the_database_refuses_does_not_hold_back_the_ones_after_it() {
    let dir = temp_dir("pending_refused");
    let mut db = fresh();
    db.conn
        .execute_batch("CREATE TEMP TRIGGER refuse_poison BEFORE INSERT ON scorelog WHEN NEW.chart_key = 'poison' BEGIN SELECT RAISE(ABORT, 'refused'); END")
        .expect("the trigger is made");
    let poison = dir.join(format!("10000-poison{PENDING_RUN_SUFFIX}"));
    let later = dir.join(format!("20000-aa{PENDING_RUN_SUFFIX}"));
    for (path, chart) in [(&poison, "poison"), (&later, "aa")] {
        let run = PendingRun { log: play(chart, 10, 100), updates_best: true, body: None, played_as: None };
        std::fs::write(path, run.to_ron().expect("serialises")).expect("written");
    }
    let report = db.import_pending_runs(&dir).expect("the import goes on past a refused play");
    assert_eq!(report.imported, 1);
    assert_eq!(report.refused.iter().map(|(path, _)| path.as_path()).collect::<Vec<_>>(), [poison.as_path()]);
    assert_eq!(db.history("aa", 10).expect("reads").len(), 1, "the play kept after the refused one is in");
    assert!(poison.exists(), "the refused play's file is left for the next start");
    assert!(!later.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A kept play read in once whose file could not be removed is not read in again the next time:
/// the history does not hold it twice and the day's totals do not count it twice. Its file goes.
#[test]
fn a_kept_play_already_read_in_is_not_read_in_twice() {
    let dir = temp_dir("pending_twice");
    let mut db = fresh();
    let file = dir.join(format!("10000-aa{PENDING_RUN_SUFFIX}"));
    let run = PendingRun { log: play("aa", 10, 100), updates_best: true, body: None, played_as: None };
    let text = run.to_ron().expect("serialises");
    std::fs::write(&file, &text).expect("written");
    assert_eq!(db.import_pending_runs(&dir).expect("imported").imported, 1);
    let day = db.day_stats(i64::MIN, i64::MAX).expect("reads");
    std::fs::write(&file, &text).expect("the same file again, as one that could not be removed");
    let report = db.import_pending_runs(&dir).expect("imported");
    assert_eq!(report.imported, 0, "the play was already in");
    assert_eq!(db.history("aa", 10).expect("reads").len(), 1, "the history holds the play once");
    assert_eq!(db.day_stats(i64::MIN, i64::MAX).expect("reads"), day, "the day's totals count it once");
    assert!(!file.exists(), "its file is removed");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn each_chart_keeps_its_own_recent_window() {
    let dir = temp_dir("gc_charts");
    let mut db = fresh();
    seed_replays(&mut db, &dir, "aa", 3, 10);
    seed_replays(&mut db, &dir, "bb", 3, 10);
    let policy = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 2, max_total_bytes: 0 };
    let plan = db.replay_gc_plan(&dir, &policy, None).expect("planned");
    assert_eq!(plan.kept, 4, "two per chart");
    assert_eq!(plan.delete, vec!["aa-0.ron".to_string(), "bb-0.ron".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_database_with_no_replays_plans_nothing() {
    let dir = temp_dir("gc_empty");
    let mut db = fresh();
    db.record_play(&play("aa", 10, 100), true).expect("a play with no replay");
    let plan = db.replay_gc_plan(&dir, &ReplayPolicy::default(), None).expect("planned");
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

/// The rule generation before plays were recorded as the reference records them.
const RULE_BEFORE_THE_REFERENCE_RECORD: u32 = REFERENCE_RECORD_RULE_VERSION - 1;

/// The `avgjudge` an old play holds: the size of its hits' mean signed offset, which cancels early
/// against late and so comes out near nothing for a run that was anything but accurate.
const CANCELLED_OUT_AVGJUDGE: i64 = 29;

/// A reference `avgjudge`: the mean distance of every note from its input.
const REFERENCE_AVGJUDGE: i64 = 10_000;

/// An old play's `avgjudge` measured something else, so it never stands as a chart's best against
/// one of the reference's: the best is the reference value however much smaller the old one is.
#[test]
fn an_old_generation_avgjudge_does_not_beat_a_reference_one() {
    let mut db = fresh();
    let old = PlayLog { avgjudge: CANCELLED_OUT_AVGJUDGE, rule_version: RULE_BEFORE_THE_REFERENCE_RECORD, ..play("abcd", 1, 10) };
    db.record_play(&old, true).expect("the old play writes");
    let best = db.best("abcd", old.mode).expect("reads").expect("exists");
    assert_eq!(best.avgjudge, UNSET_MINIMUM, "an old play alone leaves the chart with no avgjudge");

    db.record_play(&PlayLog { avgjudge: REFERENCE_AVGJUDGE, ..play("abcd", 2, 10) }, true).expect("the new play writes");
    let best = db.best("abcd", old.mode).expect("reads").expect("exists");
    assert_eq!(best.avgjudge, REFERENCE_AVGJUDGE);
}

/// A build older than the reference record may write to the database after one of this rule has,
/// folding an old `avgjudge` into the best; the next open of this build folds the best afresh from
/// the plays.
#[test]
fn an_old_avgjudge_folded_in_by_an_older_build_is_taken_out_on_the_next_open() {
    let mut db = fresh();
    db.record_play(&PlayLog { avgjudge: REFERENCE_AVGJUDGE, ..play("abcd", 2, 10) }, true).expect("the new play writes");
    db.record_play(&PlayLog { avgjudge: CANCELLED_OUT_AVGJUDGE, rule_version: RULE_BEFORE_THE_REFERENCE_RECORD, ..play("abcd", 3, 10) }, true)
        .expect("the old build's play writes");
    db.conn.execute("UPDATE score SET avgjudge = ?1", params![CANCELLED_OUT_AVGJUDGE]).expect("the older build folds its own value in");
    db.migrate().expect("this build opens the database again");
    let best = db.best("abcd", mode_id("BEAT_7K")).expect("reads").expect("exists");
    assert_eq!(best.avgjudge, REFERENCE_AVGJUDGE);
}

/// Notes of the fixture chart an old failed play took before its gauge ran out.
const OLD_FAILED_PLAY_TAKEN: u32 = 10;

/// Notes of the fixture chart an old failed play never reached.
const OLD_FAILED_PLAY_UNREACHED: u32 = 90;

/// An old play the gauge ended counted only what it judged as its bad-poor count, which made dying
/// at once the best MISS COUNT a chart could have. Wherever it is compared it is charged for the
/// notes it never reached, as the reference records it (`BMSPlayer.java:909`): when it is folded in,
/// and when the bests are folded afresh on open. A play of this rule already counts them.
#[test]
fn an_old_failed_play_is_charged_for_the_notes_it_never_reached() {
    let notes = OLD_FAILED_PLAY_TAKEN + OLD_FAILED_PLAY_UNREACHED;
    let old = PlayLog { clear: LAMP_FAILED, notes, rule_version: RULE_BEFORE_THE_REFERENCE_RECORD, ..play("abcd", 1, OLD_FAILED_PLAY_TAKEN) };
    assert_eq!(charged_minbp(&old), i64::from(OLD_FAILED_PLAY_UNREACHED));
    let new = PlayLog { clear: LAMP_FAILED, notes, minbp: OLD_FAILED_PLAY_UNREACHED, ..play("abcd", 2, OLD_FAILED_PLAY_TAKEN) };
    assert_eq!(charged_minbp(&new), i64::from(OLD_FAILED_PLAY_UNREACHED), "a play of this rule is charged once");

    let mut db = fresh();
    db.record_play(&old, true).expect("the old play writes");
    assert_eq!(db.best("abcd", old.mode).expect("reads").expect("exists").minbp, i64::from(OLD_FAILED_PLAY_UNREACHED));
    db.conn.execute("UPDATE score SET minbp = 0", []).expect("an older build folded the uncharged count in");
    db.migrate().expect("this build opens the database again");
    assert_eq!(db.best("abcd", old.mode).expect("reads").expect("exists").minbp, i64::from(OLD_FAILED_PLAY_UNREACHED));
}

/// A pop'n BAD leaves its note to be judged again, so an old pop'n play that took every note is
/// charged nothing for its BADs.
#[test]
fn an_old_popn_play_is_not_charged_for_its_bads() {
    let mut finished = play("abcd", 1, 10);
    finished.mode = mode_id("POPN_9K");
    finished.rule_version = RULE_BEFORE_THE_REFERENCE_RECORD;
    finished.judge = JudgeCounts::from_early_late([10, 0, 0, 2, 0, 0], [0; JUDGE_TIER_COUNT]);
    finished.minbp = 2;
    assert_eq!(charged_minbp(&finished), 2);
}
