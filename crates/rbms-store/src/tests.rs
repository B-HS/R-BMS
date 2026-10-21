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
        ln_mode: SCORE_LN_MODE_FROM_CHART.into(),
        assisted: false,
        assist: 0,
        log_id: None,
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

/// An assisted run is history rather than a score, but its lamp still counts: the reference records
/// the lamp of an assisted play after demoting it, so a record written with a demoted lamp is what
/// the LED shows (`BMSPlayer.java:864-874`).
#[test]
fn an_assisted_run_never_becomes_the_best_ex_but_its_lamp_still_counts() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 1, 40));
    let mut assisted = rec("AA", 2, 900);
    assisted.clear = 3;
    assisted.assisted = true;
    book.push(assisted);
    assert_eq!(book.for_md5("AA").len(), 2, "the assisted play is still in the history list");
    assert_eq!(book.best_ex_for_md5("AA"), Some(40), "the assisted EX is not a best");
    assert_eq!(book.best_clear_for_md5("AA"), Some(5), "the unassisted lamp is still the higher one");

    let mut book = ScoreBook::default();
    book.push(rec_clear("AA", 1));
    let mut assisted = rec("AA", 2, 900);
    assisted.clear = 3;
    assisted.assisted = true;
    book.push(assisted);
    assert_eq!(book.best_clear_for_md5("AA"), Some(3), "a demoted assist lamp raises the LED as far as the assist allows");
}

#[test]
fn a_chart_with_only_assisted_records_has_no_best_ex_but_keeps_its_lamp() {
    let mut book = ScoreBook::default();
    let mut assisted = rec("AA", 1, 500);
    assisted.clear = 2;
    assisted.assisted = true;
    book.push(assisted);
    assert_eq!(book.best_ex_for_md5("AA"), None);
    assert_eq!(book.best_clear_for_md5("AA"), Some(2));
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
    assert_eq!(SCORE_RULE_VERSION, 4, "the current judging-rule generation");
    assert_eq!(REFERENCE_RECORD_RULE_VERSION, SCORE_RULE_VERSION, "version 4 records a run as the reference does");
    assert!(!is_stale_rule_version(SCORE_RULE_VERSION));
    assert!(!is_stale_rule_version(SCORE_RULE_VERSION - 1), "version 4 changed what a run records, not how a run by hand is judged or lamped");
    assert!(!is_stale_rule_version(SCORE_RULE_VERSION - 2), "version 3 changed SCRATCH AUTO alone, so a record by hand of version 2 is not old");
    assert!(!is_stale_rule_version(SCORE_RULE_VERSION + 1), "a newer record is not marked as old");
}

/// A record written by the build before the judge-parity pass keeps loading and reads as stale, so
/// the screens can mark it rather than dropping it.
#[test]
fn a_record_from_the_previous_rule_generation_still_loads_and_reads_as_stale() {
    let s = r#"(records: [(
        md5: "AA", title: "t", mode: "BEAT-7K", clear: 5, ex_score: 10, max_ex: 100,
        counts: (0, 0, 0, 0, 0, 0), max_combo: 0, total_notes: 50,
        gauge: "NORMAL", gauge_value: 80.0, random: "OFF", played_at: 1, rule_version: 1
    )])"#;
    let book: ScoreBook = ron::from_str(s).expect("a previous-generation record parses");
    assert_eq!(book.records()[0].rule_version, 1);
    assert_eq!(book.records()[0].ex_score, 10, "the record keeps every value it was written with");
    assert!(is_stale_rule_version(book.records()[0].rule_version));
    assert_eq!(book.best_ex_for_md5("AA"), Some(10), "a stale record still counts towards the bests");
}

/// A replay written before the JUDGE settings were recorded plays back the way the engine judged
/// when it was made, rather than picking up whatever the settings hold now.
#[test]
fn a_replay_without_a_judge_block_reads_as_the_pre_parity_defaults() {
    let s = r#"(
        chart_path: "/songs/a.bms", md5: "AA", mode: "BEAT_7K", random: "OFF", seed: 1, offset_ms: 0,
        events: []
    )"#;
    let replay: Replay = ron::from_str(s).expect("a pre-parity replay parses");
    assert_eq!(replay.judge, ReplayJudge::default());
    assert_eq!(replay.judge.algorithm, REPLAY_LEGACY_ALGORITHM);
    assert_eq!(replay.judge.judge_rate_key, [REPLAY_UNMODIFIED_RATE_PERCENT; REPLAY_JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(replay.judge.judge_rate_scratch, [REPLAY_UNMODIFIED_RATE_PERCENT; REPLAY_JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(replay.judge.longnote_margin_rate, REPLAY_UNMODIFIED_RATE_PERCENT);
}

/// The recorded JUDGE settings survive the file, which is what makes a replay reproduce its run.
#[test]
fn the_recorded_judge_settings_round_trip_through_ron() {
    let replay = sample_replay();
    let text = ron::ser::to_string_pretty(&replay, ron::ser::PrettyConfig::default()).unwrap();
    let back: Replay = ron::from_str(&text).unwrap();
    assert_eq!(back.judge, replay.judge);
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

/// The TOTAL the sample run was played on in place of the chart's.
const SAMPLE_TOTAL_OVERRIDE: f64 = 480.0;

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
        judge: ReplayJudge {
            algorithm: "Combo".into(),
            judge_rate_key: [95, 90, 85],
            judge_rate_scratch: [110, 105, 100],
            longnote_margin_rate: 120,
            ln_mode: "CN".into(),
            gauge_set: "LR2".into(),
            gauge_auto_shift: "BESTCLEAR".into(),
            bottom_shiftable_gauge: "normal".into(),
            offset_steps: vec![ReplayOffsetStep { from_event: 0, offset_ms: -12 }, ReplayOffsetStep { from_event: 2, offset_ms: -11 }],
            clock: REPLAY_EVENT_CLOCK.into(),
            total_override: SAMPLE_TOTAL_OVERRIDE,
        },
        events: vec![
            ReplayEvent { t: 0, lane: 0, press: true, backward: false },
            ReplayEvent { t: 1500, lane: 0, press: false, backward: false },
            ReplayEvent { t: 2000, lane: 7, press: true, backward: true },
        ],
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
        assert_eq!(a, b);
    }
    assert_eq!(back.judge, r.judge, "every JUDGE row the run was played under survives the round trip");
}

#[test]
fn a_replay_written_before_the_judge_rows_existed_reads_as_the_engines_old_behaviour() {
    let judge: ReplayJudge = ron::from_str("()").expect("an empty judge block is the pre-parity shape");
    assert_eq!(judge.algorithm, REPLAY_LEGACY_ALGORITHM);
    assert_eq!(judge.judge_rate_key, [REPLAY_UNMODIFIED_RATE_PERCENT; REPLAY_JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(judge.ln_mode, REPLAY_LEGACY_LN_MODE);
    assert_eq!(judge.gauge_set, REPLAY_LEGACY_GAUGE_SET);
    assert_eq!(judge.gauge_auto_shift, REPLAY_LEGACY_GAUGE_AUTO_SHIFT);
    assert_eq!(judge.bottom_shiftable_gauge, REPLAY_LEGACY_BOTTOM_SHIFTABLE_GAUGE);
    assert_eq!(judge.clock, REPLAY_LEGACY_CLOCK, "a replay that does not say how its clock moved was judged frame by frame");
    assert_eq!(judge.total_override, REPLAY_CHART_TOTAL, "a replay that does not state a TOTAL was played on the chart's own");
}

#[test]
fn an_event_written_before_the_scratch_direction_existed_reads_as_forward() {
    let event: ReplayEvent = ron::from_str("(t: 42, lane: 7, press: true)").expect("the pre-parity event shape still parses");
    assert_eq!(event, ReplayEvent { t: 42, lane: 7, press: true, backward: false });
}

#[test]
fn a_record_written_before_the_ln_mode_key_existed_compares_as_a_chart_stated_one() {
    let text = ron::ser::to_string(&rec("AA", 1, 5)).unwrap().replace(&format!("ln_mode:\"{SCORE_LN_MODE_FROM_CHART}\","), "");
    let back: ScoreRecord = ron::from_str(&text).expect("a record with no ln_mode still parses");
    assert_eq!(back.ln_mode, SCORE_LN_MODE_FROM_CHART);
}

#[test]
fn the_ln_mode_key_splits_the_bests_of_one_chart() {
    let mut book = ScoreBook::default();
    let mut plain = rec("AA", 100, 40);
    plain.ln_mode = "LN".into();
    let mut charge = rec("AA", 200, 90);
    charge.ln_mode = "CN".into();
    charge.clear = 7;
    book.push(plain);
    book.push(charge);
    assert_eq!(book.best_ex_for_md5_in_ln_mode("AA", "LN"), Some(40), "the CN run has twice the notes, so it is not a better LN run");
    assert_eq!(book.best_ex_for_md5_in_ln_mode("AA", "CN"), Some(90));
    assert_eq!(book.best_clear_for_md5_in_ln_mode("AA", "LN"), Some(5));
    assert_eq!(book.best_clear_for_md5_in_ln_mode("AA", "CN"), Some(7));
    assert_eq!(book.best_ex_for_md5("AA"), Some(90), "the unkeyed fold still spans every mode");
}

#[test]
fn an_assisted_record_from_an_older_rule_version_cannot_raise_the_clear_lamp() {
    let mut book = ScoreBook::default();
    let mut stale = rec("AA", 100, 10);
    stale.assisted = true;
    stale.rule_version = 1;
    stale.clear = 8;
    book.push(stale);
    assert_eq!(book.best_clear_for_md5("AA"), None, "nothing demoted that lamp, so it is not an assisted clear this build would have written");

    let mut current = rec("AA", 200, 10);
    current.assisted = true;
    current.clear = 3;
    book.push(current);
    assert_eq!(book.best_clear_for_md5("AA"), Some(3), "a demoted lamp this build wrote still lights the LED");

    let mut counted_the_turntable = rec("AA", 300, 10);
    counted_the_turntable.assisted = true;
    counted_the_turntable.rule_version = SCORE_RULE_VERSION - 1;
    counted_the_turntable.clear = 4;
    book.push(counted_the_turntable);
    assert_eq!(book.best_clear_for_md5("AA"), Some(4), "a SCRATCH AUTO record of the rule before keeps its demoted lamp");
    assert_eq!(book.best_ex_for_md5("AA"), None, "and still sets no score");
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
fn a_replay_written_before_the_offset_could_move_has_no_offset_steps() {
    let s = r#"(
        chart_path: "/c.bms", md5: "AA", mode: "BEAT-7K", random: "OFF", seed: 7, offset_ms: -4,
        judge: (algorithm: "Combo", judge_rate_key: (100, 100, 100)),
        events: []
    )"#;
    let r: Replay = ron::from_str(s).expect("a replay with no offset steps parses");
    assert_eq!(r.offset_ms, -4);
    assert!(r.judge.offset_steps.is_empty(), "the whole run was judged at the one offset");
    assert!(ReplayJudge::default().offset_steps.is_empty());
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
    sample_replay().try_save(&path).expect("the replay is written");
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

/// The bits of a Unix file mode that say who may read or write the file.
#[cfg(unix)]
const PERMISSION_BITS: u32 = 0o777;
#[cfg(unix)]
const OWNER_ONLY: u32 = 0o600;
#[cfg(unix)]
const OPEN_TO_OTHERS: u32 = 0o644;

#[cfg(unix)]
fn mode_of(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).expect("the file is there").permissions().mode() & PERMISSION_BITS
}

#[cfg(unix)]
fn open_to_others(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(OPEN_TO_OTHERS)).expect("opened");
}

#[cfg(unix)]
#[test]
fn a_private_write_creates_a_file_only_its_owner_can_read() {
    let dir = std::env::temp_dir().join(format!("rbms_store_private_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("settings.ron");
    write_atomic_private(&path, "(ir_token: Some(\"secret\"))").expect("written");
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "(ir_token: Some(\"secret\"))");
    assert_eq!(mode_of(&path), OWNER_ONLY, "a file with a token was left open to other users");
    assert_eq!(std::fs::read_dir(&dir).expect("lists").count(), 1, "no temp file is left behind");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_private_write_closes_a_file_an_older_build_left_open() {
    let dir = std::env::temp_dir().join(format!("rbms_store_private_replace_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("settings.ron");
    write_atomic(&path, "old").expect("written");
    restrict_to_owner(&path).expect("restricted");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(OPEN_TO_OTHERS)).expect("opened");
    }
    assert_eq!(mode_of(&path), OPEN_TO_OTHERS, "the fixture is a file other users can read");
    write_atomic_private(&path, "new").expect("written");
    assert_eq!(std::fs::read_to_string(&path).expect("reads"), "new");
    assert_eq!(mode_of(&path), OWNER_ONLY, "the replaced file kept the mode of the one it replaced");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn restricting_a_file_closes_it_to_other_users_and_a_missing_file_is_an_error_the_caller_sees() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("rbms_store_restrict_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder");
    let path = dir.join("settings.ron.v2.bak");
    std::fs::write(&path, "backup").expect("written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(OPEN_TO_OTHERS)).expect("opened");
    restrict_to_owner(&path).expect("restricted");
    assert_eq!(mode_of(&path), OWNER_ONLY);
    assert_eq!(restrict_to_owner(&dir.join("absent")).expect_err("nothing to restrict").kind(), std::io::ErrorKind::NotFound);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A folder closed to everyone but its owner for reading and writing cannot be entered, and a link
/// would carry the change to the file it points at: neither is a file to close.
#[cfg(unix)]
#[test]
fn restricting_leaves_a_folder_and_a_link_as_they_are() {
    use std::os::unix::fs::PermissionsExt;
    const FOLDER_MODE: u32 = 0o755;
    let dir = std::env::temp_dir().join(format!("rbms_store_restrict_kinds_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(FOLDER_MODE)).expect("mode");
    restrict_to_owner(&dir).expect("a folder is left alone");
    assert_eq!(mode_of(&dir), FOLDER_MODE, "a folder was closed");

    let target = dir.join("dotfiles-settings.ron");
    std::fs::write(&target, "kept").expect("written");
    open_to_others(&target);
    let link = dir.join("settings.ron");
    std::os::unix::fs::symlink(&target, &link).expect("linked");
    restrict_to_owner(&link).expect("a link is left alone");
    assert_eq!(mode_of(&target), OPEN_TO_OTHERS, "the file behind a link was changed");
    let _ = std::fs::remove_dir_all(&dir);
}

/// How many threads of one process write one file at once, and how many times each.
const RACING_WRITERS: usize = 8;
const WRITES_PER_WRITER: usize = 40;

/// Two threads of one process saving the same file at once each write their own temp file: the
/// target ends as one whole write, and no write fails over a temp file the other renamed away.
#[test]
fn threads_of_one_process_writing_one_file_never_share_a_temp_file() {
    let dir = std::env::temp_dir().join(format!("rbms_store_atomic_race_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("settings.ron");
    let contents: Vec<String> = (0..RACING_WRITERS).map(|writer| format!("writer {writer} ").repeat(writer + 1)).collect();
    let failures: usize = std::thread::scope(|scope| {
        let handles: Vec<_> = contents
            .iter()
            .map(|text| {
                let path = path.clone();
                scope.spawn(move || (0..WRITES_PER_WRITER).filter(|_| write_atomic(&path, text).is_err()).count())
            })
            .collect();
        handles.into_iter().map(|handle| handle.join().expect("a writer finishes")).sum()
    });
    assert_eq!(failures, 0, "no write lost its temp file to another thread");
    let written = std::fs::read_to_string(&path).expect("the target is there");
    assert!(contents.contains(&written), "the target is one whole write: {written:?}");
    assert_eq!(std::fs::read_dir(&dir).expect("lists").count(), 1, "no temp file is left behind");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A staged write shows a directory it cannot write into when it is created, and leaves nothing
/// behind when it is let go unfinished.
#[test]
fn a_staged_write_fails_where_its_file_cannot_be_created_and_cleans_up_when_dropped() {
    let dir = std::env::temp_dir().join(format!("rbms_store_staged_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder");
    let in_the_way = dir.join("replays");
    std::fs::write(&in_the_way, "a file where the folder should be").expect("written");
    assert!(StagedWrite::create(&in_the_way.join("r.ron")).is_err(), "a file in the way of the folder is seen at once");

    let target = dir.join("kept/r.ron");
    drop(StagedWrite::create(&target).expect("the folder is made"));
    assert_eq!(std::fs::read_dir(dir.join("kept")).expect("lists").count(), 0, "an unfinished write leaves no temp file");
    StagedWrite::create(&target).expect("staged").finish(b"whole").expect("finished");
    assert_eq!(std::fs::read_to_string(&target).expect("reads"), "whole");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_book_forgets_a_replay_that_is_gone() {
    let mut book = ScoreBook::default();
    book.push(ScoreRecord { replay_file: Some("gone.ron".into()), ..rec("AA", 1, 5) });
    book.push(ScoreRecord { replay_file: Some("kept.ron".into()), ..rec("AA", 2, 5) });
    assert_eq!(book.forget_replay("gone.ron"), 1);
    let files: Vec<Option<&str>> = book.for_md5("AA").iter().map(|record| record.replay_file.as_deref()).collect();
    assert_eq!(files, [Some("kept.ron"), None]);
}

#[test]
fn the_serialized_book_carries_only_the_record_list() {
    let mut book = ScoreBook::default();
    book.push(rec("AA", 1, 5));
    let s = ron::ser::to_string_pretty(&book, ron::ser::PrettyConfig::default()).unwrap().replace("\r\n", "\n");
    assert!(s.trim_start().starts_with("(\n    records: ["), "the file shape is unchanged: {s}");
    assert!(!s.contains("index"), "the md5 lookup index never reaches the file: {s}");
}

#[test]
fn the_best_with_a_replay_is_the_oldest_run_of_the_best_score() {
    let with_replay = |played_at: i64, ex: u32, file: Option<&str>| ScoreRecord { replay_file: file.map(str::to_string), ..rec("AA", played_at, ex) };
    let mut book = ScoreBook::default();
    book.push(with_replay(1, 30, Some("weaker.ron")));
    book.push(with_replay(2, 40, None));
    book.push(ScoreRecord { assisted: true, ..with_replay(3, 40, Some("assisted.ron")) });
    book.push(ScoreRecord { ln_mode: "CN".into(), ..with_replay(4, 40, Some("charge.ron")) });
    book.push(with_replay(5, 40, Some("second.ron")));
    book.push(with_replay(6, 40, Some("third.ron")));
    let best = book.best_record_with_replay_in_ln_mode("aa", SCORE_LN_MODE_FROM_CHART).and_then(|r| r.replay_file.as_deref());
    assert_eq!(best, Some("second.ron"), "the first run of the best score kept no replay, so the next one of the same score stands in");
    assert_eq!(book.best_record_with_replay_in_ln_mode("AA", "CN").and_then(|r| r.replay_file.as_deref()), Some("charge.ron"));
    assert!(book.best_record_with_replay_in_ln_mode("BB", SCORE_LN_MODE_FROM_CHART).is_none());
    let mut no_replay = ScoreBook::default();
    no_replay.push(with_replay(1, 30, Some("weaker.ron")));
    no_replay.push(with_replay(2, 40, None));
    assert!(no_replay.best_record_with_replay_in_ln_mode("AA", SCORE_LN_MODE_FROM_CHART).is_none(), "a weaker run's replay is no stand-in for the best");
}
