use std::path::Path;

use serde::{Deserialize, Serialize};

/// Version of the judging rules a record was produced under. Bumped whenever a rule change makes
/// older records not directly comparable; records written before the field existed default to 0.
pub const SCORE_RULE_VERSION: u32 = 1;

/// Short marker appended to a record's timestamp when it was judged under an older rule version.
pub fn rule_version_mark(rule_version: u32) -> &'static str {
    if rule_version < SCORE_RULE_VERSION { " *" } else { "" }
}

/// Spelled-out counterpart of [`rule_version_mark`] for the record-detail modal.
pub fn rule_version_note(rule_version: u32) -> &'static str {
    if rule_version < SCORE_RULE_VERSION { "   OLD RULE" } else { "" }
}

/// One persisted play result, kept locally so play history and replays survive without a score
/// server. `clear` is the reference implementation's `ClearType` id (0..10) so lamps round-trip; `replay_file` is
/// the basename (under `replays/`) of the saved replay, when one was recorded.
#[derive(Clone, Serialize, Deserialize)]
pub struct ScoreRecord {
    pub md5: String,
    pub title: String,
    pub mode: String,
    pub clear: u8,
    pub ex_score: u32,
    pub max_ex: u32,
    pub counts: [u32; 6],
    #[serde(default)]
    pub empty_poor: u32,
    pub max_combo: u32,
    pub total_notes: u32,
    pub gauge: String,
    pub gauge_value: f32,
    pub random: String,
    pub played_at: i64,
    #[serde(default)]
    pub replay_file: Option<String>,
    /// Judging-rule version this record was produced under (see [`SCORE_RULE_VERSION`]).
    #[serde(default)]
    pub rule_version: u32,
    /// The run used an assist (widened judge window or an auto-played lane), so it is kept as
    /// history but excluded from the stored bests — the reference implementation clears the same `score` flag and
    /// gates exscore/minbp/combo on it (`ScoreData.java:548,566,572,578`). Records written before
    /// the field existed default to `false`.
    #[serde(default)]
    pub assisted: bool,
}

/// All local play records. Append-only in practice; queried by chart md5 for the select screen.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoreBook {
    pub records: Vec<ScoreRecord>,
}

impl ScoreBook {
    pub fn load(path: &Path) -> ScoreBook {
        match std::fs::read_to_string(path) {
            Ok(s) => ron::from_str(&s).unwrap_or_else(|e| {
                let backup = path.with_extension("ron.bak");
                let _ = std::fs::rename(path, &backup);
                eprintln!("scores parse failed ({e}); backed up to {} and starting empty", backup.display());
                ScoreBook::default()
            }),
            Err(_) => ScoreBook::default(),
        }
    }

    pub fn save(&self, path: &Path) {
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => {
                if let Err(e) = crate::write_atomic(path, &s) {
                    eprintln!("scores write failed ({}): {e}", path.display());
                }
            }
            Err(e) => eprintln!("scores save failed: {e}"),
        }
    }

    pub fn push(&mut self, record: ScoreRecord) {
        self.records.push(record);
    }

    /// All records for a chart, newest first.
    pub fn for_md5(&self, md5: &str) -> Vec<&ScoreRecord> {
        let mut v: Vec<&ScoreRecord> = self.records.iter().filter(|r| r.md5.eq_ignore_ascii_case(md5)).collect();
        v.sort_by(|a, b| b.played_at.cmp(&a.played_at));
        v
    }

    /// Records for a chart that may set its bests: assisted runs are history only.
    fn scoring_records(&self, md5: &str) -> impl Iterator<Item = &ScoreRecord> {
        self.records.iter().filter(move |r| !r.assisted && r.md5.eq_ignore_ascii_case(md5))
    }

    /// Best EX on a chart, folded without the allocate-and-sort of `for_md5` (called every frame for
    /// the live score graph). `None` if the chart has no unassisted records.
    pub fn best_ex_for_md5(&self, md5: &str) -> Option<u32> {
        self.scoring_records(md5).map(|r| r.ex_score).max()
    }

    /// Best clear-lamp id on a chart (highest `ClearType` id), folded without allocation — used for
    /// the per-row clear-lamp LED in the select list. `None` if the chart has no unassisted records.
    /// The reference implementation updates the lamp for assisted runs too, but only after demoting it to
    /// `AssistEasy`/`LightAssistEasy` (`BMSPlayer.java:866`); rbms cannot demote the lamp yet, so an
    /// assisted run must not raise the LED at all.
    pub fn best_clear_for_md5(&self, md5: &str) -> Option<u8> {
        self.scoring_records(md5).map(|r| r.clear).max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(back.records.len(), 1);
        assert_eq!(back.records[0].ex_score, 42);
    }

    // --- for_md5 edge cases ---

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
        // Equal timestamps: sort is by played_at only; both records must still be present.
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

    // --- best_ex / best_clear ---

    #[test]
    fn best_ex_picks_max_case_insensitively() {
        let mut book = ScoreBook::default();
        book.push(rec("AA", 1, 30));
        book.push(rec("aa", 2, 80));
        book.push(rec("Aa", 3, 50));
        book.push(rec("BB", 4, 999)); // different chart
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
        // Invariant: the folded best_ex equals the max over the queried records.
        let mut book = ScoreBook::default();
        for (t, ex) in [(1_i64, 12_u32), (2, 80), (3, 45)] {
            book.push(rec("AA", t, ex));
        }
        let folded = book.best_ex_for_md5("AA");
        let by_query = book.for_md5("AA").iter().map(|r| r.ex_score).max();
        assert_eq!(folded, by_query, "two paths to the best EX agree");
        assert_eq!(folded, Some(80));
    }

    // --- push / count preservation ---

    #[test]
    fn push_appends_and_preserves_order_and_count() {
        let mut book = ScoreBook::default();
        assert_eq!(book.records.len(), 0);
        book.push(rec("AA", 1, 1));
        book.push(rec("BB", 2, 2));
        book.push(rec("CC", 3, 3));
        assert_eq!(book.records.len(), 3, "push grows by one each time");
        let md5s: Vec<&str> = book.records.iter().map(|r| r.md5.as_str()).collect();
        assert_eq!(md5s, ["AA", "BB", "CC"], "append-only, insertion order kept");
    }

    // --- serde defaults / back-compat ---

    #[test]
    fn empty_poor_and_replay_file_default_when_absent() {
        // A record written before empty_poor/replay_file existed still parses (serde default).
        let s = r#"(records: [(
            md5: "AA", title: "t", mode: "BEAT-7K", clear: 5, ex_score: 10, max_ex: 100,
            counts: (0, 0, 0, 0, 0, 0), max_combo: 0, total_notes: 50,
            gauge: "NORMAL", gauge_value: 80.0, random: "OFF", played_at: 1
        )])"#;
        let book: ScoreBook = ron::from_str(s).expect("back-compat record parses");
        assert_eq!(book.records.len(), 1);
        assert_eq!(book.records[0].empty_poor, 0, "empty_poor defaults to 0");
        assert_eq!(book.records[0].replay_file, None, "replay_file defaults to None");
    }

    #[test]
    fn rule_version_defaults_to_zero_for_a_pre_versioning_record() {
        let s = r#"(records: [(
            md5: "AA", title: "t", mode: "BEAT-7K", clear: 5, ex_score: 10, max_ex: 100,
            counts: (0, 0, 0, 0, 0, 0), max_combo: 0, total_notes: 50,
            gauge: "NORMAL", gauge_value: 80.0, random: "OFF", played_at: 1
        )])"#;
        let book: ScoreBook = ron::from_str(s).expect("back-compat record parses");
        assert_eq!(book.records[0].rule_version, 0);
        assert!(!book.records[0].assisted, "assisted defaults to false for a pre-versioning record");
        assert_eq!(rule_version_mark(book.records[0].rule_version), " *");
        assert_eq!(rule_version_note(book.records[0].rule_version), "   OLD RULE");
    }

    #[test]
    fn current_rule_version_records_are_not_marked() {
        assert_eq!(SCORE_RULE_VERSION, 1, "the current judging-rule generation");
        assert_eq!(rule_version_mark(SCORE_RULE_VERSION), "");
        assert_eq!(rule_version_note(SCORE_RULE_VERSION), "");
        assert_eq!(rule_version_mark(SCORE_RULE_VERSION + 1), "", "a newer record is not marked as old");
    }

    #[test]
    fn rule_version_round_trips_through_ron() {
        let mut book = ScoreBook::default();
        let mut r = rec("AA", 1, 5);
        r.rule_version = 7;
        book.push(r);
        let s = ron::ser::to_string_pretty(&book, ron::ser::PrettyConfig::default()).unwrap();
        let back: ScoreBook = ron::from_str(&s).unwrap();
        assert_eq!(back.records[0].rule_version, 7);
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
        // serde(default) on ScoreBook: empty/unit yields an empty record list.
        let book: ScoreBook = ron::from_str("()").expect("empty unit parses");
        assert!(book.records.is_empty());
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
        let b = &back.records[0];
        assert_eq!(b.counts, [1, 2, 3, 4, 5, 6]);
        assert_eq!(b.empty_poor, 9);
        assert_eq!(b.replay_file.as_deref(), Some("r/abc.ron"));
        assert_eq!(b.max_combo, 321);
        assert_eq!(b.played_at, 12345);
        assert!((b.gauge_value - 73.5).abs() < 1e-6);
    }

    // --- load() defaults on missing file ---

    #[test]
    fn load_missing_file_returns_empty_book() {
        let dir = std::env::temp_dir().join(format!("rbms_scores_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("scores.ron");
        let book = ScoreBook::load(&path);
        assert!(book.records.is_empty(), "missing scores file => empty book");
        // unlike keyconfig, scores load does NOT write a file out
        assert!(!path.exists(), "load() of a missing scores file does not create it");
    }

    #[test]
    fn save_then_load_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("rbms_scores_io_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested/scores.ron");
        let mut book = ScoreBook::default();
        book.push(rec("AA", 5, 42));
        book.save(&path); // also creates the nested dir
        assert!(path.exists(), "save creates the file (and parent dir)");
        let back = ScoreBook::load(&path);
        assert_eq!(back.records.len(), 1);
        assert_eq!(back.records[0].ex_score, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
