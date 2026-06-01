use rbms_model::Mode;

use crate::Judge;

/// Judge timing windows in microseconds. Each pair is `(late_bound, early_bound)` where
/// the matched delta `dmtime = note_time - press_time` (>0 = pressed early/FAST, <0 =
/// late/SLOW) must satisfy `late <= dmtime <= early`. Values are beatoraja's
/// `JudgeProperty` 7K NOTE table at judgerank 100.
#[derive(Debug, Clone, Copy)]
pub struct JudgeWindows {
    pub pg: (i64, i64),
    pub gr: (i64, i64),
    pub gd: (i64, i64),
    pub bd: (i64, i64),
    pub ms: (i64, i64),
}

impl JudgeWindows {
    pub const SEVENKEY_NOTE: JudgeWindows = JudgeWindows {
        pg: (-20_000, 20_000),
        gr: (-60_000, 60_000),
        gd: (-150_000, 150_000),
        bd: (-280_000, 220_000),
        ms: (-150_000, 500_000),
    };

    /// LN/CN release window (beatoraja `JudgeProperty` longnote end, 7K).
    pub const SEVENKEY_LN_END: JudgeWindows = JudgeWindows {
        pg: (-120_000, 120_000),
        gr: (-150_000, 150_000),
        gd: (-200_000, 200_000),
        bd: (-250_000, 250_000),
        ms: (-250_000, 500_000),
    };

    /// pop'n (PMS) NOTE window. Currently mirrors [`SEVENKEY_NOTE`](Self::SEVENKEY_NOTE) pending
    /// verified beatoraja `JudgeProperty` PMS values; kept as a distinct constant so tuning pop'n
    /// never touches the beat table.
    pub const POPN_NOTE: JudgeWindows = JudgeWindows {
        pg: (-20_000, 20_000),
        gr: (-60_000, 60_000),
        gd: (-150_000, 150_000),
        bd: (-280_000, 220_000),
        ms: (-150_000, 500_000),
    };

    /// pop'n (PMS) LN release window. Mirrors [`SEVENKEY_LN_END`](Self::SEVENKEY_LN_END) for now
    /// (see [`POPN_NOTE`](Self::POPN_NOTE)).
    pub const POPN_LN_END: JudgeWindows = JudgeWindows {
        pg: (-120_000, 120_000),
        gr: (-150_000, 150_000),
        gd: (-200_000, 200_000),
        bd: (-250_000, 250_000),
        ms: (-250_000, 500_000),
    };

    /// NOTE timing window for `mode`. The BEAT modes (5K/7K/10K/14K) share beatoraja's SEVENKEYS
    /// table; POPN (PMS) selects its own. Selecting by mode keeps the project's "new key modes are
    /// data" rule — a new mode only adds a row here, never branches in the engine.
    pub fn note_for_mode(mode: &Mode) -> JudgeWindows {
        match mode.name {
            "POPN_9K" => Self::POPN_NOTE,
            _ => Self::SEVENKEY_NOTE,
        }
    }

    /// LN/CN release window for `mode` (see [`note_for_mode`](Self::note_for_mode)).
    pub fn ln_end_for_mode(mode: &Mode) -> JudgeWindows {
        match mode.name {
            "POPN_9K" => Self::POPN_LN_END,
            _ => Self::SEVENKEY_LN_END,
        }
    }

    /// Apply a `judgerank` percentage (100 = default). PG/GR/GD/BD scale; the MS window is
    /// fixed (beatoraja `fixjudge` for NORMAL only fixes index 4).
    pub fn scaled(&self, judgerank_percent: i32) -> JudgeWindows {
        let p = judgerank_percent.max(1) as i64;
        let s = |w: (i64, i64)| (w.0 * p / 100, w.1 * p / 100);
        JudgeWindows { pg: s(self.pg), gr: s(self.gr), gd: s(self.gd), bd: s(self.bd), ms: self.ms }
    }

    /// Classify a timing delta into a judge, or `None` if outside even the MS window.
    pub fn judge(&self, dmtime: i64) -> Option<Judge> {
        let inw = |w: (i64, i64)| dmtime >= w.0 && dmtime <= w.1;
        if inw(self.pg) {
            Some(Judge::PerfectGreat)
        } else if inw(self.gr) {
            Some(Judge::Great)
        } else if inw(self.gd) {
            Some(Judge::Good)
        } else if inw(self.bd) {
            Some(Judge::Bad)
        } else if inw(self.ms) {
            Some(Judge::Poor)
        } else {
            None
        }
    }
}

/// `#RANK` index (0..4 = VERYHARD/HARD/NORMAL/EASY/VERYEASY) → judgerank percent for the
/// BEAT modes (`JudgeWindowRule.NORMAL.judgerank`). Out-of-range falls back to NORMAL.
pub fn rank_to_judgerank(rank: i32) -> i32 {
    const TABLE: [i32; 5] = [25, 50, 75, 100, 125];
    *TABLE.get(rank.clamp(0, 4) as usize).unwrap_or(&75)
}

#[cfg(test)]
mod windows_tests {
    use super::*;
    use rbms_model::Mode;

    const W: JudgeWindows = JudgeWindows::SEVENKEY_NOTE;

    // --- judge() exact boundaries: PG ---------------------------------------

    #[test]
    fn pg_boundaries_inclusive() {
        assert_eq!(W.judge(0), Some(Judge::PerfectGreat), "exact note time");
        assert_eq!(W.judge(20_000), Some(Judge::PerfectGreat), "early PG edge inclusive");
        assert_eq!(W.judge(-20_000), Some(Judge::PerfectGreat), "late PG edge inclusive");
    }

    #[test]
    fn pg_just_outside_becomes_great() {
        assert_eq!(W.judge(20_001), Some(Judge::Great), "1us past early PG -> GR");
        assert_eq!(W.judge(-20_001), Some(Judge::Great), "1us past late PG -> GR");
    }

    // --- judge() exact boundaries: GR ---------------------------------------

    #[test]
    fn gr_boundaries_inclusive() {
        assert_eq!(W.judge(60_000), Some(Judge::Great));
        assert_eq!(W.judge(-60_000), Some(Judge::Great));
    }

    #[test]
    fn gr_just_outside_becomes_good() {
        assert_eq!(W.judge(60_001), Some(Judge::Good));
        assert_eq!(W.judge(-60_001), Some(Judge::Good));
    }

    // --- judge() exact boundaries: GD ---------------------------------------

    #[test]
    fn gd_boundaries_inclusive() {
        assert_eq!(W.judge(150_000), Some(Judge::Good), "early GD edge");
        assert_eq!(W.judge(-150_000), Some(Judge::Good), "late GD edge (== ms.0, but GD wins)");
    }

    #[test]
    fn gd_just_outside_becomes_bad() {
        assert_eq!(W.judge(150_001), Some(Judge::Bad), "early past GD -> BD");
        assert_eq!(W.judge(-150_001), Some(Judge::Bad), "late past GD -> BD");
    }

    // --- judge() exact boundaries: BD (asymmetric) --------------------------

    #[test]
    fn bd_early_upper_edge_inclusive() {
        assert_eq!(W.judge(220_000), Some(Judge::Bad), "early BD upper edge");
    }

    #[test]
    fn bd_late_lower_edge_inclusive() {
        assert_eq!(W.judge(-280_000), Some(Judge::Bad), "late BD lower edge (asymmetric, wider than early)");
    }

    #[test]
    fn bd_is_asymmetric() {
        // Early BD only reaches +220ms but late BD reaches -280ms: a +250ms delta is past BD (POOR),
        // while a symmetric -250ms is still BD.
        assert_eq!(W.judge(250_000), Some(Judge::Poor), "+250ms is past early BD -> empty POOR");
        assert_eq!(W.judge(-250_000), Some(Judge::Bad), "-250ms is still within late BD");
    }

    // --- judge() POOR / MS band & out-of-range None -------------------------

    #[test]
    fn early_poor_band_only_above_bd() {
        // POOR appears only on the EARLY side, in +220_001..=+500_000.
        assert_eq!(W.judge(220_001), Some(Judge::Poor), "1us past early BD -> POOR");
        assert_eq!(W.judge(500_000), Some(Judge::Poor), "ms early edge inclusive");
    }

    #[test]
    fn there_is_no_late_poor() {
        // ms.0 == gd.0 == -150_000 and bd reaches -280_000, so nothing classifies as a LATE poor:
        // every late delta is GD/BD until it falls off the BD edge into None.
        for dm in [-150_000, -200_000, -280_000] {
            assert_ne!(W.judge(dm), Some(Judge::Poor), "no late POOR at {dm}");
        }
    }

    #[test]
    fn out_of_range_is_none() {
        assert_eq!(W.judge(500_001), None, "1us past early ms edge");
        assert_eq!(W.judge(-280_001), None, "1us past late bd edge (no ms below -150k anyway)");
        assert_eq!(W.judge(10_000_000), None, "far early");
        assert_eq!(W.judge(-10_000_000), None, "far late");
    }

    #[test]
    fn monotonic_tier_widening_from_center() {
        // Walking outward from 0 on the early side, the tier never improves.
        let mut prev = Judge::PerfectGreat as usize;
        for dm in (0..=520_000).step_by(1_000) {
            let j = W.judge(dm).map(|x| x as usize).unwrap_or(6);
            assert!(j >= prev, "tier must be non-decreasing outward; dm={dm} j={j} prev={prev}");
            prev = j;
        }
    }

    // --- scaled() -----------------------------------------------------------

    #[test]
    fn scaled_100_is_identity_for_scaled_tiers() {
        let s = W.scaled(100);
        assert_eq!(s.pg, W.pg);
        assert_eq!(s.gr, W.gr);
        assert_eq!(s.gd, W.gd);
        assert_eq!(s.bd, W.bd);
    }

    #[test]
    fn scaled_leaves_ms_fixed() {
        for p in [1, 50, 100, 125, 200] {
            assert_eq!(W.scaled(p).ms, W.ms, "ms must stay fixed at judgerank {p}");
        }
    }

    #[test]
    fn scaled_50_halves_pg_gr_gd_bd() {
        let s = W.scaled(50);
        assert_eq!(s.pg, (-10_000, 10_000));
        assert_eq!(s.gr, (-30_000, 30_000));
        assert_eq!(s.gd, (-75_000, 75_000));
        assert_eq!(s.bd, (-140_000, 110_000), "bd halves and stays asymmetric");
    }

    #[test]
    fn scaled_125_widens() {
        let s = W.scaled(125);
        assert_eq!(s.pg, (-25_000, 25_000));
        assert_eq!(s.bd, (-350_000, 275_000));
    }

    #[test]
    fn scaled_clamps_percent_to_at_least_1() {
        // judgerank 0 and negatives clamp to 1% (max(1)), never zero-width or panic.
        let s0 = W.scaled(0);
        let sn = W.scaled(-999);
        assert_eq!(s0.pg, sn.pg, "0 and negative both clamp to 1%");
        // 1% of pg (-20_000,20_000) -> (-200, 200)
        assert_eq!(s0.pg, (-200, 200));
        // window remains usable: exact center is still PG.
        assert_eq!(s0.judge(0), Some(Judge::PerfectGreat));
    }

    #[test]
    fn scaled_tighter_window_demotes_borderline_hit() {
        // At 50%, a +15ms press that was PG at 100% becomes GR (pg shrinks to +-10ms).
        let s = W.scaled(50);
        assert_eq!(W.judge(15_000), Some(Judge::PerfectGreat));
        assert_eq!(s.judge(15_000), Some(Judge::Great));
    }

    #[test]
    fn scaled_is_truncating_integer_division() {
        // 33% of pg upper 20_000 = 20_000*33/100 = 6_600 (exact); use a value that truncates.
        // 33% of gr 60_000 = 19_800; 7% of pg 20_000 = 1_400. Pick gd 150_000 * 33 / 100 = 49_500.
        let s = W.scaled(33);
        assert_eq!(s.pg, (-6_600, 6_600));
        assert_eq!(s.gd, (-49_500, 49_500));
        // truncation: 150_000 * 7 / 100 = 10_500 exact; use a non-divisible one: bd.1 220_000*33/100=72_600
        assert_eq!(s.bd.1, 72_600);
    }

    // --- mode selection -----------------------------------------------------

    #[test]
    fn note_for_beat_modes_uses_sevenkey_table() {
        for m in [Mode::BEAT_7K, Mode::BEAT_5K, Mode::BEAT_10K, Mode::BEAT_14K] {
            let w = JudgeWindows::note_for_mode(&m);
            assert_eq!(w.pg, JudgeWindows::SEVENKEY_NOTE.pg, "{} note pg", m.name);
            assert_eq!(w.bd, JudgeWindows::SEVENKEY_NOTE.bd, "{} note bd", m.name);
            assert_eq!(w.ms, JudgeWindows::SEVENKEY_NOTE.ms, "{} note ms", m.name);
        }
    }

    #[test]
    fn note_for_popn_uses_popn_table() {
        let w = JudgeWindows::note_for_mode(&Mode::POPN_9K);
        assert_eq!(w.pg, JudgeWindows::POPN_NOTE.pg);
        assert_eq!(w.gr, JudgeWindows::POPN_NOTE.gr);
        assert_eq!(w.gd, JudgeWindows::POPN_NOTE.gd);
        assert_eq!(w.bd, JudgeWindows::POPN_NOTE.bd);
        assert_eq!(w.ms, JudgeWindows::POPN_NOTE.ms);
    }

    #[test]
    fn ln_end_for_beat_modes_uses_sevenkey_ln() {
        for m in [Mode::BEAT_7K, Mode::BEAT_5K, Mode::BEAT_10K, Mode::BEAT_14K] {
            let w = JudgeWindows::ln_end_for_mode(&m);
            assert_eq!(w.pg, JudgeWindows::SEVENKEY_LN_END.pg, "{} ln pg", m.name);
            assert_eq!(w.bd, JudgeWindows::SEVENKEY_LN_END.bd, "{} ln bd", m.name);
        }
    }

    #[test]
    fn ln_end_for_popn_uses_popn_ln() {
        let w = JudgeWindows::ln_end_for_mode(&Mode::POPN_9K);
        assert_eq!(w.pg, JudgeWindows::POPN_LN_END.pg);
        assert_eq!(w.ms, JudgeWindows::POPN_LN_END.ms);
    }

    #[test]
    fn ln_end_window_is_wider_than_note_window() {
        // The LN release window must be more lenient than the head: PG +-120ms vs +-20ms.
        let note = JudgeWindows::SEVENKEY_NOTE;
        let ln = JudgeWindows::SEVENKEY_LN_END;
        assert!(ln.pg.1 > note.pg.1, "LN PG wider");
        assert!(ln.gr.1 > note.gr.1, "LN GR wider");
        assert_eq!(ln.judge(120_000), Some(Judge::PerfectGreat), "LN release 120ms early is still PG");
        assert_eq!(note.judge(120_000), Some(Judge::Good), "note 120ms early is only GD");
    }

    // --- rank_to_judgerank table & clamping ---------------------------------

    #[test]
    fn rank_table_exact_values() {
        assert_eq!(rank_to_judgerank(0), 25, "VERYHARD");
        assert_eq!(rank_to_judgerank(1), 50, "HARD");
        assert_eq!(rank_to_judgerank(2), 75, "NORMAL-ish");
        assert_eq!(rank_to_judgerank(3), 100, "EASY/default");
        assert_eq!(rank_to_judgerank(4), 125, "VERYEASY");
    }

    #[test]
    fn rank_clamps_out_of_range() {
        // clamp(0,4): negatives -> index 0 (25), large -> index 4 (125).
        assert_eq!(rank_to_judgerank(-1), 25);
        assert_eq!(rank_to_judgerank(-100), 25);
        assert_eq!(rank_to_judgerank(5), 125);
        assert_eq!(rank_to_judgerank(i32::MAX), 125);
        assert_eq!(rank_to_judgerank(i32::MIN), 25);
    }

    #[test]
    fn rank_is_monotonic_nondecreasing() {
        let mut prev = 0;
        for r in -2..=6 {
            let v = rank_to_judgerank(r);
            assert!(v >= prev, "rank_to_judgerank must be non-decreasing: r={r} v={v} prev={prev}");
            prev = v;
        }
    }
}
