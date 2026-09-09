//! A `scores.ron` written before the store crate existed must keep loading unchanged.

use std::path::PathBuf;

use rbms_store::{ScoreBook, is_stale_rule_version};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scores-legacy.ron")
}

#[test]
fn a_pre_migration_scores_file_loads_with_every_record() {
    let book = ScoreBook::load(&fixture());
    assert_eq!(book.records().len(), 3, "every legacy record survives the load");
    assert_eq!(book.records()[0].title, "Legacy Chart One");
    assert_eq!(book.records()[2].mode, "BEAT-14K");
}

#[test]
fn legacy_records_default_the_fields_added_after_they_were_written() {
    let book = ScoreBook::load(&fixture());
    for r in book.records() {
        assert_eq!(r.empty_poor, 0, "empty_poor defaults to 0");
        assert_eq!(r.replay_file, None, "replay_file defaults to None");
        assert_eq!(r.rule_version, 0, "a pre-versioning record reads as rule version 0");
        assert!(!r.assisted, "assisted defaults to false");
        assert!(is_stale_rule_version(r.rule_version), "legacy records are judged under an older rule");
    }
}

#[test]
fn queries_on_a_loaded_legacy_file_match_the_pre_migration_results() {
    let book = ScoreBook::load(&fixture());
    let one = book.for_md5("0123456789abcdef0123456789ABCDEF");
    assert_eq!(one.len(), 2, "both plays of the chart match regardless of md5 case");
    assert_eq!(one[0].played_at, 1700003600, "newest first");
    assert_eq!(one[1].played_at, 1700000000);
    assert_eq!(book.best_ex_for_md5("0123456789ABCDEF0123456789ABCDEF"), Some(1350));
    assert_eq!(book.best_clear_for_md5("0123456789ABCDEF0123456789ABCDEF"), Some(6));
    assert_eq!(book.for_md5("fedcba9876543210fedcba9876543210").len(), 1);
    assert_eq!(book.best_ex_for_md5("FEDCBA9876543210FEDCBA9876543210"), Some(300));
    assert_eq!(book.best_ex_for_md5("no-such-chart"), None);
}

#[test]
fn loading_a_legacy_file_does_not_rewrite_or_back_it_up() {
    let before = std::fs::read_to_string(fixture()).expect("the fixture is committed");
    let _ = ScoreBook::load(&fixture());
    assert_eq!(std::fs::read_to_string(fixture()).unwrap(), before, "a successful load leaves the file untouched");
    assert!(!fixture().with_extension("ron.bak").exists(), "no backup is written for a file that parsed");
}
