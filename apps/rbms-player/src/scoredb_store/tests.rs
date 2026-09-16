use std::path::PathBuf;

use rbms_store::SCORE_LN_MODE_FROM_CHART;
use rbms_store::scoredb::{JUDGE_PGREAT, UNSET_MINIMUM};

use super::*;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms_scoredb_store_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temp dir is creatable");
    dir
}

fn summary() -> PlaySummary {
    PlaySummary {
        counts: [90, 6, 2, 1, 1, 0],
        ex_score: 186,
        max_ex_score: 200,
        max_combo: 96,
        total_notes: 100,
        total_judged: 100,
        fast: 40,
        slow: 60,
        early: [40, 2, 1, 0, 1, 0],
        late: [50, 4, 1, 1, 0, 0],
        avg_judge_us: -4_200,
        empty_poor: 0,
        gauge_value: 78.5,
        clear_lamp: ClearType::Normal,
        min_bp: 2,
        failed: false,
        finished_gauge: Some(GaugeKind::Normal),
        gauge_shifted: false,
    }
}

fn finished<'a>(summary: &'a PlaySummary, md5: &'a str) -> FinishedPlay<'a> {
    FinishedPlay {
        md5,
        sha256: "SHA",
        title: "Chart",
        mode: Mode::BEAT_7K,
        ln_mode: SCORE_LN_MODE_FROM_CHART,
        lamp: ClearType::Normal,
        summary,
        gauge: GaugeKind::Normal,
        random: NoteOption::Mirror,
        seed: 42,
        assist: 0,
        played_at_ms: 1_700_000_123_456,
        playtime_ms: 95_000,
        ir_submitted: true,
        replay_file: Some("aa-1.ron".into()),
    }
}

fn fresh() -> ScoreDb {
    let mut db = ScoreDb::open_in_memory().expect("an in-memory database opens");
    db.migrate().expect("the schema applies");
    db
}

#[test]
fn score_paths_sit_beside_the_settings_file() {
    let (db, ron, replays) = score_paths(Path::new("/cfg/rbms"));
    assert_eq!(db, PathBuf::from("/cfg/rbms/scoredb.sqlite"));
    assert_eq!(ron, PathBuf::from("/cfg/rbms/scores.ron"));
    assert_eq!(replays, PathBuf::from("/cfg/rbms/replays"));
}

#[test]
fn a_finished_run_becomes_the_row_the_database_stores() {
    let summary = summary();
    let log = play_log(&finished(&summary, "AbCd"));
    assert_eq!(log.chart_key, "abcd", "the md5 keys the record even though a sha256 is known");
    assert_eq!(log.md5, "abcd");
    assert_eq!(log.sha256, "sha");
    assert_eq!(log.mode, mode_id("BEAT_7K"));
    assert_eq!(log.clear, clear_type_id(ClearType::Normal));
    assert_eq!(log.judge.early, summary.early, "the early/late split is carried, not folded");
    assert_eq!(log.judge.late, summary.late);
    assert_eq!(log.judge.ex_score(), summary.ex_score, "and it reproduces the EX the screen showed");
    assert_eq!(log.notes, 100);
    assert_eq!(log.minbp, 2);
    assert_eq!(log.avgjudge, 4_200, "the mean timing error is stored as a magnitude");
    assert_eq!(log.gauge, gauge_id("normal"));
    assert_eq!(log.random, random_id("MIRROR"));
    assert_eq!(log.date, 1_700_000_123, "the millisecond stamp becomes a second");
    assert_eq!(log.state, PLAY_STATE_LIVE);
    assert_eq!(log.rule_version, SCORE_RULE_VERSION);
    assert!(log.ir_submitted);
    assert_eq!(log.replay_file.as_deref(), Some("aa-1.ron"));
}

#[test]
fn a_chart_with_no_md5_falls_back_to_its_sha256() {
    let summary = summary();
    let mut play = finished(&summary, "");
    play.sha256 = "FFEE";
    assert_eq!(play_log(&play).chart_key, "ffee");
}

#[test]
fn a_stored_row_reads_back_as_the_record_the_screens_query() {
    let summary = summary();
    let log = play_log(&finished(&summary, "abcd"));
    let record = record_of(&log);
    assert_eq!(record.md5, "abcd");
    assert_eq!(record.title, "Chart");
    assert_eq!(record.mode, "BEAT_7K", "the mode name the player writes round-trips through its id");
    assert_eq!(record.clear, clear_type_id(ClearType::Normal));
    assert_eq!(record.ex_score, summary.ex_score);
    assert_eq!(record.max_ex, summary.max_ex_score, "the EX ceiling falls out of the note count");
    assert_eq!(record.counts, summary.counts, "the two halves of each tier add back up");
    assert_eq!(record.max_combo, summary.max_combo);
    assert_eq!(record.total_notes, summary.total_notes);
    assert_eq!(record.gauge, "normal");
    assert_eq!(record.random, "MIRROR");
    assert_eq!(record.played_at, 1_700_000_123_000, "the second becomes a millisecond stamp again");
    assert_eq!(record.ln_mode, SCORE_LN_MODE_FROM_CHART);
    assert_eq!(record.rule_version, SCORE_RULE_VERSION);
    assert!(!record.assisted);
    assert_eq!(record.replay_file.as_deref(), Some("aa-1.ron"));
}

#[test]
fn an_assisted_run_reads_back_as_assisted() {
    let summary = summary();
    let mut play = finished(&summary, "abcd");
    play.assist = 2;
    let record = record_of(&play_log(&play));
    assert!(record.assisted, "the assist level the run carried is what marks the record");
}

#[test]
fn a_row_whose_gauge_id_names_nothing_reads_as_the_default_gauge() {
    let summary = summary();
    let mut log = play_log(&finished(&summary, "abcd"));
    log.gauge = 999;
    assert_eq!(record_of(&log).gauge, gauge_token(GaugeKind::Normal));
}

#[test]
fn the_book_is_rebuilt_from_the_database_with_the_queries_the_screens_make() {
    let mut db = fresh();
    let summary = summary();
    let log = play_log(&finished(&summary, "abcd"));
    assert!(record_finished_play(&mut db, &log, true), "the play records");

    let mut weaker = log.clone();
    weaker.judge = JudgeCounts::from_early_late([1, 0, 0, 0, 0, 0], [0; JUDGE_TIER_COUNT]);
    weaker.date = log.date + 60;
    assert!(record_finished_play(&mut db, &weaker, true));

    let book = book_from_db(&db);
    assert_eq!(book.records().len(), 2);
    assert_eq!(book.for_md5("ABCD").len(), 2, "the book still answers case-insensitively");
    assert_eq!(book.best_ex_for_md5("abcd"), Some(summary.ex_score));
    assert_eq!(book.best_clear_for_md5("abcd"), Some(clear_type_id(ClearType::Normal)));
    assert_eq!(book.best_ex_for_md5_in_ln_mode("abcd", SCORE_LN_MODE_FROM_CHART), Some(summary.ex_score));
    assert_eq!(book.for_md5("abcd")[0].played_at, weaker.date * 1_000, "newest first");
}

#[test]
fn an_assisted_run_is_history_in_the_rebuilt_book_just_as_it_is_in_the_database() {
    let mut db = fresh();
    let summary = summary();
    let clean = play_log(&finished(&summary, "abcd"));
    record_finished_play(&mut db, &clean, true);
    let mut assisted = clean.clone();
    assisted.assist = 2;
    assisted.date = clean.date + 60;
    assisted.judge = JudgeCounts::from_early_late([500, 0, 0, 0, 0, 0], [0; JUDGE_TIER_COUNT]);
    record_finished_play(&mut db, &assisted, false);

    let book = book_from_db(&db);
    assert_eq!(book.best_ex_for_md5("abcd"), Some(summary.ex_score), "the assisted run is not a best in the book");
    let best = db.best("abcd", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.ex_score(), summary.ex_score, "nor in the database");
    assert_eq!(best.playcount, 2, "both stores counted the play");
}

#[test]
fn a_run_with_no_recorded_timing_keeps_the_unset_marker_out_of_the_bests() {
    let mut db = fresh();
    let summary = summary();
    let mut log = play_log(&finished(&summary, "abcd"));
    log.avgjudge = UNSET_MINIMUM;
    record_finished_play(&mut db, &log, true);
    let best = db.best("abcd", mode_id("BEAT_7K")).expect("queried").expect("exists");
    assert_eq!(best.avgjudge, UNSET_MINIMUM, "an unset timing never looks like the best timing");
    assert_eq!(best.judge.tier(JUDGE_PGREAT), summary.counts[JUDGE_PGREAT], "the rest of the run still set the best");
}

#[test]
fn opening_the_database_imports_the_book_once_and_reports_it() {
    let dir = temp_dir("open");
    let (db_path, ron_path, _) = score_paths(&dir);
    let mut book = ScoreBook::default();
    let summary = summary();
    book.push(record_of(&play_log(&finished(&summary, "abcd"))));
    book.try_save(&ron_path).expect("the book is written");

    let db = open_score_db(&db_path, &ron_path).expect("the database opens");
    assert_eq!(db.play_count().expect("counted"), 1);
    assert!(!ron_path.exists(), "the book is moved aside once its records are in the database");
    drop(db);

    let db = open_score_db(&db_path, &ron_path).expect("it opens again");
    assert_eq!(db.play_count().expect("counted"), 1, "and is not re-imported");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_database_that_cannot_be_opened_leaves_the_player_running() {
    let dir = temp_dir("unopenable");
    let db_path = dir.join("scoredb.sqlite");
    std::fs::write(&db_path, "this is not a database").expect("a file in the way");
    assert!(open_score_db(&db_path, &dir.join("nothing.ron")).is_none(), "the failure is reported, not fatal");
    assert!(db_path.exists(), "and the file that is not a database is left exactly as it was");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_retention_run_deletes_the_files_it_planned_and_forgets_their_rows() {
    let dir = temp_dir("gc");
    let mut db = fresh();
    let summary = summary();
    for i in 0..4 {
        let name = format!("aa-{i}.ron");
        std::fs::write(dir.join(&name), "replay").expect("a replay file");
        let mut log = play_log(&finished(&summary, "abcd"));
        log.date += i;
        log.replay_file = Some(name);
        record_finished_play(&mut db, &log, true);
    }
    let policy = ReplayPolicy { keep_best_per_chart: false, keep_recent_per_chart: 2, max_total_bytes: 0 };
    let plan = run_replay_gc(&mut db, &dir, &policy).expect("planned and applied");
    assert_eq!(plan.delete, vec!["aa-0.ron".to_string(), "aa-1.ron".to_string()]);
    assert!(!dir.join("aa-0.ron").exists(), "the file is gone");
    assert!(dir.join("aa-2.ron").exists(), "the kept ones are not");
    let history = db.history("abcd", 10).expect("queried");
    assert_eq!(history[3].replay_file, None, "the oldest row no longer offers a replay that is not there");
    assert_eq!(history[0].replay_file.as_deref(), Some("aa-3.ron"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_retention_run_with_nothing_to_delete_changes_nothing() {
    let dir = temp_dir("gc_noop");
    let mut db = fresh();
    let summary = summary();
    let mut log = play_log(&finished(&summary, "abcd"));
    log.replay_file = None;
    record_finished_play(&mut db, &log, true);
    let plan = run_replay_gc(&mut db, &dir, &ReplayPolicy::default()).expect("planned");
    assert!(plan.delete.is_empty());
    assert_eq!(plan.kept, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_play_time_of_a_run_is_the_song_time_of_its_last_row() {
    assert_eq!(playtime_ms(95_432_000), 95_432);
    assert_eq!(playtime_ms(0), 0);
    assert_eq!(playtime_ms(-5), 0, "a chart with no rows is no play time, never a negative one");
}
