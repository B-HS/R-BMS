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

    // =====================================================================
    // Additional edge-case coverage
    // =====================================================================

    use matcher::JudgeEngine as Eng;

    fn note(t: i64) -> JudgeEngine {
        Eng::new(vec![vec![t]], JudgeWindows::SEVENKEY_NOTE)
    }

    // --- press matching / gating -------------------------------------------

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
        // Two notes both reachable; press lands closer to the second -> matches index 1.
        let mut e = Eng::new(vec![vec![100_000, 180_000]], JudgeWindows::SEVENKEY_NOTE);
        let r = e.press(0, 170_000).unwrap(); // dm to n0 = -70_000, to n1 = +10_000
        assert_eq!(r.note_index, 1, "nearest by |dm| wins");
        assert_eq!(r.judge, Judge::PerfectGreat);
    }

    #[test]
    fn press_too_early_beyond_ms_returns_none() {
        // Gate early bound is ms.1 = +500_000. A press 600ms before the note finds no candidate.
        let mut e = note(1_000_000);
        assert!(e.press(0, 400_000).is_none(), "dm +600_000 > gate_early");
        assert_eq!(e.empty_poor, 0);
        assert_eq!(e.total_judged(), 0);
    }

    #[test]
    fn press_too_late_beyond_bd_returns_none() {
        // Gate late bound is bd.0 = -280_000. A press 300ms after the note finds no candidate.
        let mut e = note(100_000);
        assert!(e.press(0, 400_000).is_none(), "dm -300_000 < gate_late");
    }

    #[test]
    fn already_judged_note_is_not_rematched() {
        let mut e = note(100_000);
        e.press(0, 100_000).unwrap();
        assert!(e.press(0, 100_000).is_none(), "a consumed note cannot be hit twice");
        assert_eq!(e.counts[0], 1);
    }

    // --- BD / late edges via press -----------------------------------------

    #[test]
    fn press_late_bad_lower_edge_is_consumed() {
        // dm exactly -280_000 (late BD edge) is a real BAD: consumes the note, breaks combo.
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
        // dm exactly +220_000 (early BD edge) is a real BAD that consumes the note.
        let mut e = note(1_000_000);
        let r = e.press(0, 780_000).unwrap();
        assert_eq!(r.judge, Judge::Bad);
        assert_eq!(r.delta_us, 220_000);
        assert!(r.fast, "early press is fast");
        assert_eq!(e.early[3], 1);
        assert_eq!(e.fast, 1, "BAD is not a Miss so it counts as fast");
    }

    // --- empty poor invariants ---------------------------------------------

    #[test]
    fn empty_poor_at_exact_ms_early_edge() {
        // dm exactly +500_000 is the far edge of the MS window -> empty poor.
        let mut e = note(1_000_000);
        let r = e.press(0, 500_000).unwrap();
        assert_eq!(r.judge, Judge::Poor);
        assert_eq!(r.delta_us, 500_000);
        assert!(r.fast);
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.counts, [0; 6], "empty poor never touches counts");
    }

    #[test]
    fn empty_poor_does_not_touch_timing_or_direction_tallies() {
        let mut e = note(1_000_000);
        e.press(0, 700_000); // empty poor, dm +300_000
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.early, [0; 6], "empty poor not split early/late");
        assert_eq!(e.late, [0; 6]);
        assert_eq!(e.fast, 0, "empty poor excluded from fast/slow");
        assert_eq!(e.slow, 0);
        assert_eq!(e.avg_judge_us(), 0, "no timed hit yet");
        assert_eq!(e.last_judge, Some(Judge::Poor));
    }

    #[test]
    fn empty_poor_does_not_advance_cursor_or_block_later_sweep() {
        // After an empty poor, sweeping past the note must still produce exactly one MISS.
        let mut e = note(1_000_000);
        e.press(0, 700_000); // empty poor
        e.update(2_000_000); // sweep the still-unjudged note
        assert_eq!(e.counts[5], 1, "the note is still there to be missed");
        assert_eq!(e.empty_poor, 1);
        assert_eq!(e.total_judged(), 1, "only the miss is in counts");
    }

    #[test]
    fn empty_poor_consecutive_presses_accumulate() {
        let mut e = note(1_000_000);
        e.press(0, 600_000); // dm +400_000 empty poor
        e.press(0, 650_000); // dm +350_000 empty poor
        assert_eq!(e.empty_poor, 2, "each far-early mash adds an empty poor");
        assert_eq!(e.counts, [0; 6]);
    }

    // --- LN: head / release / final = worse(head, end) ---------------------

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
        e.press(0, 100_000).unwrap(); // head PG
        let r = e.release(0, 600_000).unwrap(); // release at end -> end PG
        assert_eq!(r.judge, Judge::PerfectGreat, "worse(PG, PG) = PG");
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.combo, 1);
        assert_eq!(e.ex_score, 2);
    }

    #[test]
    fn ln_release_takes_worse_when_end_is_bad() {
        // Head PG, but release far from end so end window classifies worse.
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap(); // head PG
        // ln_end gd is +-200_000; release 230ms early -> dm +230_000 -> BD on the end.
        let r = e.release(0, 370_000).unwrap();
        assert_eq!(r.judge, Judge::Bad, "worse(PG, BD) = BD");
        assert_eq!(e.counts[3], 1);
        assert_eq!(e.combo, 0, "a BAD release breaks combo");
    }

    #[test]
    fn ln_release_keeps_worse_head_when_end_is_perfect() {
        // Head only GOOD (pressed 130ms early on a 7K head: gd window), end released perfectly.
        let mut e = ln(1_000_000, 1_600_000);
        let hr = e.press(0, 870_000).unwrap(); // dm +130_000 -> head GOOD
        assert_eq!(hr.judge, Judge::Good);
        let r = e.release(0, 1_600_000).unwrap(); // end PG
        assert_eq!(r.judge, Judge::Good, "worse(GOOD, PG) = GOOD");
        assert_eq!(e.counts[2], 1);
        assert_eq!(e.combo, 1, "GOOD keeps combo");
    }

    #[test]
    fn ln_release_delta_is_end_minus_release_and_records_timing() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        let r = e.release(0, 590_000).unwrap(); // dm = 600_000 - 590_000 = +10_000
        assert_eq!(r.delta_us, 10_000);
        assert!(r.fast, "released early");
        assert_eq!(e.fast, 1, "release timing counted");
        assert_eq!(e.avg_judge_us(), 10_000, "single timed hit = its own delta");
    }

    #[test]
    fn ln_release_without_hold_returns_none() {
        let mut e = ln(100_000, 600_000);
        // never pressed the head -> nothing is holding.
        assert!(e.release(0, 600_000).is_none());
    }

    #[test]
    fn ln_release_on_invalid_lane_returns_none() {
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        assert!(e.release(9, 600_000).is_none(), "no such lane");
    }

    #[test]
    fn ln_update_force_finalizes_over_held_ln() {
        // Held LN never released; update past end + LN_MARGIN must finalize it.
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap(); // head PG, holding
        e.update(600_000 + 200_000); // exactly end + LN_MARGIN: NOT yet finalized (strict >)
        assert_eq!(e.total_judged(), 0, "at exactly end+margin the LN is still held");
        e.update(600_000 + 200_001); // 1us past margin -> finalized
        assert_eq!(e.total_judged(), 1, "over-held LN finalized");
        // dm = end - now = 600_000 - 800_001 = -200_001 -> ln_end BD; worse(PG, BD) = BD.
        assert_eq!(e.counts[3], 1, "force-finalized as BAD");
        assert_eq!(e.late[3], 1, "sweep/force-finalize is always late");
        assert_eq!(e.combo, 0);
    }

    #[test]
    fn ln_held_note_blocks_cursor_until_finalized() {
        // While an LN is held, update must not sweep it early as a miss.
        let mut e = ln(100_000, 600_000);
        e.press(0, 100_000).unwrap();
        e.update(650_000); // past end but within margin
        assert_eq!(e.total_judged(), 0, "still holding, not missed");
        assert_eq!(e.combo, 0);
    }

    #[test]
    fn set_ln_end_widens_release_leniency() {
        // Default ln_end gd is +-200_000; a 230ms-early release is BD. After widening to a huge
        // window, the same release lands inside PG.
        let mut e = ln(100_000, 600_000);
        let wide = JudgeWindows {
            pg: (-400_000, 400_000),
            gr: (-450_000, 450_000),
            gd: (-500_000, 500_000),
            bd: (-550_000, 550_000),
            ms: (-550_000, 600_000),
        };
        e.set_ln_end(wide);
        e.press(0, 100_000).unwrap(); // head PG
        let r = e.release(0, 370_000).unwrap(); // dm +230_000, now inside widened PG
        assert_eq!(r.judge, Judge::PerfectGreat, "wide ln_end keeps release PG");
        assert_eq!(e.counts[0], 1);
    }

    #[test]
    fn ln_head_empty_poor_does_not_start_hold() {
        // A far-early press on an LN head is an empty poor: it must NOT start a hold.
        let mut e = ln(1_000_000, 1_600_000);
        let r = e.press(0, 700_000).unwrap(); // dm +300_000 -> empty poor
        assert_eq!(r.judge, Judge::Poor);
        assert_eq!(e.empty_poor, 1);
        // nothing is holding, so a release finds nothing.
        assert!(e.release(0, 1_600_000).is_none(), "empty poor must not arm a hold");
        // the head is still hittable.
        let r2 = e.press(0, 1_000_000).unwrap();
        assert_eq!(r2.judge, Judge::PerfectGreat);
    }

    // --- sweeps / update edges ---------------------------------------------

    #[test]
    fn update_miss_bound_is_exclusive_strict_less() {
        // A note is swept only when head - now < miss_bound (bd.0 = -280_000), i.e. strictly past.
        let mut e = note(1_000_000);
        // now such that head - now == miss_bound exactly: now = head - bd.0 = 1_000_000 + 280_000.
        e.update(1_280_000);
        assert_eq!(e.total_judged(), 0, "at exactly the bound the note is not yet swept (strict <)");
        e.update(1_280_001);
        assert_eq!(e.counts[5], 1, "1us further sweeps it to MISS");
    }

    #[test]
    fn update_is_idempotent_after_sweep() {
        let mut e = note(100_000);
        e.update(5_000_000);
        let counts = e.counts;
        e.update(9_000_000);
        assert_eq!(e.counts, counts, "re-running update does not double-count");
        assert_eq!(e.counts[5], 1);
    }

    #[test]
    fn update_sweeps_multiple_lanes_and_notes() {
        let mut e = Eng::new(vec![vec![100_000, 200_000], vec![300_000]], JudgeWindows::SEVENKEY_NOTE);
        e.update(10_000_000);
        assert_eq!(e.counts[5], 3, "all three notes swept to MISS");
        assert_eq!(e.combo, 0);
        for i in 0..6 {
            assert_eq!(e.counts[i], e.early[i] + e.late[i], "counts[{i}] == early+late");
        }
        assert_eq!(e.late[5], 3, "every swept miss is late");
        assert_eq!(e.early[5], 0);
    }

    #[test]
    fn swept_miss_is_excluded_from_avg_and_fast_slow() {
        let mut e = note(100_000);
        e.update(5_000_000);
        assert_eq!(e.counts[5], 1);
        assert_eq!(e.fast, 0, "miss not fast");
        assert_eq!(e.slow, 0, "miss not slow");
        assert_eq!(e.avg_judge_us(), 0, "miss excluded from timing average");
    }

    // --- count/early/late invariants over a mixed run ----------------------

    #[test]
    fn early_plus_late_equals_counts_invariant_mixed_run() {
        let mut e = Eng::new(
            vec![vec![1_000_000, 2_000_000, 3_000_000, 4_000_000, 5_000_000]],
            JudgeWindows::SEVENKEY_NOTE,
        );
        e.press(0, 990_000); // early PG
        e.press(0, 2_010_000); // late PG
        e.press(0, 2_950_000); // dm +50_000 early GR
        e.press(0, 4_100_000); // dm -100_000 late GD
        e.update(9_000_000); // note 5 swept -> late MISS
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
    fn dm_zero_is_classified_as_late_not_early() {
        // dm == 0 (perfect timing) is recorded as LATE (record_direction uses dm > 0 for early).
        let mut e = note(1_000_000);
        e.press(0, 1_000_000).unwrap(); // dm 0 -> PG, late
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.late[0], 1, "dm==0 counts as late");
        assert_eq!(e.early[0], 0);
        assert_eq!(e.fast, 0, "dm==0 is neither fast nor slow");
        assert_eq!(e.slow, 0);
    }

    // --- determinism --------------------------------------------------------

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

    // --- ex_score accumulation ---------------------------------------------

    #[test]
    fn ex_score_is_2pg_plus_1gr_only() {
        let mut e = Eng::new(vec![vec![1_000_000, 2_000_000, 3_000_000, 4_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 1_000_000); // PG +2
        e.press(0, 2_050_000); // dm +50_000 GR +1
        e.press(0, 3_100_000); // dm -100_000 GD +0
        e.press(0, 4_250_000); // dm -250_000 BD +0
        assert_eq!(e.counts[0], 1);
        assert_eq!(e.counts[1], 1);
        assert_eq!(e.counts[2], 1);
        assert_eq!(e.counts[3], 1);
        assert_eq!(e.ex_score, 3, "EX = 2*PG + 1*GR = 2 + 1");
    }

    #[test]
    fn max_combo_persists_after_break() {
        let mut e = Eng::new(vec![vec![1_000_000, 2_000_000, 3_000_000]], JudgeWindows::SEVENKEY_NOTE);
        e.press(0, 1_000_000); // PG combo 1
        e.press(0, 2_000_000); // PG combo 2
        e.press(0, 3_250_000); // dm -250_000 BD -> combo 0
        assert_eq!(e.combo, 0);
        assert_eq!(e.max_combo, 2, "max_combo remembers the best streak");
    }

    // --- from_model wiring (mode-driven ln_end) -----------------------------

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
        // The normal note is hittable.
        let r = e.press(0, 100_000).unwrap();
        assert_eq!(r.judge, Judge::PerfectGreat);
        // The LN head is hittable and starts a hold.
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
        // A LongStart with no matching LongEnd produces no note (pending_start never flushed).
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
        // POPN_9K with rank 3 (judgerank 100): ln_end == POPN_LN_END at 100%, so a 130ms-early
        // release on a POPN LN is GR (POPN_LN_END pg is +-120_000, gr +-150_000).
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
            timelines: vec![
                mk_tl(1_000_000, NoteKind::LongStart { ln: LnKind::Ln }),
                mk_tl(1_600_000, NoteKind::LongEnd { ln: LnKind::Ln }),
            ],
            md5: String::new(),
            sha256: String::new(),
        };
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::POPN_NOTE);
        e.press(0, 1_000_000).unwrap(); // head PG
        let r = e.release(0, 1_730_000).unwrap(); // dm +130_000 -> POPN_LN_END GR
        assert_eq!(r.judge, Judge::Great, "POPN ln_end classifies 130ms-early release as GR");
    }

    // --- CN/HCN charge notes: head + release end are two counted judgments ---------------------
    // (beatoraja JudgeManager calls updateMicro at both press and key-up; see
    // docs/reference/cn-hcn-judgment.md). Plain LN stays one judgment — the regression guard below.

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
        // Head PG counted at press; releasing 500ms early is outside the (75%-scaled) LN-end window,
        // so the end is POOR on its own — not the worse-of-head-and-end a plain LN would yield.
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
        e.update(5_000_000); // sweep far past the head — never hit
        assert_eq!(e.counts[5], 2, "a never-hit CN misses both head and end");
        assert_eq!(e.total_judged(), 2);
    }

    #[test]
    fn hcn_end_is_also_two_judgments() {
        use rbms_model::LnKind;
        // HCN shares the CN end-judgment model (its continuous gauge is a separate, deferred concern).
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
        // Regression guard: a plain LN is unchanged — one judged object, resolved at release as the
        // worse of head/end, with nothing counted at press.
        let model = cn_model(LnKind::Ln, 1_000_000, 2_000_000);
        let mut e = JudgeEngine::from_model(&model, JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(e.total_notes(), 1, "a plain LN is a single judged object");
        assert_eq!(e.press(0, 1_000_000).unwrap().judge, Judge::PerfectGreat);
        assert_eq!(e.counts[0], 0, "LN head is not counted until release");
        let r = e.release(0, 1_500_000).unwrap(); // early -> POOR end -> worse(PG, POOR) = POOR
        assert_eq!(r.judge, Judge::Poor);
        assert_eq!(e.total_judged(), 1, "still one judgment for the LN");
    }
}
