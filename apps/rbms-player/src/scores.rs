use std::path::Path;

use serde::{Deserialize, Serialize};

/// One persisted play result, kept locally so play history and replays survive without a score
/// server. `clear` is the beatoraja `ClearType` id (0..10) so lamps round-trip; `replay_file` is
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
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("scores dir create failed ({}): {e}", dir.display());
                return;
            }
        }
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => {
                if let Err(e) = std::fs::write(path, &s) {
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

    /// Best EX on a chart, folded without the allocate-and-sort of `for_md5` (called every frame for
    /// the live score graph). `None` if the chart has no records.
    pub fn best_ex_for_md5(&self, md5: &str) -> Option<u32> {
        self.records.iter().filter(|r| r.md5.eq_ignore_ascii_case(md5)).map(|r| r.ex_score).max()
    }

    /// Best clear-lamp id on a chart (highest `ClearType` id), folded without allocation — used for
    /// the per-row clear-lamp LED in the select list. `None` if the chart has no records.
    pub fn best_clear_for_md5(&self, md5: &str) -> Option<u8> {
        self.records.iter().filter(|r| r.md5.eq_ignore_ascii_case(md5)).map(|r| r.clear).max()
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
}
