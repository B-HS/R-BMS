use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::StoreError;

/// Version of the judging rules a record was produced under. Bumped whenever a rule change makes
/// older records not directly comparable; records written before the field existed default to 0.
pub const SCORE_RULE_VERSION: u32 = 1;

/// Whether a record was judged under a rule version this build no longer produces, so what is shown
/// next to it is the presenting layer's decision rather than this crate's.
pub fn is_stale_rule_version(rule_version: u32) -> bool {
    rule_version < SCORE_RULE_VERSION
}

/// One persisted play result, kept locally so play history and replays survive without a score
/// server. `clear` is the reference implementation's `ClearType` id (0..10) so lamps round-trip; `replay_file` is
/// the basename (under `replays/`) of the saved replay, when one was recorded.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// Wire shape of `scores.ron`: the record list and nothing else, so the lookup index below never
/// reaches the file and old files keep parsing byte for byte.
#[derive(Default, Deserialize)]
#[serde(default)]
struct ScoreRecords {
    records: Vec<ScoreRecord>,
}

impl From<ScoreRecords> for ScoreBook {
    fn from(wire: ScoreRecords) -> ScoreBook {
        ScoreBook::from_records(wire.records)
    }
}

/// All local play records. Append-only in practice; queried by chart md5 for the select screen.
///
/// The record list is read through [`ScoreBook::records`] and only ever grown through
/// [`ScoreBook::push`], so the md5 lookup index cannot fall out of step with it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(from = "ScoreRecords")]
pub struct ScoreBook {
    records: Vec<ScoreRecord>,
    /// Lowercased md5 to the positions in `records` that carry it, so a per-chart query costs one
    /// hash lookup instead of a scan of the whole book on every select-screen row.
    #[serde(skip)]
    index: HashMap<String, Vec<usize>>,
}

impl ScoreBook {
    /// Build a book from records already in memory, indexing them as it goes.
    pub fn from_records(records: Vec<ScoreRecord>) -> ScoreBook {
        let mut book = ScoreBook { records, index: HashMap::new() };
        book.rebuild_index();
        book
    }

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

    /// Serialize and durably replace `path`, reporting why on failure.
    pub fn try_save(&self, path: &Path) -> Result<(), StoreError> {
        let s = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        write_atomic(path, &s).map_err(StoreError::Write)
    }

    /// [`ScoreBook::try_save`] for the fire-and-forget call sites: a failed save is reported on
    /// stderr and never interrupts play.
    pub fn save(&self, path: &Path) {
        match self.try_save(path) {
            Ok(()) => {}
            Err(StoreError::Write(e)) => eprintln!("scores write failed ({}): {e}", path.display()),
            Err(e) => eprintln!("scores save failed: {e}"),
        }
    }

    /// Every record in the book, in insertion order.
    pub fn records(&self) -> &[ScoreRecord] {
        &self.records
    }

    pub fn push(&mut self, record: ScoreRecord) {
        self.index.entry(record.md5.to_ascii_lowercase()).or_default().push(self.records.len());
        self.records.push(record);
    }

    /// Rebuild the md5 lookup index from the record list. Called after a load.
    pub fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, r) in self.records.iter().enumerate() {
            self.index.entry(r.md5.to_ascii_lowercase()).or_default().push(i);
        }
    }

    /// Positions in `records` holding a chart's plays, in insertion order.
    fn positions(&self, md5: &str) -> &[usize] {
        self.index.get(&md5.to_ascii_lowercase()).map(Vec::as_slice).unwrap_or(&[])
    }

    /// All records for a chart, newest first.
    pub fn for_md5(&self, md5: &str) -> Vec<&ScoreRecord> {
        let mut v: Vec<&ScoreRecord> = self.positions(md5).iter().map(|&i| &self.records[i]).collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.played_at));
        v
    }

    /// Records for a chart that may set its bests: assisted runs are history only.
    fn scoring_records(&self, md5: &str) -> impl Iterator<Item = &ScoreRecord> {
        self.positions(md5).iter().map(|&i| &self.records[i]).filter(|r| !r.assisted)
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
