use rbms_model::Mode;
use serde::{Deserialize, Serialize};

use crate::Judge;
use crate::algorithm::NoteType;

/// Judge timing windows in microseconds. Each pair is `(late_bound, early_bound)` where
/// the matched delta `dmtime = note_time - press_time` (>0 = pressed early/FAST, <0 =
/// late/SLOW) must satisfy `late <= dmtime <= early`.
///
/// `ms` is the reference implementation's fifth pair (index 4 of `JudgeProperty`), the 空POOR band: a press that
/// reaches only this window neither consumes the note nor (on 7K) breaks combo. Long-note end
/// tables have no fifth pair in the reference implementation (`JudgeProperty.longnote` is 8 longs, not 10), so `ms`
/// is `None` there and anything outside BAD is judge code 4 (見逃し POOR) at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeWindows {
    pub pg: (i64, i64),
    pub gr: (i64, i64),
    pub gd: (i64, i64),
    pub bd: (i64, i64),
    pub ms: Option<(i64, i64)>,
}

/// Reference implementation `JudgeProperty.MissCondition`. `Always` counts every 見逃し POOR; `One` (PMS) counts
/// only the first one per note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissCondition {
    Always,
    One,
}

/// One row of the reference implementation's `JudgeProperty` enum: the four timing tables, their release margins and
/// the per-judge combo/vanish policy. Adding a play mode is a new row here, never a branch in the
/// judge engine.
#[derive(Debug, Clone, Copy)]
pub struct JudgeProperty {
    pub note: JudgeWindows,
    pub scratch: JudgeWindows,
    pub ln_end: JudgeWindows,
    pub ln_scratch_end: JudgeWindows,
    /// `JudgeProperty.longnoteMargin` (µs): how long after a too-early release a plain long note
    /// waits before it is finalised. 0 for the BEAT modes, 200 ms for PMS.
    pub longnote_margin: i64,
    /// `JudgeProperty.longscratchMargin` (µs).
    pub longscratch_margin: i64,
    /// `JudgeProperty.combo`, indexed by judge (PG, GR, GD, BD, PR, MS). `true` keeps the combo
    /// (and increments it for indices below 5), `false` resets it.
    pub combo: [bool; 6],
    /// `JudgeProperty.judgeVanish`: whether the judged note is consumed, indexed by judge.
    pub judge_vanish: [bool; 6],
    pub miss_condition: MissCondition,
}

const fn w5(pg: (i64, i64), gr: (i64, i64), gd: (i64, i64), bd: (i64, i64), ms: (i64, i64)) -> JudgeWindows {
    JudgeWindows { pg, gr, gd, bd, ms: Some(ms) }
}

const fn w4(pg: (i64, i64), gr: (i64, i64), gd: (i64, i64), bd: (i64, i64)) -> JudgeWindows {
    JudgeWindows { pg, gr, gd, bd, ms: None }
}

impl JudgeProperty {
    /// Reference implementation `JudgeProperty.FIVEKEYS` (`JudgeProperty.java:12-22`).
    pub const FIVEKEYS: JudgeProperty = JudgeProperty {
        note: w5((-20_000, 20_000), (-50_000, 50_000), (-100_000, 100_000), (-150_000, 150_000), (-150_000, 500_000)),
        scratch: w5((-30_000, 30_000), (-60_000, 60_000), (-110_000, 110_000), (-160_000, 160_000), (-160_000, 500_000)),
        ln_end: w4((-120_000, 120_000), (-150_000, 150_000), (-200_000, 200_000), (-250_000, 250_000)),
        ln_scratch_end: w4((-130_000, 130_000), (-160_000, 160_000), (-110_000, 110_000), (-260_000, 260_000)),
        longnote_margin: 0,
        longscratch_margin: 0,
        combo: [true, true, true, false, false, false],
        judge_vanish: [true, true, true, true, true, false],
        miss_condition: MissCondition::Always,
    };

    /// Reference implementation `JudgeProperty.SEVENKEYS` (`JudgeProperty.java:23-33`).
    pub const SEVENKEYS: JudgeProperty = JudgeProperty {
        note: w5((-20_000, 20_000), (-60_000, 60_000), (-150_000, 150_000), (-280_000, 220_000), (-150_000, 500_000)),
        scratch: w5((-30_000, 30_000), (-70_000, 70_000), (-160_000, 160_000), (-290_000, 230_000), (-160_000, 500_000)),
        ln_end: w4((-120_000, 120_000), (-160_000, 160_000), (-200_000, 200_000), (-280_000, 220_000)),
        ln_scratch_end: w4((-130_000, 130_000), (-170_000, 170_000), (-210_000, 210_000), (-290_000, 230_000)),
        longnote_margin: 0,
        longscratch_margin: 0,
        combo: [true, true, true, false, false, true],
        judge_vanish: [true, true, true, true, true, false],
        miss_condition: MissCondition::Always,
    };

    /// Reference implementation `JudgeProperty.PMS` (`JudgeProperty.java:34-44`). pop'n has no scratch lane, so
    /// the reference implementation leaves both scratch tables empty; the note/LN tables stand in here so a stray
    /// scratch lookup can never hit a zero-length window.
    pub const PMS: JudgeProperty = JudgeProperty {
        note: w5((-20_000, 20_000), (-50_000, 50_000), (-117_000, 117_000), (-183_000, 183_000), (-175_000, 500_000)),
        scratch: w5((-20_000, 20_000), (-50_000, 50_000), (-117_000, 117_000), (-183_000, 183_000), (-175_000, 500_000)),
        ln_end: w4((-120_000, 120_000), (-150_000, 150_000), (-217_000, 217_000), (-283_000, 283_000)),
        ln_scratch_end: w4((-120_000, 120_000), (-150_000, 150_000), (-217_000, 217_000), (-283_000, 283_000)),
        longnote_margin: 200_000,
        longscratch_margin: 0,
        combo: [true, true, true, false, false, false],
        judge_vanish: [true, true, true, false, true, false],
        miss_condition: MissCondition::One,
    };

    /// Reference implementation `JudgeProperty.KEYBOARD` (`JudgeProperty.java:45-55`). Data only — the 24K mode is
    /// not wired into `rbms_model::Mode` yet, so nothing selects this row.
    pub const KEYBOARD: JudgeProperty = JudgeProperty {
        note: w5((-30_000, 30_000), (-90_000, 90_000), (-200_000, 200_000), (-320_000, 240_000), (-200_000, 650_000)),
        scratch: w5((-30_000, 30_000), (-90_000, 90_000), (-200_000, 200_000), (-320_000, 240_000), (-200_000, 650_000)),
        ln_end: w4((-160_000, 25_000), (-200_000, 75_000), (-260_000, 140_000), (-320_000, 240_000)),
        ln_scratch_end: w4((-160_000, 25_000), (-200_000, 75_000), (-260_000, 140_000), (-320_000, 240_000)),
        longnote_margin: 0,
        longscratch_margin: 0,
        combo: [true, true, true, false, false, true],
        judge_vanish: [true, true, true, true, true, false],
        miss_condition: MissCondition::Always,
    };

    /// Table for `mode`, read from the bundled `data/judge.ron`.
    ///
    /// The data file is the source the engine judges against; [`JudgeProperty::defaults_for_mode`]
    /// is the compiled-in fallback for a mode the file has no row for, and the parity guard in
    /// [`crate::data`] asserts the two agree field for field.
    pub fn for_mode(mode: &Mode) -> JudgeProperty {
        match crate::data::builtin_judge_tables().for_mode(mode) {
            Some(row) => JudgeProperty::from(*row),
            None => Self::defaults_for_mode(mode),
        }
    }

    /// Compiled-in table for `mode`: BEAT_5K/10K use FIVEKEYS, BEAT_7K/14K use SEVENKEYS, POPN_9K
    /// uses PMS. Used when the data file has no row for the mode, and as the baseline the data
    /// file's parity guard compares against.
    pub fn defaults_for_mode(mode: &Mode) -> JudgeProperty {
        match mode.name {
            "BEAT_5K" | "BEAT_10K" => Self::FIVEKEYS,
            "POPN_9K" => Self::PMS,
            _ => Self::SEVENKEYS,
        }
    }

    /// Widest candidate window across the NOTE and SCRATCH tables, as `(late, early)`. The reference implementation
    /// seeds `mjudgestart`/`mjudgeend` at 0 and folds both tables' bounds in
    /// (`JudgeManager.java:189-197`), so the gate is chart-global, not per-lane.
    pub fn candidate_gate(&self) -> (i64, i64) {
        let mut start = 0;
        let mut end = 0;
        for w in [self.note, self.scratch] {
            for pair in w.pairs() {
                start = start.min(pair.0);
                end = end.max(pair.1);
            }
        }
        (start, end)
    }
}

impl JudgeWindows {
    /// Reference implementation `JudgeProperty.SEVENKEYS` NOTE row, kept as a standalone constant because it is
    /// the default window for the mode-less constructors and most tests.
    pub const SEVENKEY_NOTE: JudgeWindows = JudgeProperty::SEVENKEYS.note;

    /// Reference implementation `JudgeProperty.SEVENKEYS` LONGNOTE_END row.
    pub const SEVENKEY_LN_END: JudgeWindows = JudgeProperty::SEVENKEYS.ln_end;

    /// Reference implementation `JudgeProperty.PMS` NOTE row.
    pub const POPN_NOTE: JudgeWindows = JudgeProperty::PMS.note;

    /// Reference implementation `JudgeProperty.PMS` LONGNOTE_END row.
    pub const POPN_LN_END: JudgeWindows = JudgeProperty::PMS.ln_end;

    /// The window pairs in reference implementation order (PG, GR, GD, BD, and MS when present).
    pub fn pairs(&self) -> Vec<(i64, i64)> {
        let mut v = vec![self.pg, self.gr, self.gd, self.bd];
        if let Some(ms) = self.ms {
            v.push(ms);
        }
        v
    }

    /// NOTE timing window for `mode` (see [`JudgeProperty::for_mode`]).
    pub fn note_for_mode(mode: &Mode) -> JudgeWindows {
        JudgeProperty::for_mode(mode).note
    }

    /// LN/CN release window for `mode` (see [`JudgeProperty::for_mode`]).
    pub fn ln_end_for_mode(mode: &Mode) -> JudgeWindows {
        JudgeProperty::for_mode(mode).ln_end
    }

    /// Apply a `judgerank` percentage (100 = default). PG/GR/GD/BD scale; the MS window is
    /// fixed (reference implementation `JudgeWindowRule.NORMAL.fixjudge` fixes only index 4). It
    /// (`JudgeProperty.java:234`) applies the percentage with no lower clamp, so a judgerank of 0
    /// collapses PG/GR/GD/BD to `(0, 0)` here too.
    pub fn scaled(&self, judgerank_percent: i32) -> JudgeWindows {
        let p = judgerank_percent.max(0) as i64;
        let s = |w: (i64, i64)| (w.0 * p / 100, w.1 * p / 100);
        JudgeWindows { pg: s(self.pg), gr: s(self.gr), gd: s(self.gd), bd: s(self.bd), ms: self.ms }
    }

    /// Apply the user JUDGE WIDTH rates (percent, `[PG, GR, GD]`), reference implementation
    /// `JudgeWindowRule.create` lines 262-273. Only the first three tiers scale; each bound is then
    /// clamped so it never exceeds the BAD bound (`judge[6 + j]` in the Java, i.e. index 3) and
    /// never shrinks below the tier before it. BAD and MS are untouched.
    pub fn with_window_rate(&self, rates: [i32; 3]) -> JudgeWindows {
        let mut tiers = [self.pg, self.gr, self.gd];
        let limit = self.bd;
        for i in 0..3 {
            let rate = rates[i].max(0) as i64;
            let bounds = [tiers[i].0, tiers[i].1];
            let limits = [limit.0, limit.1];
            let prev = if i > 0 { Some([tiers[i - 1].0, tiers[i - 1].1]) } else { None };
            let mut out = [0i64; 2];
            for j in 0..2 {
                let mut v = bounds[j] * rate / 100;
                if v.abs() > limits[j].abs() {
                    v = limits[j];
                }
                if let Some(p) = prev
                    && v.abs() < p[j].abs()
                {
                    v = p[j];
                }
                out[j] = v;
            }
            tiers[i] = (out[0], out[1]);
        }
        JudgeWindows { pg: tiers[0], gr: tiers[1], gd: tiers[2], bd: self.bd, ms: self.ms }
    }

    /// One window bound, mirroring the reference implementation's
    /// `JudgeWindow.getTime(type, judge, early)`: `judge` is the judge index (0 = PG .. 4 = MS) and
    /// `early` picks the EARLY upper bound instead of the LATE lower bound. An index the table does
    /// not carry yields 0, exactly like the reference's array bounds check — which is why
    /// `note_type` matters: the long-note end tables have no fifth (空POOR) pair.
    pub fn get_time(&self, note_type: NoteType, judge: usize, early: bool) -> i64 {
        let pair = match judge {
            0 => Some(self.pg),
            1 => Some(self.gr),
            2 => Some(self.gd),
            3 => Some(self.bd),
            4 => self.ms.filter(|_| note_type.has_empty_poor_window()),
            _ => None,
        };
        match pair {
            Some(p) if early => p.1,
            Some(p) => p.0,
            None => 0,
        }
    }

    /// Whether `dmtime` sits inside the MS (空POOR) band. The reference implementation tests window index 4 on its own
    /// for an already-judged note (`JudgeManager.java:400-401`), rather than walking the
    /// PG/GR/GD/BD ladder first. Tables without an MS pair (long-note ends) never match.
    pub fn in_ms_band(&self, dmtime: i64) -> bool {
        self.ms.is_some_and(|w| dmtime >= w.0 && dmtime <= w.1)
    }

    /// Classify a timing delta. The MS band yields [`Judge::Miss`] — the reference implementation's judge code 5
    /// (空POOR, `JudgeManager.java:404` maps window index 4 to code 5) — and `None` means the press
    /// reached no window at all. Tables without an MS pair (long-note ends) never yield `Miss`.
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
        } else if self.ms.is_some_and(inw) {
            Some(Judge::Miss)
        } else {
            None
        }
    }
}

/// judgerank percent for the NORMAL window rule, indexed by `#RANK`
/// (0..4 = VERYHARD/HARD/NORMAL/EASY/VERYEASY).
const RANK_TABLE: [i32; 5] = [25, 50, 75, 100, 125];

/// judgerank percent used when `#RANK` is missing or out of range, and the base that
/// `#DEFEXRANK` scales (reference implementation `JudgeWindowRule.NORMAL.judgerank[2][1]`).
const NORMAL_JUDGERANK: i32 = 75;

/// `#RANK` index → judgerank percent (`BMSPlayerRule.java:62`). Out-of-range falls back to
/// NORMAL (75) — the reference implementation does not clamp to the table ends.
pub fn rank_to_judgerank(rank: i32) -> i32 {
    if rank < 0 { NORMAL_JUDGERANK } else { RANK_TABLE.get(rank as usize).copied().unwrap_or(NORMAL_JUDGERANK) }
}

/// Effective judgerank percent for a chart (`BMSPlayerRule.java:62-63`). A chart carrying
/// `#DEFEXRANK` switches to the `BMS_DEFEXRANK` branch, which never consults the `#RANK` table:
/// a positive value scales the NORMAL judgerank, anything else falls straight back to NORMAL (75).
/// Only a chart without `#DEFEXRANK` uses the `#RANK` table.
pub fn judgerank_for(rank: i32, defexrank: Option<f64>) -> i32 {
    match defexrank {
        Some(d) if d > 0.0 => ((d * NORMAL_JUDGERANK as f64) / 100.0) as i32,
        Some(_) => NORMAL_JUDGERANK,
        None => rank_to_judgerank(rank),
    }
}

#[cfg(test)]
mod windows_tests {
    use super::*;
    use rbms_model::Mode;

    const W: JudgeWindows = JudgeWindows::SEVENKEY_NOTE;

    #[test]
    fn sevenkeys_note_row_matches_judgeproperty_line_23() {
        assert_eq!(W.pg, (-20_000, 20_000));
        assert_eq!(W.gr, (-60_000, 60_000));
        assert_eq!(W.gd, (-150_000, 150_000));
        assert_eq!(W.bd, (-280_000, 220_000));
        assert_eq!(W.ms, Some((-150_000, 500_000)));
    }

    #[test]
    fn fivekeys_note_row_matches_judgeproperty_line_12() {
        let w = JudgeProperty::FIVEKEYS.note;
        assert_eq!(w.pg, (-20_000, 20_000));
        assert_eq!(w.gr, (-50_000, 50_000));
        assert_eq!(w.gd, (-100_000, 100_000));
        assert_eq!(w.bd, (-150_000, 150_000));
        assert_eq!(w.ms, Some((-150_000, 500_000)));
    }

    #[test]
    fn pms_note_row_matches_judgeproperty_line_34() {
        let w = JudgeProperty::PMS.note;
        assert_eq!(w.gd, (-117_000, 117_000));
        assert_eq!(w.bd, (-183_000, 183_000));
        assert_eq!(w.ms, Some((-175_000, 500_000)));
    }

    #[test]
    fn keyboard_note_row_matches_judgeproperty_line_45() {
        let w = JudgeProperty::KEYBOARD.note;
        assert_eq!(w.pg, (-30_000, 30_000));
        assert_eq!(w.gr, (-90_000, 90_000));
        assert_eq!(w.gd, (-200_000, 200_000));
        assert_eq!(w.bd, (-320_000, 240_000));
        assert_eq!(w.ms, Some((-200_000, 650_000)));
    }

    #[test]
    fn sevenkeys_scratch_row_is_wider_than_the_key_row() {
        let s = JudgeProperty::SEVENKEYS.scratch;
        assert_eq!(s.pg, (-30_000, 30_000));
        assert_eq!(s.gr, (-70_000, 70_000));
        assert_eq!(s.gd, (-160_000, 160_000));
        assert_eq!(s.bd, (-290_000, 230_000));
        assert_eq!(s.ms, Some((-160_000, 500_000)));
    }

    #[test]
    fn sevenkeys_ln_end_row_is_the_7k_table_not_the_5k_one() {
        let l = JudgeWindows::SEVENKEY_LN_END;
        assert_eq!(l.pg, (-120_000, 120_000));
        assert_eq!(l.gr, (-160_000, 160_000));
        assert_eq!(l.gd, (-200_000, 200_000));
        assert_eq!(l.bd, (-280_000, 220_000));
        assert_eq!(l.ms, None, "the reference implementation's longnote table has only four pairs");
    }

    #[test]
    fn sevenkeys_longscratch_row_matches_judgeproperty_line_27() {
        let l = JudgeProperty::SEVENKEYS.ln_scratch_end;
        assert_eq!(l.pg, (-130_000, 130_000));
        assert_eq!(l.gr, (-170_000, 170_000));
        assert_eq!(l.gd, (-210_000, 210_000));
        assert_eq!(l.bd, (-290_000, 230_000));
        assert_eq!(l.ms, None);
    }

    #[test]
    fn pms_ln_end_row_matches_judgeproperty_line_36() {
        let l = JudgeProperty::PMS.ln_end;
        assert_eq!(l.gd, (-217_000, 217_000));
        assert_eq!(l.bd, (-283_000, 283_000));
    }

    #[test]
    fn ln_end_out_of_range_has_no_empty_poor_band() {
        assert_eq!(JudgeWindows::SEVENKEY_LN_END.judge(300_000), None);
        assert_eq!(JudgeWindows::SEVENKEY_LN_END.judge(-300_000), None);
    }

    #[test]
    fn longnote_margin_is_zero_for_beat_modes_and_200ms_for_pms() {
        assert_eq!(JudgeProperty::SEVENKEYS.longnote_margin, 0);
        assert_eq!(JudgeProperty::FIVEKEYS.longnote_margin, 0);
        assert_eq!(JudgeProperty::KEYBOARD.longnote_margin, 0);
        assert_eq!(JudgeProperty::PMS.longnote_margin, 200_000);
    }

    #[test]
    fn combo_table_keeps_empty_poor_only_on_seven_keys() {
        assert_eq!(JudgeProperty::SEVENKEYS.combo, [true, true, true, false, false, true]);
        assert_eq!(JudgeProperty::FIVEKEYS.combo, [true, true, true, false, false, false]);
        assert_eq!(JudgeProperty::PMS.combo, [true, true, true, false, false, false]);
    }

    #[test]
    fn pms_does_not_vanish_a_bad_and_misses_once() {
        assert_eq!(JudgeProperty::PMS.judge_vanish, [true, true, true, false, true, false]);
        assert_eq!(JudgeProperty::PMS.miss_condition, MissCondition::One);
        assert_eq!(JudgeProperty::SEVENKEYS.miss_condition, MissCondition::Always);
    }

    #[test]
    fn for_mode_maps_beat_five_and_ten_to_fivekeys() {
        assert_eq!(JudgeProperty::for_mode(&Mode::BEAT_5K).note.gd, (-100_000, 100_000));
        assert_eq!(JudgeProperty::for_mode(&Mode::BEAT_10K).note.gd, (-100_000, 100_000));
        assert_eq!(JudgeProperty::for_mode(&Mode::BEAT_7K).note.gd, (-150_000, 150_000));
        assert_eq!(JudgeProperty::for_mode(&Mode::BEAT_14K).note.gd, (-150_000, 150_000));
        assert_eq!(JudgeProperty::for_mode(&Mode::POPN_9K).note.gd, (-117_000, 117_000));
    }

    #[test]
    fn candidate_gate_spans_note_and_scratch_bounds() {
        assert_eq!(JudgeProperty::SEVENKEYS.candidate_gate(), (-290_000, 500_000));
        assert_eq!(JudgeProperty::FIVEKEYS.candidate_gate(), (-160_000, 500_000));
    }

    #[test]
    fn judge_returns_miss_for_the_ms_band() {
        assert_eq!(W.judge(0), Some(Judge::PerfectGreat));
        assert_eq!(W.judge(220_000), Some(Judge::Bad), "early BD edge");
        assert_eq!(W.judge(220_001), Some(Judge::Miss), "past BD on the early side is 空POOR");
        assert_eq!(W.judge(500_000), Some(Judge::Miss), "MS early edge inclusive");
        assert_eq!(W.judge(500_001), None);
        assert_eq!(W.judge(-280_000), Some(Judge::Bad), "late BD edge");
        assert_eq!(W.judge(-280_001), None, "there is no late 空POOR: ms.0 == -150_000 sits inside GD");
    }

    #[test]
    fn scaled_50_halves_pg_gr_gd_bd_and_fixes_ms() {
        let s = W.scaled(50);
        assert_eq!(s.pg, (-10_000, 10_000));
        assert_eq!(s.gr, (-30_000, 30_000));
        assert_eq!(s.gd, (-75_000, 75_000));
        assert_eq!(s.bd, (-140_000, 110_000));
        assert_eq!(s.ms, W.ms, "reference implementation fixjudge[4] keeps the MS band at 100%");
    }

    #[test]
    fn scaled_at_zero_percent_collapses_every_scaling_tier_but_keeps_ms() {
        let w = W.scaled(0);
        assert_eq!(w.pg, (0, 0));
        assert_eq!(w.gr, (0, 0));
        assert_eq!(w.gd, (0, 0));
        assert_eq!(w.bd, (0, 0));
        assert_eq!(w.ms, Some((-150_000, 500_000)), "the MS band is fixjudge and never scales");
        assert_eq!(w.judge(0), Some(Judge::PerfectGreat), "only a dead-on press still lands");
        assert_eq!(w.judge(1_000), Some(Judge::Miss), "1ms off falls through to the MS band");
        assert_eq!(judgerank_for(3, Some(1.0)), 0, "#DEFEXRANK 1 truncates to judgerank 0");
    }

    #[test]
    fn with_window_rate_scales_only_the_first_three_tiers() {
        let w = W.with_window_rate([50, 50, 50]);
        assert_eq!(w.pg, (-10_000, 10_000));
        assert_eq!(w.gr, (-30_000, 30_000));
        assert_eq!(w.gd, (-75_000, 75_000));
        assert_eq!(w.bd, W.bd, "BAD is never touched by JUDGE WIDTH");
        assert_eq!(w.ms, W.ms, "the 空POOR band is never touched either");
    }

    #[test]
    fn with_window_rate_clamps_each_tier_to_the_bad_bounds() {
        let w = W.with_window_rate([100, 100, 300]);
        assert_eq!(w.gd, (-280_000, 220_000));
    }

    #[test]
    fn with_window_rate_never_lets_a_tier_shrink_below_the_previous_one() {
        let w = W.with_window_rate([100, 0, 100]);
        assert_eq!(w.gr, (-20_000, 20_000));
        let w2 = W.with_window_rate([100, 100, 10]);
        assert_eq!(w2.gd, (-60_000, 60_000));
    }

    #[test]
    fn with_window_rate_100_is_identity() {
        assert_eq!(W.with_window_rate([100, 100, 100]), W);
    }

    #[test]
    fn rank_table_exact_values() {
        assert_eq!(rank_to_judgerank(0), 25, "VERYHARD");
        assert_eq!(rank_to_judgerank(1), 50, "HARD");
        assert_eq!(rank_to_judgerank(2), 75, "NORMAL");
        assert_eq!(rank_to_judgerank(3), 100, "EASY");
        assert_eq!(rank_to_judgerank(4), 125, "VERYEASY");
    }

    #[test]
    fn rank_out_of_range_falls_back_to_normal_75() {
        assert_eq!(rank_to_judgerank(-1), 75);
        assert_eq!(rank_to_judgerank(5), 75);
        assert_eq!(rank_to_judgerank(i32::MAX), 75);
        assert_eq!(rank_to_judgerank(i32::MIN), 75);
    }

    #[test]
    fn defexrank_scales_the_normal_judgerank() {
        assert_eq!(judgerank_for(3, Some(100.0)), 75);
        assert_eq!(judgerank_for(3, Some(200.0)), 150);
        assert_eq!(judgerank_for(3, Some(50.0)), 37, "37.5 truncates to 37");
    }

    #[test]
    fn defexrank_zero_or_negative_falls_back_to_normal_75_ignoring_rank() {
        assert_eq!(judgerank_for(0, Some(0.0)), 75, "#DEFEXRANK 0 is NORMAL, not #RANK 0 (25)");
        assert_eq!(judgerank_for(4, Some(-10.0)), 75, "negative #DEFEXRANK is NORMAL, not #RANK 4 (125)");
        assert_eq!(judgerank_for(1, None), 50, "no #DEFEXRANK, so the #RANK table applies");
    }
}
