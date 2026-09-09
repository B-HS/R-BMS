#![forbid(unsafe_code)]

pub mod algorithm;
pub mod data;
pub mod gauge;
pub mod matcher;
pub mod windows;

pub use algorithm::{JudgeAlgorithm, NoteRef, NoteType};
pub use data::{
    GaugeModifier, GaugeParams, GaugeSet, GaugeTables, JudgeDataError, JudgePropertyData, JudgeTables, JudgeWindowsData, builtin_gauge_tables,
    builtin_judge_tables, load_gauge_tables, load_judge_tables,
};
pub use gauge::{ClearType, Gauge, GaugeKind, clear_lamp, clear_type_from_id, clear_type_id};
pub use matcher::{JudgeEngine, JudgeResult};
pub use windows::{JudgeProperty, JudgeWindows, MissCondition, judgerank_for, rank_to_judgerank};

/// A single judgment outcome, in the reference implementation's judge-code order. `Poor` is index 4 (見逃し POOR — a
/// note that went by unhit) and `Miss` is index 5 (空POOR — a press that reached only the MS band
/// and consumed no note). Combo behaviour per index comes from the mode's
/// [`JudgeProperty::combo`](windows::JudgeProperty::combo) table. EX score = 2·PG + 1·GR.
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
        assert_eq!(w.judge(300_000), Some(Judge::Miss), "the MS band is reference judge code 5 (empty poor)");
        assert_eq!(w.judge(-300_000), None);
        assert_eq!(w.judge(450_000), Some(Judge::Miss));
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
    fn swept_miss_uses_poor_slot_index_4() {
        let mut e = JudgeEngine::new(vec![vec![100_000]], JudgeWindows::SEVENKEY_NOTE);
        e.combo = 5;
        e.update(500_000);
        assert_eq!(e.counts[4], 1);
        assert_eq!(e.counts[5], 0, "nothing lands in the empty-poor slot");
        assert_eq!(e.combo, 0);
    }

    #[test]
    fn swept_miss_costs_the_poor_gauge_delta_normal_minus_6() {
        let mut e = JudgeEngine::new(vec![vec![100_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 200.0);
        e.update(500_000);
        assert_eq!(e.gauge.value(), 14.0, "20.0 - 6.0");
    }

    #[test]
    fn empty_poor_costs_the_ms_gauge_delta_normal_minus_2() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 200.0);
        e.press(0, 700_000);
        assert_eq!(e.gauge.value(), 18.0, "20.0 - 2.0");
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
        assert_eq!(e.counts[4], 1);
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
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.combo = 7;
        let r = e.press(0, 700_000).unwrap();
        assert_eq!(r.judge, Judge::Miss, "300ms-early press is an empty poor (judge code 5)");
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.counts[5], 1, "the reference implementation tallies it via addJudgeCount(5) -> ems/lms");
        assert_eq!(e.combo, 7, "SEVENKEYS combo[5] = true, so an empty poor must not break combo");
        assert_eq!(e.counts[..5], [0; 5], "empty poor does not consume a note");
        let r2 = e.press(0, 1_000_000).unwrap();
        assert_eq!(r2.judge, Judge::PerfectGreat, "the un-consumed note is still hittable");
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.combo, 8);
    }

    #[test]
    fn empty_poor_does_not_block_full_combo() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000, 1_300_000]], JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 200.0);
        e.press(0, 650_000);
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
        e.press(0, 990_000);
        e.press(0, 2_050_000);
        e.update(4_000_000);
        assert_eq!(e.early[0], 1, "early PGREAT");
        assert_eq!(e.late[1], 1, "late GREAT");
        assert_eq!(e.late[4], 1, "a swept poor counts as late");
        assert_eq!(e.early[4], 0, "there is no early swept poor in normal play");
        for i in 0..6 {
            assert_eq!(e.counts[i], e.early[i] + e.late[i], "counts[{i}] must equal early+late");
        }
        assert_eq!(e.fast, 1, "one early hit");
        assert_eq!(e.slow, 1, "one late hit (the swept miss is excluded from fast/slow)");
    }

    #[test]
    fn avg_judge_is_mean_signed_delta_of_hits() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000, 2_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 990_000);
        e.press(0, 1_994_000);
        assert_eq!(e.avg_judge_us(), 8_000, "mean of +10ms and +6ms");
    }

    use matcher::JudgeEngine as Eng;

    fn note(t: i64) -> JudgeEngine {
        Eng::new(vec![vec![t]], JudgeWindows::SEVENKEY_NOTE)
    }

    #[test]
    fn press_on_invalid_lane_returns_none() {
        let mut e = note(100_000);
        assert!(e.press(5, 100_000).is_none(), "out-of-range lane index");
        assert_eq!(e.total_judged(), 0);
    }

    #[test]
    fn press_on_empty_lane_returns_none() {
        let mut e = Eng::new(vec![vec![]], JudgeWindows::SEVENKEY_NOTE);
        assert!(e.press(0, 0).is_none());
    }

    #[test]
    fn press_matches_nearest_even_when_earlier_note_in_range() {
        let mut e = Eng::new(vec![vec![100_000, 180_000]], JudgeWindows::SEVENKEY_NOTE);
        let r = e.press(0, 170_000).unwrap();
        assert_eq!(r.note_index, 1, "nearest by |dm| wins");
        assert_eq!(r.judge, Judge::PerfectGreat);
    }

    #[test]
    fn press_too_early_beyond_ms_returns_none() {
        let mut e = note(1_000_000);
        assert!(e.press(0, 400_000).is_none(), "dm +600_000 > gate_early");
        assert_eq!(e.empty_poor, 0);
        assert_eq!(e.total_judged(), 0);
    }

    #[test]
    fn press_too_late_beyond_bd_returns_none() {
        let mut e = note(100_000);
        assert!(e.press(0, 400_000).is_none(), "dm -300_000 < gate_late");
    }

    #[test]
    fn already_judged_note_is_not_rematched_but_yields_an_empty_poor() {
        let mut e = note(100_000);
        e.press(0, 100_000).unwrap();
        let again = e.press(0, 100_000).unwrap();
        assert_eq!(again.judge, Judge::Miss, "a consumed note cannot be hit twice, the press is an empty poor");
        assert_eq!(e.counts[0], 1, "the original PGREAT stands");
        assert_eq!(e.counts[5], 1);
    }

    #[test]
    fn press_late_bad_lower_edge_is_consumed() {
        let mut e = note(100_000);
        e.combo = 4;
        let r = e.press(0, 380_000).unwrap();
        assert_eq!(r.judge, Judge::Bad);
        assert_eq!(r.delta_us, -280_000);
        assert!(!r.fast, "late press is not fast");
        assert_eq!(e.counts[3], 1);
        assert_eq!(e.combo, 0);
        assert_eq!(e.late[3], 1, "late BAD recorded as late");
    }

    #[test]
    fn press_early_bad_upper_edge_is_consumed() {
        let mut e = note(1_000_000);
        let r = e.press(0, 780_000).unwrap();
        assert_eq!(r.judge, Judge::Bad);
        assert_eq!(r.delta_us, 220_000);
        assert!(r.fast, "early press is fast");
        assert_eq!(e.early[3], 1);
        assert_eq!(e.fast, 1, "BAD is not a Miss so it counts as fast");
    }

    #[test]
    fn empty_poor_at_exact_ms_early_edge() {
        let mut e = note(1_000_000);
        let r = e.press(0, 500_000).unwrap();
        assert_eq!(r.judge, Judge::Miss);
        assert_eq!(r.delta_us, 500_000);
        assert!(r.fast);
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.counts[..5], [0; 5], "empty poor never consumes a note");
    }

    #[test]
    fn empty_poor_does_not_touch_timing_or_direction_tallies() {
        let mut e = note(1_000_000);
        e.press(0, 700_000);
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.early[5], 1, "an early empty poor feeds the IR `ems` field");
        assert_eq!(e.late[5], 0);
        assert_eq!(e.fast, 0, "empty poor excluded from fast/slow (reference implementation records only judge < 4)");
        assert_eq!(e.slow, 0);
        assert_eq!(e.avg_judge_us(), 0, "no timed hit yet");
        assert_eq!(e.last_judge, Some(Judge::Miss));
    }

    #[test]
    fn empty_poor_does_not_advance_cursor_or_block_later_sweep() {
        let mut e = note(1_000_000);
        e.press(0, 700_000);
        e.update(2_000_000);
        assert_eq!(e.counts[4], 1, "the note is still there to be swept into POOR");
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.total_judged(), 1, "only the miss is in counts");
    }

    #[test]
    fn empty_poor_consecutive_presses_accumulate() {
        let mut e = note(1_000_000);
        e.press(0, 600_000);
        e.press(0, 650_000);
        assert_eq!(e.empty_poor, 2, "each far-early mash adds an empty poor");
        assert_eq!(e.counts[5], 2);
        assert_eq!(e.counts[..5], [0; 5]);
    }

    fn ln(head: i64, end: i64) -> JudgeEngine {
        Eng::from_pairs(vec![vec![(head, Some(end))]], JudgeWindows::SEVENKEY_NOTE)
    }

    #[test]
    fn ln_head_press_starts_hold_without_scoring() {
        let mut e = ln(100_000, 600_000);
        let r = e.press(0, 100_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat, "head judge reported");
        assert_eq!(e.counts, [0; 6], "head press alone scores nothing");
        assert_eq!(e.combo, 0, "combo only advances on release");
        assert_eq!(e.ex_score, 0);
    }

    #[test]
    fn ln_release_finalizes_as_worse_of_head_and_end_pg_pg() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        let r = e.release(0, 600_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat, "worse(PG, PG) = PG");
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.combo, 1);
        assert_eq!(e.ex_score, 2);
    }

    #[test]
    fn ln_release_takes_worse_when_end_is_bad() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        let r = e.release(0, 390_000).unwrap();
        assert_eq!(r.judge, Judge::Bad, "worse(PG, BD) = BD");
        assert_eq!(e.counts[3], 1);
        assert_eq!(e.combo, 0, "a BAD release breaks combo");
    }

    #[test]
    fn ln_release_keeps_worse_head_when_end_is_perfect() {
        let mut e = ln(1_000_000, 1_600_000);
        let hr = e.press(0, 870_000).unwrap();
        assert_eq!(hr.judge, Judge::Good);
        let r = e.release(0, 1_600_000).unwrap();
        assert_eq!(r.judge, Judge::Good, "worse(GOOD, PG) = GOOD");
        assert_eq!(e.counts[2], 1);
        assert_eq!(e.combo, 1, "GOOD keeps combo");
    }

    #[test]
    fn ln_release_delta_is_end_minus_release_and_records_timing() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        let r = e.release(0, 590_000).unwrap();
        assert_eq!(r.delta_us, 10_000);
        assert!(r.fast, "released early");
        assert_eq!(e.fast, 1, "release timing counted");
        assert_eq!(e.avg_judge_us(), 10_000, "single timed hit = its own delta");
    }

    #[test]
    fn ln_release_without_hold_returns_none() {
        let mut e = ln(100_000, 600_000);
        assert!(e.release(0, 600_000).is_none());
    }

    #[test]
    fn ln_release_on_invalid_lane_returns_none() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        assert!(e.release(9, 600_000).is_none(), "no such lane");
    }

    #[test]
    fn over_held_plain_ln_takes_the_head_judgment() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        e.update(600_000);
        assert_eq!(e.total_judged(), 0, "at exactly the end the LN is still held");
        e.update(600_001);
        assert_eq!(e.total_judged(), 1, "over-held LN finalized once");
        assert_eq!(e.counts[0], 1, "finalized with the head PGREAT, not a re-judged end");
        assert_eq!(e.early[0], 1, "the commit takes the head delta's direction, not the sweep's");
        assert_eq!(e.late[0], 0);
        assert_eq!(e.combo, 1);
    }

    #[test]
    fn longnote_margin_rate_moves_the_pms_over_hold_commit() {
        use rbms_model::Mode;
        let build = |rate: i32| {
            let mut e = JudgeEngine::from_pairs(vec![vec![(100_000, Some(600_000))]], JudgeWindows::SEVENKEY_NOTE);
            e.apply_mode(&Mode::POPN_9K, 100);
            e.set_longnote_margin_rate(rate);
            e.press(0, 100_000).unwrap();
            e
        };

        let mut stock = build(100);
        stock.update(700_000);
        assert_eq!(stock.total_judged(), 0, "at end + 200ms the stock PMS margin has not elapsed");
        stock.update(800_001);
        assert_eq!(stock.total_judged(), 1, "committed once past end + 200ms");

        let mut none = build(0);
        none.update(600_001);
        assert_eq!(none.total_judged(), 1, "a 0% rate commits as soon as the end passes");
    }

    #[test]
    fn over_held_plain_ln_commit_is_late_when_the_head_was_hit_late() {
        let mut e = ln(100_000, 600_000);
        assert_eq!(e.press(0, 130_000).unwrap().judge, Judge::Great, "30ms late head is GREAT at 100%");
        e.update(600_001);
        assert_eq!(e.counts[1], 1, "committed with the head GREAT");
        assert_eq!(e.late[1], 1, "a late head makes the over-hold commit LATE");
        assert_eq!(e.early[1], 0);
    }

    #[test]
    fn ln_held_note_blocks_cursor_until_finalized() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        e.update(500_000);
        assert_eq!(e.total_judged(), 0, "still holding, not missed");
        assert_eq!(e.combo, 0);
    }

    #[test]
    fn set_ln_end_widens_release_leniency() {
        let mut e = ln(100_000, 600_000);
        let wide =
            JudgeWindows { pg: (-400_000, 400_000), gr: (-450_000, 450_000), gd: (-500_000, 500_000), bd: (-550_000, 550_000), ms: Some((-550_000, 600_000)) };
        e.set_ln_end(wide);
        e.press(0, 100_000).unwrap();
        let r = e.release(0, 370_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat, "wide ln_end keeps release PG");
        assert_eq!(e.counts[0], 1);
    }

    #[test]
    fn ln_head_empty_poor_does_not_start_hold() {
        let mut e = ln(1_000_000, 1_600_000);
        let r = e.press(0, 700_000).unwrap();
        assert_eq!(r.judge, Judge::Miss);
        assert_eq!(e.empty_poor, 1);
        assert!(e.release(0, 1_600_000).is_none(), "empty poor must not arm a hold");
        let r2 = e.press(0, 1_000_000).unwrap();
        assert_eq!(r2.judge, Judge::PerfectGreat);
    }

    #[test]
    fn update_miss_bound_is_exclusive_strict_less() {
        let mut e = note(1_000_000);
        e.update(1_280_000);
        assert_eq!(e.total_judged(), 0, "at exactly the bound the note is not yet swept (strict <)");
        e.update(1_280_001);
        assert_eq!(e.counts[4], 1, "1us further sweeps it into the 見逃し POOR slot");
    }

    #[test]
    fn update_is_idempotent_after_sweep() {
        let mut e = note(100_000);
        e.update(5_000_000);
        let counts = e.counts;
        e.update(9_000_000);
        assert_eq!(e.counts, counts, "re-running update does not double-count");
        assert_eq!(e.counts[4], 1);
    }

    #[test]
    fn update_sweeps_multiple_lanes_and_notes() {
        let mut e = Eng::new(vec![vec![100_000, 200_000], vec![300_000]], JudgeWindows::SEVENKEY_NOTE);
        e.update(10_000_000);
        assert_eq!(e.counts[4], 3, "all three notes swept into 見逃し POOR");
        assert_eq!(e.combo, 0);
        for i in 0..6 {
            assert_eq!(e.counts[i], e.early[i] + e.late[i], "counts[{i}] == early+late");
        }
        assert_eq!(e.late[4], 3, "every swept poor is late");
        assert_eq!(e.early[4], 0);
    }

    #[test]
    fn swept_miss_is_excluded_from_avg_and_fast_slow() {
        let mut e = note(100_000);
        e.update(5_000_000);
        assert_eq!(e.counts[4], 1);
        assert_eq!(e.fast, 0, "miss not fast");
        assert_eq!(e.slow, 0, "miss not slow");
        assert_eq!(e.avg_judge_us(), 0, "miss excluded from timing average");
    }

    #[test]
    fn early_plus_late_equals_counts_invariant_mixed_run() {
        let mut e = Eng::new(vec![vec![1_000_000, 2_000_000, 3_000_000, 4_000_000, 5_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 990_000);
        e.press(0, 2_010_000);
        e.press(0, 2_950_000);
        e.press(0, 4_100_000);
        e.update(9_000_000);
        let total: u32 = e.counts.iter().sum();
        assert_eq!(total, 5, "five notes resolved");
        for i in 0..6 {
            assert_eq!(e.counts[i], e.early[i] + e.late[i], "counts[{i}] == early+late");
        }
        let sum_early: u32 = e.early.iter().sum();
        let sum_late: u32 = e.late.iter().sum();
        assert_eq!(sum_early + sum_late, total, "every judged note is either early or late");
    }

    #[test]
    fn dm_zero_is_classified_as_early() {
        let mut e = note(1_000_000);
        e.press(0, 1_000_000).unwrap();
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.early[0], 1, "dm==0 counts as early");
        assert_eq!(e.late[0], 0);
        assert_eq!(e.fast, 1, "dm==0 is fast");
        assert_eq!(e.slow, 0);
    }

    #[test]
    fn replaying_same_inputs_is_deterministic() {
        let run = || {
            let mut e = Eng::new(vec![vec![1_000_000, 2_000_000, 3_000_000]], JudgeWindows::SEVENKEY_NOTE);
            e.press(0, 1_010_000);
            e.press(0, 2_005_000);
            e.update(4_000_000);
            (e.counts, e.combo, e.max_combo, e.ex_score, e.fast, e.slow, e.avg_judge_us())
        };
        assert_eq!(run(), run(), "identical inputs yield identical state");
    }

    #[test]
    fn ex_score_is_2pg_plus_1gr_only() {
        let mut e = Eng::new(vec![vec![1_000_000, 2_000_000, 3_000_000, 4_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 1_000_000);
        e.press(0, 2_050_000);
        e.press(0, 3_100_000);
        e.press(0, 4_250_000);
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.counts[1], 1);
        assert_eq!(e.counts[2], 1);
        assert_eq!(e.counts[3], 1);
        assert_eq!(e.ex_score, 3, "EX = 2*PG + 1*GR = 2 + 1");
    }

    #[test]
    fn max_combo_persists_after_break() {
        let mut e = Eng::new(vec![vec![1_000_000, 2_000_000, 3_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 1_000_000);
        e.press(0, 2_000_000);
        e.press(0, 3_250_000);
        assert_eq!(e.combo, 0);
        assert_eq!(e.max_combo, 2, "max_combo remembers the best streak");
    }

    #[test]
    fn from_model_builds_notes_and_lns_and_skips_mines() {
        use rbms_model::{LnKind, Mode, Model, ModelMeta, Note, NoteKind, TimeLine};
        let mode = Mode::BEAT_7K;
        let lanes = mode.key;
        let mk_tl = |t: i64, lane: usize, kind: NoteKind| {
            let mut tl = TimeLine::empty(lanes, t, 0.0, 130.0);
            tl.notes[lane] = Some(Note { kind, wav: 0, start_us: 0, duration_us: 0, time_us: t, section: 0.0, layered: Vec::new() });
            tl
        };
        let timelines = vec![
            mk_tl(100_000, 0, NoteKind::Normal),
            mk_tl(200_000, 0, NoteKind::LongStart { ln: LnKind::Ln }),
            mk_tl(400_000, 0, NoteKind::LongEnd { ln: LnKind::Ln }),
            mk_tl(500_000, 1, NoteKind::Mine { damage: 5.0 }),
        ];
        let model = Model {
            mode,
            meta: ModelMeta { total: 300.0, rank: 2, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines,
            md5: String::new(),
            sha256: String::new(),
        };
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.total_notes(), 2, "one normal + one LN; the Mine is not a playable note");
        let r = e.press(0, 100_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat);
        let h = e.press(0, 200_000).unwrap();
        assert_eq!(h.judge, Judge::PerfectGreat);
        let rel = e.release(0, 400_000).unwrap();
        assert_eq!(rel.judge, Judge::PerfectGreat);
        assert_eq!(e.counts[0], 2);
    }

    #[test]
    fn from_model_unterminated_long_start_is_dropped() {
        use rbms_model::{LnKind, Mode, Model, ModelMeta, Note, NoteKind, TimeLine};
        let mode = Mode::BEAT_7K;
        let lanes = mode.key;
        let mk_tl = |t: i64, kind: NoteKind| {
            let mut tl = TimeLine::empty(lanes, t, 0.0, 130.0);
            tl.notes[0] = Some(Note { kind, wav: 0, start_us: 0, duration_us: 0, time_us: t, section: 0.0, layered: Vec::new() });
            tl
        };
        let model = Model {
            mode,
            meta: ModelMeta { total: 200.0, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines: vec![mk_tl(100_000, NoteKind::LongStart { ln: LnKind::Ln })],
            md5: String::new(),
            sha256: String::new(),
        };
        let e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.total_notes(), 0, "an unterminated LongStart is not a note");
    }

    #[test]
    fn from_model_popn_uses_popn_ln_end_window() {
        use rbms_model::{LnKind, Mode, Model, ModelMeta, Note, NoteKind, TimeLine};
        let mode = Mode::POPN_9K;
        let lanes = mode.key;
        let mk_tl = |t: i64, kind: NoteKind| {
            let mut tl = TimeLine::empty(lanes, t, 0.0, 130.0);
            tl.notes[0] = Some(Note { kind, wav: 0, start_us: 0, duration_us: 0, time_us: t, section: 0.0, layered: Vec::new() });
            tl
        };
        let model = Model {
            mode,
            meta: ModelMeta { total: 200.0, rank: 3, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines: vec![mk_tl(1_000_000, NoteKind::LongStart { ln: LnKind::Ln }), mk_tl(1_600_000, NoteKind::LongEnd { ln: LnKind::Ln })],
            md5: String::new(),
            sha256: String::new(),
        };
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::POPN_NOTE);
        e.press(0, 1_000_000).unwrap();
        let r = e.release(0, 1_730_000).unwrap();
        assert_eq!(r.judge, Judge::Great, "POPN ln_end classifies 130ms-early release as GR");
    }

    fn cn_model(start_ln: rbms_model::LnKind, head_t: i64, end_t: i64) -> rbms_model::Model {
        use rbms_model::{Mode, Model, ModelMeta, Note, NoteKind, TimeLine};
        let mode = Mode::BEAT_7K;
        let lanes = mode.key;
        let mk = |t: i64, kind: NoteKind| {
            let mut tl = TimeLine::empty(lanes, t, 0.0, 130.0);
            tl.notes[0] = Some(Note { kind, wav: 0, start_us: 0, duration_us: 0, time_us: t, section: 0.0, layered: Vec::new() });
            tl
        };
        Model {
            mode,
            meta: ModelMeta { total: 300.0, rank: 2, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines: vec![mk(head_t, NoteKind::LongStart { ln: start_ln }), mk(end_t, NoteKind::LongEnd { ln: start_ln })],
            md5: String::new(),
            sha256: String::new(),
        }
    }

    #[test]
    fn cn_head_and_end_are_two_judgments() {
        use rbms_model::LnKind;
        let model = cn_model(LnKind::Cn, 1_000_000, 1_600_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.total_notes(), 2, "a CN counts as two judged objects (head + end)");
        let h = e.press(0, 1_000_000).unwrap();
        assert_eq!(h.judge, Judge::PerfectGreat);
        assert_eq!(e.counts[0], 1, "the CN head is counted immediately at press");
        assert_eq!(e.ex_score, 2, "head PG = 2 EX");
        let r = e.release(0, 1_600_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat, "exact release = PG end");
        assert_eq!(e.counts[0], 2, "head + end = two PGreat");
        assert_eq!(e.ex_score, 4);
        assert_eq!(e.total_judged(), 2);
    }

    #[test]
    fn cn_early_release_judges_end_without_capping_head() {
        use rbms_model::LnKind;
        let model = cn_model(LnKind::Cn, 1_000_000, 2_000_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat);
        assert_eq!(e.counts[0], 1, "head PG counted at press");
        let r = e.release(0, 1_500_000).unwrap();
        assert_eq!(r.judge, Judge::Poor, "early CN release = POOR end");
        assert_eq!(e.counts[0], 1, "still one PGreat (the head)");
        assert_eq!(e.counts[4], 1, "plus one POOR (the end)");
        assert_eq!(e.total_judged(), 2);
    }

    #[test]
    fn cn_never_hit_misses_head_and_end() {
        use rbms_model::LnKind;
        let model = cn_model(LnKind::Cn, 1_000_000, 1_600_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        e.update(5_000_000);
        assert_eq!(e.counts[4], 2, "a never-hit CN poors both head and end");
        assert_eq!(e.total_judged(), 2);
    }

    #[test]
    fn hcn_end_is_also_two_judgments() {
        use rbms_model::LnKind;
        let model = cn_model(LnKind::Hcn, 1_000_000, 1_600_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.total_notes(), 2);
        e.press(0, 1_000_000).unwrap();
        assert_eq!(e.counts[0], 1, "HCN head counted at press");
        e.release(0, 1_600_000).unwrap();
        assert_eq!(e.counts[0], 2, "HCN head + end");
    }

    #[test]
    fn ln_remains_single_judgment_worse_of_head_end() {
        use rbms_model::LnKind;
        let model = cn_model(LnKind::Ln, 1_000_000, 2_000_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.total_notes(), 1, "a plain LN is a single judged object");
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat);
        assert_eq!(e.counts[0], 0, "LN head is not counted until release");
        let r = e.release(0, 1_500_000).unwrap();
        assert_eq!(r.judge, Judge::Poor);
        assert_eq!(e.total_judged(), 1, "still one judgment for the LN");
    }

    fn lane_model(mode: rbms_model::Mode, notes: Vec<(usize, i64, rbms_model::NoteKind)>) -> rbms_model::Model {
        use rbms_model::{Model, ModelMeta, Note, TimeLine};
        let lanes = mode.key;
        let timelines = notes
            .into_iter()
            .map(|(lane, t, kind)| {
                let mut tl = TimeLine::empty(lanes, t, 0.0, 130.0);
                tl.notes[lane] = Some(Note { kind, wav: 0, start_us: 0, duration_us: 0, time_us: t, section: 0.0, layered: Vec::new() });
                tl
            })
            .collect();
        Model {
            mode,
            meta: ModelMeta { total: 300.0, rank: 3, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines,
            md5: String::new(),
            sha256: String::new(),
        }
    }

    #[test]
    fn empty_poor_keeps_combo_on_seven_keys_but_breaks_it_on_five() {
        use rbms_model::{Mode, NoteKind};
        let seven = lane_model(Mode::BEAT_7K, vec![(0, 1_000_000, NoteKind::Normal)]);
        let mut e7 = JudgeEngine::from_model(&seven, JudgeWindows::SEVENKEY_NOTE);
        e7.combo = 5;
        e7.press(0, 700_000);
        assert_eq!(e7.counts[5], 1, "7K empty poor tallied");
        assert_eq!(e7.combo, 5, "7K keeps the combo through an empty poor");

        let five = lane_model(Mode::BEAT_5K, vec![(0, 1_000_000, NoteKind::Normal)]);
        let mut e5 = JudgeEngine::from_model_for_mode(&five);
        e5.combo = 5;
        e5.press(0, 700_000);
        assert_eq!(e5.counts[5], 1, "5K empty poor tallied");
        assert_eq!(e5.combo, 0, "5K combo[5] is false, so the combo breaks");
    }

    #[test]
    fn from_model_for_mode_uses_the_five_key_note_table_not_the_seven_key_one() {
        use rbms_model::{Mode, NoteKind};
        let five = lane_model(Mode::BEAT_5K, vec![(0, 1_000_000, NoteKind::Normal)]);
        let mut e5 = JudgeEngine::from_model_for_mode(&five);
        assert_eq!(e5.press(0, 880_000).unwrap().judge, Judge::Bad, "120ms early is past the 5K GOOD edge");

        let seven = lane_model(Mode::BEAT_7K, vec![(0, 1_000_000, NoteKind::Normal)]);
        let mut e7 = JudgeEngine::from_model_for_mode(&seven);
        assert_eq!(e7.press(0, 880_000).unwrap().judge, Judge::Good, "the same press is GOOD on 7K");
    }

    #[test]
    fn scratch_lane_uses_the_wider_scratch_window() {
        use rbms_model::{Mode, NoteKind};
        let m = lane_model(Mode::BEAT_7K, vec![(0, 1_000_000, NoteKind::Normal), (7, 2_000_000, NoteKind::Normal)]);
        let mut e = JudgeEngine::from_model(&m, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.press(0, 975_000).unwrap().judge, Judge::Great, "25ms off on a key lane is GREAT");
        assert_eq!(e.press(7, 1_975_000).unwrap().judge, Judge::PerfectGreat, "25ms off on the scratch lane is still PGREAT");
    }

    #[test]
    fn mine_damages_the_gauge_only_while_the_lane_is_held() {
        use rbms_model::{Mode, NoteKind};
        let m = lane_model(Mode::BEAT_7K, vec![(0, 1_000_000, NoteKind::Mine { damage: 5.0 })]);
        let mut held = JudgeEngine::from_model(&m, JudgeWindows::SEVENKEY_NOTE);
        held.set_gauge(GaugeKind::Normal, 300.0);
        held.press(0, 900_000);
        held.update(1_000_000);
        assert_eq!(held.gauge.value(), 15.0, "20.0 - 5.0 mine damage");
        assert_eq!(held.counts, [0; 6], "a mine is never a judgment");

        let mut idle = JudgeEngine::from_model(&m, JudgeWindows::SEVENKEY_NOTE);
        idle.set_gauge(GaugeKind::Normal, 300.0);
        idle.update(1_000_000);
        assert_eq!(idle.gauge.value(), 20.0, "an un-held lane takes no mine damage");
    }

    #[test]
    fn mine_damage_stops_once_the_key_is_released() {
        use rbms_model::{Mode, NoteKind};
        let m = lane_model(Mode::BEAT_7K, vec![(0, 1_000_000, NoteKind::Mine { damage: 5.0 })]);
        let mut e = JudgeEngine::from_model(&m, JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Normal, 300.0);
        e.press(0, 800_000);
        e.release(0, 900_000);
        e.update(1_000_000);
        assert_eq!(e.gauge.value(), 20.0, "released before the mine, so no damage");
    }

    #[test]
    fn mine_damage_is_frozen_on_a_dead_gauge() {
        use rbms_model::{Mode, NoteKind};
        let m = lane_model(Mode::BEAT_7K, vec![(0, 1_000_000, NoteKind::Mine { damage: 500.0 })]);
        let mut e = JudgeEngine::from_model(&m, JudgeWindows::SEVENKEY_NOTE);
        e.set_gauge(GaugeKind::Hard, 300.0);
        e.press(0, 900_000);
        e.update(1_000_000);
        assert_eq!(e.gauge.value(), 0.0, "damage beyond the floor clamps to min 0");
        e.gauge.add_value(-10.0);
        assert_eq!(e.gauge.value(), 0.0, "a dead gauge does not move again");
    }

    #[test]
    fn rehitting_a_resolved_note_inside_the_ms_band_is_an_empty_poor() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat);
        let r = e.press(0, 700_000).unwrap();
        assert_eq!(r.judge, Judge::Miss, "re-hit inside the MS band is an empty poor");
        assert_eq!(e.counts[5], 1);
        assert_eq!(e.counts[0], 1, "the original PGREAT is untouched");
    }

    #[test]
    fn rehitting_a_resolved_note_is_an_empty_poor_even_after_update_moved_the_cursor() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat);
        e.update(1_000_000);
        let r = e.press(0, 1_150_000).unwrap();
        assert_eq!(r.judge, Judge::Miss, "150ms late is the MS lower bound, still an empty poor");
        assert_eq!(e.counts[5], 1);
        assert_eq!(e.counts[0], 1, "the original PGREAT is untouched");
        assert_eq!(e.late[5], 1, "dm = -150000 is LATE");
    }

    #[test]
    fn a_resolved_note_stops_being_a_rehit_candidate_once_it_leaves_the_gate() {
        let mut e = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat);
        e.update(2_000_000);
        assert!(e.press(0, 1_400_000).is_none(), "400ms late is outside the MS band, so no empty poor");
        assert_eq!(e.counts[5], 0);
    }

    #[test]
    fn held_cn_whose_end_passes_the_bad_bound_becomes_poor() {
        use rbms_model::LnKind;
        let model = cn_model(LnKind::Cn, 1_000_000, 1_600_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat, "head counted at press");
        e.update(1_700_000);
        assert_eq!(e.total_judged(), 1, "100ms past the end the CN is still releasable");
        e.update(5_000_000);
        assert_eq!(e.counts[4], 1, "the unreleased CN end became a 見逃し POOR");
        assert_eq!(e.total_judged(), 2);
    }
}
