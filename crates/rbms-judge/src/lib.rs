pub mod gauge;
pub mod matcher;
pub mod windows;

pub use gauge::{ClearType, Gauge, GaugeKind, clear_lamp};
pub use matcher::{JudgeEngine, JudgeResult};
pub use windows::{JudgeWindows, rank_to_judgerank};

/// A single judgment outcome. PG/GR/GD keep combo; BD/POOR/MISS break it. EX score =
/// 2·PG + 1·GR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judge {
    PerfectGreat,
    Great,
    Good,
    Bad,
    Poor,
    Miss,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_classification_7k() {
        let w = JudgeWindows::SEVENKEY_NOTE;
        assert_eq!(w.judge(0), Some(Judge::PerfectGreat));
        assert_eq!(w.judge(20_000), Some(Judge::PerfectGreat));
        assert_eq!(w.judge(21_000), Some(Judge::Great));
        assert_eq!(w.judge(-60_000), Some(Judge::Great));
        assert_eq!(w.judge(120_000), Some(Judge::Good));
        assert_eq!(w.judge(-260_000), Some(Judge::Bad));
        assert_eq!(w.judge(300_000), Some(Judge::Poor));
        assert_eq!(w.judge(-300_000), None);
        assert_eq!(w.judge(450_000), Some(Judge::Poor));
    }

    #[test]
    fn early_is_fast_positive_delta() {
        let w = JudgeWindows::SEVENKEY_NOTE;
        assert_eq!(w.judge(-280_000), Some(Judge::Bad));
        assert_eq!(w.judge(-281_000), None);
        assert_eq!(w.judge(220_000), Some(Judge::Bad));
    }

    #[test]
    fn rank_scaling_tightens_windows() {
        let w = JudgeWindows::SEVENKEY_NOTE.scaled(50);
        assert_eq!(w.pg, (-10_000, 10_000));
        assert_eq!(w.judge(15_000), Some(Judge::Great));
        assert_eq!(w.ms, JudgeWindows::SEVENKEY_NOTE.ms);
    }

    #[test]
    fn rank_index_mapping() {
        assert_eq!(rank_to_judgerank(2), 75);
        assert_eq!(rank_to_judgerank(3), 100);
        assert_eq!(rank_to_judgerank(0), 25);
    }

    #[test]
    fn perfect_press_scores_and_combos() {
        let mut e = JudgeEngine::new(vec![vec![100_000, 200_000]], JudgeWindows::SEVENKEY_NOTE);
        let r = e.press(0, 100_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat);
        assert_eq!(e.ex_score, 2);
        assert_eq!(e.combo, 1);
        let r2 = e.press(0, 200_000 + 50_000).unwrap();
        assert_eq!(r2.judge, Judge::Great);
        assert!(!r2.fast);
        assert_eq!(e.ex_score, 3);
        assert_eq!(e.combo, 2);
        assert_eq!(e.max_combo, 2);
    }

    #[test]
    fn nearest_note_is_matched() {
        let mut e = JudgeEngine::new(vec![vec![100_000, 130_000]], JudgeWindows::SEVENKEY_NOTE);
        let r = e.press(0, 128_000).unwrap();
        assert_eq!(r.note_index, 1);
        assert_eq!(r.judge, Judge::PerfectGreat);
    }

    #[test]
    fn passed_note_becomes_miss_and_breaks_combo() {
        let mut e = JudgeEngine::new(vec![vec![100_000]], JudgeWindows::SEVENKEY_NOTE);
        e.combo = 5;
        e.update(500_000);
        assert_eq!(e.counts[5], 1);
        assert_eq!(e.combo, 0);
    }

    #[test]
    fn press_with_no_note_in_range_returns_none() {
        let mut e = JudgeEngine::new(vec![vec![5_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert!(e.press(0, 100_000).is_none());
        assert_eq!(e.total_judged(), 0);
    }

    #[test]
    fn normal_gauge_all_pg_reaches_max_and_clears() {
        let mut e = JudgeEngine::new(vec![vec![100_000, 200_000, 300_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 200.0);
        e.press(0, 100_000);
        e.press(0, 200_000);
        e.press(0, 300_000);
        assert!(e.gauge.value() >= 80.0, "gauge should rise past clear border, got {}", e.gauge.value());
        assert!(e.gauge.is_cleared());
        assert_eq!(e.clear_lamp(), ClearType::Max);
    }

    #[test]
    fn hard_gauge_drains_on_miss() {
        let mut e = JudgeEngine::new(vec![vec![100_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Hard, 300.0);
        let before = e.gauge.value();
        e.update(1_000_000);
        assert!(e.gauge.value() < before, "hard gauge must drop on miss");
        assert_eq!(e.counts[5], 1);
    }

    #[test]
    fn failed_when_gauge_below_border() {
        let mut e = JudgeEngine::new(vec![vec![100_000, 200_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 200.0);
        e.update(5_000_000);
        assert_eq!(e.clear_lamp(), ClearType::Failed);
    }

    #[test]
    fn bad_breaks_combo() {
        let mut e = JudgeEngine::new(vec![vec![100_000]], JudgeWindows::SEVENKEY_NOTE);
        e.combo = 10;
        let r = e.press(0, 100_000 + 250_000).unwrap();
        assert_eq!(r.judge, Judge::Bad);
        assert_eq!(e.combo, 0);
    }

    #[test]
    fn early_empty_poor_keeps_note_hittable_and_combo() {
        // beatoraja SEVENKEYS: a press 300ms early lands only in the MS window (beyond BAD), an
        // empty poor (judge 5). It must NOT consume the note nor break combo — the note stays
        // hittable and a later well-timed press scores it.
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.combo = 7;
        let r = e.press(0, 700_000).unwrap();
        assert_eq!(r.judge, Judge::Poor, "300ms-early press is an empty poor");
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.combo, 7, "empty poor must not break combo");
        assert_eq!(e.counts, [0; 6], "empty poor does not consume a note");
        let r2 = e.press(0, 1_000_000).unwrap();
        assert_eq!(r2.judge, Judge::PerfectGreat, "the un-consumed note is still hittable");
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.combo, 8);
    }

    #[test]
    fn empty_poor_does_not_block_full_combo() {
        // An early mash before the note (empty poor) then a clean hit on every note must still
        // be a full combo: max_combo == total_notes and no consumed BD/PR/MS.
        let mut e = JudgeEngine::new(vec![vec![1_000_000, 1_300_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 200.0);
        e.press(0, 650_000); // 350ms early -> empty poor on note 1
        e.press(0, 1_000_000);
        e.press(0, 1_300_000);
        assert_eq!(e.counts[0], 2, "both notes PGREAT");
        assert_eq!(e.max_combo, 2);
        assert!(e.empty_poor >= 1);
        assert_eq!(e.clear_lamp(), ClearType::Max, "empty poors do not break the full combo");
    }

    #[test]
    fn empty_poor_drains_gauge() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Hard, 300.0);
        let before = e.gauge.value();
        e.press(0, 700_000);
        assert!(e.gauge.value() < before, "empty poor applies the MS gauge penalty");
    }

    #[test]
    fn early_late_split_tracks_direction_and_sums_to_counts() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000, 2_000_000, 3_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 990_000); // 10ms early -> early PGREAT
        e.press(0, 2_050_000); // 50ms late -> late GREAT
        e.update(4_000_000); // note 3 swept -> late MISS
        assert_eq!(e.early[0], 1, "early PGREAT");
        assert_eq!(e.late[1], 1, "late GREAT");
        assert_eq!(e.late[5], 1, "a swept miss counts as late");
        assert_eq!(e.early[5], 0, "there is no early miss in normal play");
        for i in 0..6 {
            assert_eq!(e.counts[i], e.early[i] + e.late[i], "counts[{i}] must equal early+late");
        }
        assert_eq!(e.fast, 1, "one early hit");
        assert_eq!(e.slow, 1, "one late hit (the swept miss is excluded from fast/slow)");
    }

    #[test]
    fn avg_judge_is_mean_signed_delta_of_hits() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000, 2_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 990_000); // dm +10_000
        e.press(0, 1_994_000); // dm +6_000
        assert_eq!(e.avg_judge_us(), 8_000, "mean of +10ms and +6ms");
    }
}
