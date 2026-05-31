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
