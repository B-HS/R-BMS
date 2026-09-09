use crate::*;

fn rec(md5: &str, played_at: i64, ex: u32) -> ScoreRecord {
    ScoreRecord {
        md5: md5.into(),
        title: "t".into(),
        mode: "BEAT-7K".into(),
        clear: 5,
        ex_score: ex,
        max_ex: 100,
        counts: [0; 6],
        empty_poor: 0,
        max_combo: 0,
        total_notes: 50,
        gauge: "NORMAL".into(),
        gauge_value: 80.0,
        random: "OFF".into(),
        played_at,
        replay_file: None,
        rule_version: SCORE_RULE_VERSION,
        assisted: false,
    }
}

#[test]
fn for_md5_filters_and_sorts_newest_first() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 100, 1));
    book.push(rec("BB", 200, 2));
    book.push(rec("aa", 300, 3));
    let v = book.for_md5("aa");
    assert_eq!(v.len(), 2, "md5 match is case-insensitive");
    assert_eq!(v[0].played_at, 300, "newest first");
    assert_eq!(v[1].played_at, 100);
}

#[test]
fn ron_roundtrips() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 100, 42));
    let s = ron::ser::to_string_pretty(&book, ron::ser::PrettyConfig::default()).unwrap();
    let back: ScoreBook = ron::from_str(&s).unwrap();
    assert_eq!(back.records().len(), 1);
    assert_eq!(back.records()[0].ex_score, 42);
}

#[test]
fn for_md5_no_match_is_empty() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 100, 1));
    assert!(book.for_md5("ZZ").is_empty(), "no records for an unknown chart");
    assert!(book.for_md5("").is_empty(), "empty query matches nothing here");
}

#[test]
fn for_md5_on_empty_book_is_empty() {
    assert!(ScoreBook::default().for_md5("AA").is_empty());
}

#[test]
fn for_md5_preserves_count_of_matches() {
    let mut book = ScoreBook::default();
    for i in 0..5 {
        book.push(rec("AA", i, i as u32));
    }
    book.push(rec("BB", 99, 0));
    assert_eq!(book.for_md5("aa").len(), 5, "all and only AA records returned");
}

#[test]
fn for_md5_ties_on_played_at_keep_all_records() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 100, 7));
    book.push(rec("AA", 100, 9));
    let v = book.for_md5("AA");
    assert_eq!(v.len(), 2, "equal timestamps don't drop records");
    let exes: std::collections::HashSet<u32> = v.iter().map(|r| r.ex_score).collect();
    assert!(exes.contains(&7) && exes.contains(&9));
}

#[test]
fn for_md5_sorted_non_increasing_by_played_at() {
    let mut book = ScoreBook::default();
    for t in [50_i64, 10, 90, 30, 70] {
        book.push(rec("AA", t, 0));
    }
    let v = book.for_md5("aa");
    for w in v.windows(2) {
        assert!(w[0].played_at >= w[1].played_at, "newest-first ordering");
    }
    assert_eq!(v.first().unwrap().played_at, 90, "max timestamp first");
    assert_eq!(v.last().unwrap().played_at, 10, "min timestamp last");
}

#[test]
fn best_ex_picks_max_case_insensitively() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 1, 30));
    book.push(rec("aa", 2, 80));
    book.push(rec("Aa", 3, 50));
    book.push(rec("BB", 4, 999));
    assert_eq!(book.best_ex_for_md5("aA"), Some(80));
    assert_eq!(book.best_ex_for_md5("bb"), Some(999));
}

#[test]
fn best_ex_none_when_no_records() {
    let book = ScoreBook::default();
    assert_eq!(book.best_ex_for_md5("AA"), None);
    let mut book2 = ScoreBook::default();
    book2.push(rec("BB", 1, 5));
    assert_eq!(book2.best_ex_for_md5("AA"), None, "no record for that chart");
}

fn rec_clear(md5: &str, clear: u8) -> ScoreRecord {
    let mut r = rec(md5, 0, 0);
    r.clear = clear;
    r
}

#[test]
fn best_clear_picks_highest_lamp_id() {
    let mut book = ScoreBook::default();
    book.push(rec_clear("AA", 1));
    book.push(rec_clear("aa", 7));
    book.push(rec_clear("AA", 5));
    assert_eq!(book.best_clear_for_md5("aa"), Some(7), "max lamp id, case-insensitive");
}

#[test]
fn assisted_runs_are_kept_as_history_but_never_become_the_best() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 1, 40));
    let mut assisted = rec("AA", 2, 900);
    assisted.clear = 9;
    assisted.assisted = true;
    book.push(assisted);
    assert_eq!(book.for_md5("AA").len(), 2, "the assisted play is still in the history list");
    assert_eq!(book.best_ex_for_md5("AA"), Some(40));
    assert_eq!(book.best_clear_for_md5("AA"), Some(5));
}

#[test]
fn a_chart_with_only_assisted_records_has_no_best() {
    let mut book = ScoreBook::default();
    let mut assisted = rec("AA", 1, 500);
    assisted.assisted = true;
    book.push(assisted);
    assert_eq!(book.best_ex_for_md5("AA"), None);
    assert_eq!(book.best_clear_for_md5("AA"), None);
}

#[test]
fn best_clear_none_when_no_records() {
    assert_eq!(ScoreBook::default().best_clear_for_md5("AA"), None);
}

#[test]
fn best_ex_consistent_with_for_md5_max() {
    let mut book = ScoreBook::default();
    for (t, ex) in [(1_i64, 12_u32), (2, 80), (3, 45)] {
        book.push(rec("AA", t, ex));
    }
    let folded = book.best_ex_for_md5("AA");
    let by_query = book.for_md5("AA").iter().map(|r| r.ex_score).max();
    assert_eq!(folded, by_query, "two paths to the best EX agree");
    assert_eq!(folded, Some(80));
}

#[test]
fn push_appends_and_preserves_order_and_count() {
    let mut book = ScoreBook::default();
    assert_eq!(book.records().len(), 0);
    book.push(rec("AA", 1, 1));
    book.push(rec("BB", 2, 2));
    book.push(rec("CC", 3, 3));
    assert_eq!(book.records().len(), 3, "push grows by one each time");
    let md5s: Vec<&str> = book.records().iter().map(|r| r.md5.as_str()).collect();
    assert_eq!(md5s, ["AA", "BB", "CC"], "append-only, insertion order kept");
}

#[test]
fn empty_poor_and_replay_file_default_when_absent() {
    let s = r#"(records: [(
        md5: "AA", title: "t", mode: "BEAT-7K", clear: 5, ex_score: 10, max_ex: 100,
        counts: (0, 0, 0, 0, 0, 0), max_combo: 0, total_notes: 50,
        gauge: "NORMAL", gauge_value: 80.0, random: "OFF", played_at: 1
    )])"#;
    let book: ScoreBook = ron::from_str(s).expect("back-compat record parses");
    assert_eq!(book.records().len(), 1);
    assert_eq!(book.records()[0].empty_poor, 0, "empty_poor defaults to 0");
    assert_eq!(book.records()[0].replay_file, None, "replay_file defaults to None");
}

#[test]
fn rule_version_defaults_to_zero_for_a_pre_versioning_record() {
    let s = r#"(records: [(
        md5: "AA", title: "t", mode: "BEAT-7K", clear: 5, ex_score: 10, max_ex: 100,
        counts: (0, 0, 0, 0, 0, 0), max_combo: 0, total_notes: 50,
        gauge: "NORMAL", gauge_value: 80.0, random: "OFF", played_at: 1
    )])"#;
    let book: ScoreBook = ron::from_str(s).expect("back-compat record parses");
    assert_eq!(book.records()[0].rule_version, 0);
    assert!(!book.records()[0].assisted, "assisted defaults to false for a pre-versioning record");
    assert!(is_stale_rule_version(book.records()[0].rule_version));
}

#[test]
fn current_rule_version_records_are_not_marked() {
    assert_eq!(SCORE_RULE_VERSION, 1, "the current judging-rule generation");
    assert!(!is_stale_rule_version(SCORE_RULE_VERSION));
    assert!(!is_stale_rule_version(SCORE_RULE_VERSION + 1), "a newer record is not marked as old");
}

#[test]
fn rule_version_round_trips_through_ron() {
    let mut book = ScoreBook::default();
    let mut r = rec("AA", 1, 5);
    r.rule_version = 7;
    book.push(r);
    let s = ron::ser::to_string_pretty(&book, ron::ser::PrettyConfig::default()).unwrap();
    let back: ScoreBook = ron::from_str(&s).unwrap();
    assert_eq!(back.records()[0].rule_version, 7);
}

#[test]
fn save_leaves_no_temp_file_behind() {
    let dir = std::env::temp_dir().join(format!("rbms_scores_tmp_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("scores.ron");
    let mut book = ScoreBook::default();
    book.push(rec("AA", 1, 5));
    book.save(&path);
    assert!(path.exists());
    assert!(!path.with_file_name("scores.ron.tmp").exists(), "no leftover temp file");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn empty_book_ron_unit_parses() {
    let book: ScoreBook = ron::from_str("()").expect("empty unit parses");
    assert!(book.records().is_empty());
}

#[test]
fn full_record_ron_round_trip_preserves_all_fields() {
    let mut r = rec("DEAD", 12345, 777);
    r.counts = [1, 2, 3, 4, 5, 6];
    r.empty_poor = 9;
    r.replay_file = Some("r/abc.ron".into());
    r.max_combo = 321;
    r.gauge_value = 73.5;
    let mut book = ScoreBook::default();
    book.push(r);
    let s = ron::ser::to_string_pretty(&book, ron::ser::PrettyConfig::default()).unwrap();
    let back: ScoreBook = ron::from_str(&s).unwrap();
    let b = &back.records()[0];
    assert_eq!(b.counts, [1, 2, 3, 4, 5, 6]);
    assert_eq!(b.empty_poor, 9);
    assert_eq!(b.replay_file.as_deref(), Some("r/abc.ron"));
    assert_eq!(b.max_combo, 321);
    assert_eq!(b.played_at, 12345);
    assert!((b.gauge_value - 73.5).abs() < 1e-6);
}

#[test]
fn load_missing_file_returns_empty_book() {
    let dir = std::env::temp_dir().join(format!("rbms_scores_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("scores.ron");
    let book = ScoreBook::load(&path);
    assert!(book.records().is_empty(), "missing scores file => empty book");
    assert!(!path.exists(), "load() of a missing scores file does not create it");
}

#[test]
fn save_then_load_round_trips_on_disk() {
    let dir = std::env::temp_dir().join(format!("rbms_scores_io_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested/scores.ron");
    let mut book = ScoreBook::default();
    book.push(rec("AA", 5, 42));
    book.save(&path);
    assert!(path.exists(), "save creates the file (and parent dir)");
    let back = ScoreBook::load(&path);
    assert_eq!(back.records().len(), 1);
    assert_eq!(back.records()[0].ex_score, 42);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_deserialized_book_answers_queries_without_an_explicit_reindex() {
    let mut written = ScoreBook::default();
    written.push(rec("AA", 1, 30));
    written.push(rec("aa", 2, 90));
    written.push(rec("BB", 3, 10));
    let s = ron::ser::to_string_pretty(&written, ron::ser::PrettyConfig::default()).unwrap();
    let back: ScoreBook = ron::from_str(&s).unwrap();
    assert_eq!(back.for_md5("Aa").len(), 2, "the md5 index is rebuilt as part of deserializing");
    assert_eq!(back.best_ex_for_md5("aa"), Some(90));
}

#[test]
fn rebuild_index_restores_queries_after_the_book_is_rebuilt_from_records() {
    let mut book = ScoreBook::from_records(vec![rec("AA", 1, 10)]);
    assert_eq!(book.for_md5("AA").len(), 1);
    let mut records = book.records().to_vec();
    records.push(rec("AA", 2, 20));
    book = ScoreBook::from_records(records);
    book.rebuild_index();
    assert_eq!(book.for_md5("AA").len(), 2, "reindexing picks up the appended record");
    assert_eq!(book.best_ex_for_md5("AA"), Some(20));
}

#[test]
fn from_records_indexes_every_chart_it_is_given() {
    let book = ScoreBook::from_records(vec![rec("AA", 1, 10), rec("bb", 2, 20), rec("Aa", 3, 30)]);
    assert_eq!(book.for_md5("aa").len(), 2);
    assert_eq!(book.for_md5("BB").len(), 1);
    assert_eq!(book.for_md5("cc").len(), 0);
}

#[test]
fn try_save_reports_a_path_that_cannot_be_written() {
    let err = ScoreBook::default().try_save(std::path::Path::new("/")).expect_err("a directory path is not a writable target");
    assert!(matches!(err, StoreError::Write(_)), "the failure is reported as a write error, got {err:?}");
}

fn sample_replay() -> Replay {
    Replay {
        chart_path: "/songs/a.bms".into(),
        md5: "DEADBEEF".into(),
        mode: "BEAT-7K".into(),
        random: "RANDOM".into(),
        seed: 0xDEAD_BEEF_CAFE_F00D,
        offset_ms: -12,
        scratch_auto: true,
        gauge: "HARD".into(),
        events: vec![ReplayEvent { t: 0, lane: 0, press: true }, ReplayEvent { t: 1500, lane: 0, press: false }, ReplayEvent { t: 2000, lane: 7, press: true }],
    }
}

#[test]
fn ron_round_trip_preserves_events_and_meta() {
    let r = sample_replay();
    let s = ron::ser::to_string_pretty(&r, ron::ser::PrettyConfig::default()).unwrap();
    let back: Replay = ron::from_str(&s).unwrap();
    assert_eq!(back.chart_path, r.chart_path);
    assert_eq!(back.md5, r.md5);
    assert_eq!(back.seed, r.seed, "u64 seed survives exactly");
    assert_eq!(back.offset_ms, r.offset_ms);
    assert!(back.scratch_auto);
    assert_eq!(back.gauge, "HARD");
    assert_eq!(back.events.len(), r.events.len(), "event count preserved");
    for (a, b) in r.events.iter().zip(&back.events) {
        assert_eq!((a.t, a.lane, a.press), (b.t, b.lane, b.press));
    }
}

#[test]
fn empty_event_stream_round_trips() {
    let mut r = sample_replay();
    r.events.clear();
    let s = ron::ser::to_string_pretty(&r, ron::ser::PrettyConfig::default()).unwrap();
    let back: Replay = ron::from_str(&s).unwrap();
    assert!(back.events.is_empty(), "an empty replay is valid");
}

#[test]
fn scratch_auto_and_gauge_default_when_absent() {
    let s = r#"(
        chart_path: "/c.bms", md5: "AA", mode: "BEAT-7K", random: "OFF",
        seed: 7, offset_ms: 0, events: []
    )"#;
    let r: Replay = ron::from_str(s).expect("back-compat replay parses");
    assert!(!r.scratch_auto, "scratch_auto defaults to false");
    assert_eq!(r.gauge, "", "gauge defaults to empty string");
    assert_eq!(r.seed, 7);
}

#[test]
fn load_missing_file_is_err() {
    let path = std::env::temp_dir().join(format!("rbms_replay_missing_{}.ron", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let err = Replay::load(&path).expect_err("missing replay file is an Err, not a default");
    assert!(matches!(err, StoreError::Read(_)), "a missing file is a read error, got {err:?}");
}

#[test]
fn load_malformed_file_is_err() {
    let dir = std::env::temp_dir().join(format!("rbms_replay_bad_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.ron");
    std::fs::write(&path, "<<< not ron >>>").unwrap();
    let err = Replay::load(&path).expect_err("malformed replay is an Err");
    assert!(matches!(err, StoreError::Parse(_)), "unparsable RON is a parse error, got {err:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn replay_save_then_load_round_trips_on_disk() {
    let dir = std::env::temp_dir().join(format!("rbms_replay_io_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested/r.ron");
    sample_replay().save(&path);
    assert!(path.exists());
    let back = Replay::load(&path).expect("saved replay loads back");
    assert_eq!(back.events.len(), 3);
    assert_eq!(back.seed, 0xDEAD_BEEF_CAFE_F00D);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_atomic_writes_the_contents_and_leaves_no_temp_file() {
    let dir = std::env::temp_dir().join(format!("rbms_store_atomic_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested/out.ron");
    write_atomic(&path, "(hispeed: 1.0)").expect("atomic write creates the parent dir and the file");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "(hispeed: 1.0)");
    assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 1, "only the target file remains");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_serialized_book_carries_only_the_record_list() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 1, 5));
    let s = ron::ser::to_string_pretty(&book, ron::ser::PrettyConfig::default()).unwrap().replace("\r\n", "\n");
    assert!(s.trim_start().starts_with("(\n    records: ["), "the file shape is unchanged: {s}");
    assert!(!s.contains("index"), "the md5 lookup index never reaches the file: {s}");
}
