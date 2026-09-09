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

/// Number of judge indices a [`JudgeWindowRule`] covers: PG, GR, GD, BD and the 空POOR band.
const WINDOW_RULE_INDEX_COUNT: usize = 5;

/// Number of `#RANK` levels a [`JudgeWindowRule`] tabulates: VERYHARD, HARD, NORMAL, EASY, VERYEASY.
const RANK_COUNT: usize = 5;

/// The two bounds of one window pair: index 0 is the LATE lower bound, index 1 the EARLY upper one.
const BOUNDS_PER_PAIR: usize = 2;

/// How many judge indices `JudgeWindowRule.create` clamps between its fixed neighbours
/// (`JudgeProperty.java:239`, `Math.min(org.length, 4)`): PG, GR, GD and BD. Both rules fix the
/// 空POOR band, so it is never clamped.
const CLAMPED_INDEX_COUNT: usize = 4;

/// Number of JUDGE WIDTH tiers the user can widen or narrow (`JudgeProperty.java:263`): PG, GR, GD.
const JUDGE_WIDTH_TIER_COUNT: usize = 3;

/// The judgerank percentage that leaves a window at its tabulated width.
const FIXED_JUDGERANK_PERCENT: i32 = 100;

/// JUDGE WIDTH percentage that leaves a tier at the width its table states, the rates the reference
/// builds a window set with when the player set no custom judge (`JudgeManager.java:168-174`).
pub const UNMODIFIED_JUDGE_WIDTH_RATES: [i32; JUDGE_WIDTH_TIER_COUNT] = [100; JUDGE_WIDTH_TIER_COUNT];

/// Denominator of every percentage in this module.
const PERCENT_DENOMINATOR: i64 = 100;

/// Column of `JudgeWindowRule.judgerank` that carries a `#RANK`'s scalar judgerank
/// (`BMSPlayerRule.java:62`). It is the GREAT column, the one neither rule fixes.
const JUDGERANK_SCALAR_COLUMN: usize = 1;

/// Row of `JudgeWindowRule.judgerank` for NORMAL `#RANK`: the fallback for an out-of-range rank and
/// the base `#DEFEXRANK` scales (`BMSPlayerRule.java:62-63`).
const NORMAL_RANK_ROW: usize = 2;

/// Judge index of PERFECT GREAT inside a timing table.
pub(crate) const PGREAT_JUDGE_INDEX: usize = 0;

/// Judge index of GREAT inside a timing table, the pair `JudgeAlgorithm::Score` compares against.
pub(crate) const GREAT_JUDGE_INDEX: usize = 1;

/// Judge index of GOOD inside a timing table, the pair `JudgeAlgorithm::Combo` and the
/// `MissCondition::ONE` candidate filter compare against.
pub(crate) const GOOD_JUDGE_INDEX: usize = 2;

/// Judge index of BAD inside a timing table, the ceiling every JUDGE WIDTH tier is clamped to.
pub(crate) const BAD_JUDGE_INDEX: usize = 3;

/// Judge index of the 空POOR band, present only in the note and scratch tables.
pub(crate) const EMPTY_POOR_JUDGE_INDEX: usize = 4;

/// The judgment each window index yields, in the reference implementation's judge-code order.
const BAND_JUDGE: [Judge; WINDOW_RULE_INDEX_COUNT] = [Judge::PerfectGreat, Judge::Great, Judge::Good, Judge::Bad, Judge::Miss];

/// Reference implementation `JudgeProperty.JudgeWindowRule` (`JudgeProperty.java:209-211`): how one
/// mode turns a judgerank into per-judge window percentages, and which windows it refuses to move.
///
/// `Normal` scales PG/GR/GD/BD together and fixes only the 空POOR band. `Pms` fixes PG, BD and
/// 空POOR, scales GR and GD alone, and then clamps the scaled pair so it can neither be narrower
/// than PG nor wider than BAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JudgeWindowRule {
    Normal,
    Pms,
}

impl JudgeWindowRule {
    /// Both rules, in the reference implementation's declaration order.
    pub const ALL: [JudgeWindowRule; 2] = [JudgeWindowRule::Normal, JudgeWindowRule::Pms];

    /// `JudgeProperty.java:210` NORMAL judgerank table. Rows are `#RANK`
    /// (VERYHARD, HARD, NORMAL, EASY, VERYEASY), columns are judge index (PG, GR, GD, BD, MS).
    pub const NORMAL_JUDGERANK: [[i32; WINDOW_RULE_INDEX_COUNT]; RANK_COUNT] =
        [[25, 25, 25, 25, 100], [50, 50, 50, 50, 100], [75, 75, 75, 75, 100], [100, 100, 100, 100, 100], [125, 125, 125, 125, 100]];

    /// `JudgeProperty.java:210` NORMAL fixjudge: only the 空POOR band ignores judgerank.
    pub const NORMAL_FIXJUDGE: [bool; WINDOW_RULE_INDEX_COUNT] = [false, false, false, false, true];

    /// `JudgeProperty.java:211` PMS judgerank table, laid out like [`NORMAL_JUDGERANK`](Self::NORMAL_JUDGERANK).
    pub const PMS_JUDGERANK: [[i32; WINDOW_RULE_INDEX_COUNT]; RANK_COUNT] =
        [[100, 33, 33, 100, 100], [100, 50, 50, 100, 100], [100, 70, 70, 100, 100], [100, 100, 100, 100, 100], [100, 133, 133, 100, 100]];

    /// `JudgeProperty.java:211` PMS fixjudge: PG, BAD and the 空POOR band ignore judgerank.
    pub const PMS_FIXJUDGE: [bool; WINDOW_RULE_INDEX_COUNT] = [true, false, false, true, true];

    /// This rule's judgerank table.
    pub fn judgerank_table(self) -> [[i32; WINDOW_RULE_INDEX_COUNT]; RANK_COUNT] {
        match self {
            JudgeWindowRule::Normal => Self::NORMAL_JUDGERANK,
            JudgeWindowRule::Pms => Self::PMS_JUDGERANK,
        }
    }

    /// Which judge indices this rule holds at their tabulated width whatever the judgerank.
    pub fn fixjudge(self) -> [bool; WINDOW_RULE_INDEX_COUNT] {
        match self {
            JudgeWindowRule::Normal => Self::NORMAL_FIXJUDGE,
            JudgeWindowRule::Pms => Self::PMS_FIXJUDGE,
        }
    }

    /// Spread one scalar judgerank over the five judge indices, mirroring
    /// `JudgeWindowRule.getJudgeRank` (`JudgeProperty.java:222-224`): a fixed index takes 100.
    pub fn judgerank_per_index(self, judgerank: i32) -> [i32; WINDOW_RULE_INDEX_COUNT] {
        self.fixjudge().map(|fixed| if fixed { FIXED_JUDGERANK_PERCENT } else { judgerank })
    }

    /// The rule `mode` is judged with (`JudgeProperty.java:21, 32, 43, 54`): every reference row but
    /// PMS uses [`JudgeWindowRule::Normal`].
    pub fn for_mode(mode: &Mode) -> JudgeWindowRule {
        match mode.name {
            "POPN_9K" => JudgeWindowRule::Pms,
            _ => JudgeWindowRule::Normal,
        }
    }

    /// The judgerank a chart falls back to when it carries no usable `#RANK`, and the base a
    /// positive `#DEFEXRANK` scales (`BMSPlayerRule.java:62-63`). 75 under `Normal`, 70 under `Pms`.
    pub fn normal_judgerank(self) -> i32 {
        self.judgerank_table()[NORMAL_RANK_ROW][JUDGERANK_SCALAR_COLUMN]
    }

    /// Scalar judgerank percent for a chart `#RANK` (`BMSPlayerRule.java:62`). A rank outside 0..5
    /// falls back to NORMAL. PMS reads its own column, so `#RANK 0` is 33 there and 25 under
    /// [`Normal`](JudgeWindowRule::Normal).
    pub fn judgerank_for_rank(self, rank: i32) -> i32 {
        let row = usize::try_from(rank).ok().filter(|r| *r < RANK_COUNT).unwrap_or(NORMAL_RANK_ROW);
        self.judgerank_table()[row][JUDGERANK_SCALAR_COLUMN]
    }

    /// Effective judgerank percent for a chart under this rule (`BMSPlayerRule.java:62-63`). A chart
    /// carrying `#DEFEXRANK` never consults the `#RANK` table: a positive value scales this rule's
    /// NORMAL judgerank, anything else falls straight back to it.
    pub fn judgerank_for(self, rank: i32, defexrank: Option<f64>) -> i32 {
        match defexrank {
            Some(d) if d > 0.0 => ((d * self.normal_judgerank() as f64) / PERCENT_DENOMINATOR as f64) as i32,
            Some(_) => self.normal_judgerank(),
            None => self.judgerank_for_rank(rank),
        }
    }

    /// Build a timing table at `judgerank`, mirroring `JudgeWindowRule.create`
    /// (`JudgeProperty.java:230-260`) up to but excluding the JUDGE WIDTH pass, which stays in
    /// [`JudgeWindows::with_window_rate`].
    ///
    /// Two steps. First every non-fixed band is scaled by its own judgerank percentage. Then each of
    /// the four scalable indices is clamped between its fixed neighbours: it can be no narrower than
    /// the nearest fixed index below it and no wider than the nearest fixed index above it, which is
    /// what keeps PMS GREAT/GOOD inside PG and BAD. `Normal` fixes nothing below index 4, so the
    /// clamp is a no-op there and this reproduces [`JudgeWindows::scaled`].
    ///
    /// A negative judgerank is clamped to 0, the guard this engine has always had; the reference
    /// does not clamp because none of its judgerank sources go below zero.
    pub fn create(self, org: &JudgeWindows, judgerank: [i32; WINDOW_RULE_INDEX_COUNT]) -> JudgeWindows {
        let fixjudge = self.fixjudge();
        let src = [org.pg, org.gr, org.gd, org.bd, org.ms.unwrap_or((0, 0))];
        let mut judge = [0i64; WINDOW_RULE_INDEX_COUNT * BOUNDS_PER_PAIR];
        for i in 0..org.band_count() {
            let rank = judgerank[i].max(0) as i64;
            let bounds = [src[i].0, src[i].1];
            for j in 0..BOUNDS_PER_PAIR {
                judge[i * BOUNDS_PER_PAIR + j] = if fixjudge[i] { bounds[j] } else { bounds[j] * rank / PERCENT_DENOMINATOR };
            }
        }

        let mut fixmin: Option<usize> = None;
        for i in 0..CLAMPED_INDEX_COUNT {
            if fixjudge[i] {
                fixmin = Some(i);
                continue;
            }
            let fixmax = (i + 1..CLAMPED_INDEX_COUNT).find(|&j| fixjudge[j]);
            for j in 0..BOUNDS_PER_PAIR {
                let at = i * BOUNDS_PER_PAIR + j;
                if let Some(floor) = fixmin
                    && judge[at].abs() < judge[floor * BOUNDS_PER_PAIR + j].abs()
                {
                    judge[at] = judge[floor * BOUNDS_PER_PAIR + j];
                }
                if let Some(ceiling) = fixmax
                    && judge[at].abs() > judge[ceiling * BOUNDS_PER_PAIR + j].abs()
                {
                    judge[at] = judge[ceiling * BOUNDS_PER_PAIR + j];
                }
            }
        }

        let pair = |i: usize| (judge[i * BOUNDS_PER_PAIR], judge[i * BOUNDS_PER_PAIR + 1]);
        JudgeWindows { pg: pair(0), gr: pair(1), gd: pair(2), bd: pair(3), ms: org.ms.map(|_| pair(4)) }
    }
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
    /// scratch lookup can never hit a zero-length window. An empty reference table judges everything
    /// as PERFECT GREAT, so this is a deliberate divergence rather than a copy — see
    /// `docs/acknowledge/reference-divergences.md`.
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

    /// Reference implementation `JudgeProperty.KEYBOARD` (`JudgeProperty.java:45-55`). Its two scratch
    /// tables stand in for the reference's empty ones, exactly as [`JudgeProperty::PMS`] does.
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
    /// uses PMS and KEYBOARD_24K uses KEYBOARD. Used when the data file has no row for the mode, and
    /// as the baseline the data file's parity guard compares against.
    pub fn defaults_for_mode(mode: &Mode) -> JudgeProperty {
        match mode.name {
            "BEAT_5K" | "BEAT_10K" => Self::FIVEKEYS,
            "POPN_9K" => Self::PMS,
            crate::data::KEYBOARD_24K_KEY => Self::KEYBOARD,
            _ => Self::SEVENKEYS,
        }
    }

    /// Widest candidate window across the NOTE and SCRATCH tables, as `(late, early)`. The reference implementation
    /// seeds `mjudgestart`/`mjudgeend` at 0 and folds both tables' bounds in
    /// (`JudgeManager.java:189-197`), so the gate is chart-global, not per-lane.
    pub fn candidate_gate(&self) -> (i64, i64) {
        candidate_gate_across(&self.note, &self.scratch)
    }

    /// All four timing tables this row judges against, mirroring the reference implementation's
    /// `JudgeWindow` constructor (`JudgeProperty.java:168-173`): every table is built from the same
    /// `judgerank` under the same `rule` and from the **key** JUDGE WIDTH rates, because
    /// `JudgeManager.java:198` builds the one window set the whole engine judges against with
    /// `keyJudgeWindowRate` alone.
    ///
    /// `scratch_rates` never reach a judgment. The reference spends them on `smjudge`
    /// (`JudgeManager.java:185-197`), the scratch half of the chart-global candidate gate, which is
    /// what [`JudgeWindowSet::candidate_gate`] carries.
    pub fn window_set(
        &self,
        rule: JudgeWindowRule,
        judgerank: i32,
        key_rates: [i32; JUDGE_WIDTH_TIER_COUNT],
        scratch_rates: [i32; JUDGE_WIDTH_TIER_COUNT],
    ) -> JudgeWindowSet {
        let per_index = rule.judgerank_per_index(judgerank);
        let build = |org: &JudgeWindows, rates: [i32; JUDGE_WIDTH_TIER_COUNT]| rule.create(org, per_index).with_window_rate(rates);
        let note = build(&self.note, key_rates);
        JudgeWindowSet {
            candidate_gate: candidate_gate_across(&note, &build(&self.scratch, scratch_rates)),
            note,
            scratch: build(&self.scratch, key_rates),
            ln_end: build(&self.ln_end, key_rates),
            ln_scratch_end: build(&self.ln_scratch_end, key_rates),
        }
    }
}

/// Widest candidate window across a key and a scratch table, as `(late, early)`. Seeded at 0 like
/// the reference implementation's `mjudgestart`/`mjudgeend` (`JudgeManager.java:185-197`), so the
/// gate is chart-global rather than per-lane.
pub fn candidate_gate_across(note: &JudgeWindows, scratch: &JudgeWindows) -> (i64, i64) {
    let mut start = 0;
    let mut end = 0;
    for w in [note, scratch] {
        for pair in w.pairs() {
            start = start.min(pair.0);
            end = end.max(pair.1);
        }
    }
    (start, end)
}

/// The four timing tables one play session judges against, already scaled by the chart's judgerank
/// and the user's JUDGE WIDTH rates, plus the candidate gate the scratch rates feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeWindowSet {
    pub note: JudgeWindows,
    pub scratch: JudgeWindows,
    pub ln_end: JudgeWindows,
    pub ln_scratch_end: JudgeWindows,
    /// How far either side of a press the engine looks for a candidate note. The only place the
    /// scratch JUDGE WIDTH rates reach.
    pub candidate_gate: (i64, i64),
}

impl JudgeWindowSet {
    /// The tables `mode` judges against, taking both its [`JudgeProperty`] row and its
    /// [`JudgeWindowRule`] from the mode.
    pub fn for_mode(mode: &Mode, judgerank: i32, key_rates: [i32; JUDGE_WIDTH_TIER_COUNT], scratch_rates: [i32; JUDGE_WIDTH_TIER_COUNT]) -> JudgeWindowSet {
        JudgeProperty::for_mode(mode).window_set(JudgeWindowRule::for_mode(mode), judgerank, key_rates, scratch_rates)
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

    /// How many timing bands this table carries: five for the note and scratch rows, four for the
    /// long-note end rows, which have no 空POOR pair. [`judge_code`](Self::judge_code) returns this
    /// value when a delta lands in none of them, mirroring `mjudge.length / 2` in
    /// `JudgeWindow.getJudge` (`JudgeProperty.java:187-188`).
    pub fn band_count(&self) -> usize {
        CLAMPED_INDEX_COUNT + usize::from(self.ms.is_some())
    }

    /// Apply a `judgerank` percentage (100 = default) under [`JudgeWindowRule::Normal`], the rule
    /// every mode but PMS uses. Kept as the compatibility entry point for callers that have no mode
    /// to hand; [`JudgeWindowRule::create`] is the general form.
    pub fn scaled(&self, judgerank_percent: i32) -> JudgeWindows {
        JudgeWindowRule::Normal.create(self, JudgeWindowRule::Normal.judgerank_per_index(judgerank_percent))
    }

    /// Apply the user JUDGE WIDTH rates (percent, `[PG, GR, GD]`), reference implementation
    /// `JudgeWindowRule.create` lines 262-273. Only the first three tiers scale; each bound is then
    /// clamped so it never exceeds the BAD bound (`judge[6 + j]` in the Java, i.e. index 3) and
    /// never shrinks below the tier before it. BAD and MS are untouched.
    pub fn with_window_rate(&self, rates: [i32; JUDGE_WIDTH_TIER_COUNT]) -> JudgeWindows {
        let mut tiers = [self.pg, self.gr, self.gd];
        let limit = self.bd;
        for i in 0..JUDGE_WIDTH_TIER_COUNT {
            let rate = rates[i].max(0) as i64;
            let bounds = [tiers[i].0, tiers[i].1];
            let limits = [limit.0, limit.1];
            let prev = if i > 0 { Some([tiers[i - 1].0, tiers[i - 1].1]) } else { None };
            let mut out = [0i64; BOUNDS_PER_PAIR];
            for j in 0..BOUNDS_PER_PAIR {
                let mut v = bounds[j] * rate / PERCENT_DENOMINATOR;
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
            PGREAT_JUDGE_INDEX => Some(self.pg),
            GREAT_JUDGE_INDEX => Some(self.gr),
            GOOD_JUDGE_INDEX => Some(self.gd),
            BAD_JUDGE_INDEX => Some(self.bd),
            EMPTY_POOR_JUDGE_INDEX => self.ms.filter(|_| note_type.has_empty_poor_window()),
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

    /// Whether `dmtime` sits inside the GOOD band. The `MissCondition::ONE` candidate filter tests
    /// exactly this pair on a note that has already been judged once
    /// (`JudgeManager.java:397-399`, `getTime(type, 2, early)`).
    pub fn in_good_band(&self, dmtime: i64) -> bool {
        dmtime >= self.gd.0 && dmtime <= self.gd.1
    }

    /// Classify a timing delta into the reference implementation's window index, mirroring
    /// `JudgeWindow.getJudge` (`JudgeProperty.java:184-189`): the index of the first band that
    /// contains `dmtime`, or [`band_count`](Self::band_count) when no band does. Index 4 is the
    /// 空POOR band, which the press path remaps to judge code 5 (`JudgeManager.java:404`).
    pub fn judge_code(&self, dmtime: i64) -> usize {
        let inw = |w: (i64, i64)| dmtime >= w.0 && dmtime <= w.1;
        if inw(self.pg) {
            PGREAT_JUDGE_INDEX
        } else if inw(self.gr) {
            GREAT_JUDGE_INDEX
        } else if inw(self.gd) {
            GOOD_JUDGE_INDEX
        } else if inw(self.bd) {
            BAD_JUDGE_INDEX
        } else if self.ms.is_some_and(inw) {
            EMPTY_POOR_JUDGE_INDEX
        } else {
            self.band_count()
        }
    }

    /// Classify a timing delta. The MS band yields [`Judge::Miss`] — the reference implementation's judge code 5
    /// (空POOR, `JudgeManager.java:404` maps window index 4 to code 5) — and `None` means the press
    /// reached no window at all. Tables without an MS pair (long-note ends) never yield `Miss`.
    pub fn judge(&self, dmtime: i64) -> Option<Judge> {
        let code = self.judge_code(dmtime);
        if code >= self.band_count() { None } else { BAND_JUDGE.get(code).copied() }
    }
}

/// `#RANK` index -> judgerank percent under [`JudgeWindowRule::Normal`] (`BMSPlayerRule.java:62`).
/// Out-of-range falls back to NORMAL (75) — the reference implementation does not clamp to the
/// table ends. Use [`JudgeWindowRule::judgerank_for_rank`] for a mode whose rule is not NORMAL.
pub fn rank_to_judgerank(rank: i32) -> i32 {
    JudgeWindowRule::Normal.judgerank_for_rank(rank)
}

/// Effective judgerank percent for a chart under [`JudgeWindowRule::Normal`]
/// (`BMSPlayerRule.java:62-63`). See [`JudgeWindowRule::judgerank_for`] for the mode-aware form.
pub fn judgerank_for(rank: i32, defexrank: Option<f64>) -> i32 {
    JudgeWindowRule::Normal.judgerank_for(rank, defexrank)
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

    fn flat(w: JudgeWindows) -> Vec<i64> {
        let mut v = vec![w.pg.0, w.pg.1, w.gr.0, w.gr.1, w.gd.0, w.gd.1, w.bd.0, w.bd.1];
        if let Some(ms) = w.ms {
            v.push(ms.0);
            v.push(ms.1);
        }
        v
    }

    fn legacy_scaled(w: &JudgeWindows, judgerank_percent: i32) -> JudgeWindows {
        let p = judgerank_percent.max(0) as i64;
        let s = |b: (i64, i64)| (b.0 * p / 100, b.1 * p / 100);
        JudgeWindows { pg: s(w.pg), gr: s(w.gr), gd: s(w.gd), bd: s(w.bd), ms: w.ms }
    }

    const REFERENCE_ROWS: [(&str, JudgeProperty); 4] =
        [("FIVEKEYS", JudgeProperty::FIVEKEYS), ("SEVENKEYS", JudgeProperty::SEVENKEYS), ("PMS", JudgeProperty::PMS), ("KEYBOARD", JudgeProperty::KEYBOARD)];

    #[test]
    fn judge_property_tables_pin() {
        let five = JudgeProperty::FIVEKEYS;
        assert_eq!(flat(five.note), [-20_000, 20_000, -50_000, 50_000, -100_000, 100_000, -150_000, 150_000, -150_000, 500_000], "FIVEKEYS note");
        assert_eq!(flat(five.scratch), [-30_000, 30_000, -60_000, 60_000, -110_000, 110_000, -160_000, 160_000, -160_000, 500_000], "FIVEKEYS scratch");
        assert_eq!(flat(five.ln_end), [-120_000, 120_000, -150_000, 150_000, -200_000, 200_000, -250_000, 250_000], "FIVEKEYS longnote");
        assert_eq!(
            flat(five.ln_scratch_end),
            [-130_000, 130_000, -160_000, 160_000, -110_000, 110_000, -260_000, 260_000],
            "FIVEKEYS longscratch: the third pair really is narrower than the second in JudgeProperty.java:16, so it is copied unchanged"
        );
        assert_eq!((five.longnote_margin, five.longscratch_margin), (0, 0), "FIVEKEYS margins");
        assert_eq!(five.combo, [true, true, true, false, false, false], "FIVEKEYS combo");
        assert_eq!(five.judge_vanish, [true, true, true, true, true, false], "FIVEKEYS judgeVanish");
        assert_eq!(five.miss_condition, MissCondition::Always, "FIVEKEYS miss");

        let seven = JudgeProperty::SEVENKEYS;
        assert_eq!(flat(seven.note), [-20_000, 20_000, -60_000, 60_000, -150_000, 150_000, -280_000, 220_000, -150_000, 500_000], "SEVENKEYS note");
        assert_eq!(flat(seven.scratch), [-30_000, 30_000, -70_000, 70_000, -160_000, 160_000, -290_000, 230_000, -160_000, 500_000], "SEVENKEYS scratch");
        assert_eq!(flat(seven.ln_end), [-120_000, 120_000, -160_000, 160_000, -200_000, 200_000, -280_000, 220_000], "SEVENKEYS longnote");
        assert_eq!(flat(seven.ln_scratch_end), [-130_000, 130_000, -170_000, 170_000, -210_000, 210_000, -290_000, 230_000], "SEVENKEYS longscratch");
        assert_eq!((seven.longnote_margin, seven.longscratch_margin), (0, 0), "SEVENKEYS margins");
        assert_eq!(seven.combo, [true, true, true, false, false, true], "SEVENKEYS combo");
        assert_eq!(seven.judge_vanish, [true, true, true, true, true, false], "SEVENKEYS judgeVanish");
        assert_eq!(seven.miss_condition, MissCondition::Always, "SEVENKEYS miss");

        let pms = JudgeProperty::PMS;
        assert_eq!(flat(pms.note), [-20_000, 20_000, -50_000, 50_000, -117_000, 117_000, -183_000, 183_000, -175_000, 500_000], "PMS note");
        assert_eq!(flat(pms.ln_end), [-120_000, 120_000, -150_000, 150_000, -217_000, 217_000, -283_000, 283_000], "PMS longnote");
        assert_eq!((pms.longnote_margin, pms.longscratch_margin), (200_000, 0), "PMS margins");
        assert_eq!(pms.combo, [true, true, true, false, false, false], "PMS combo");
        assert_eq!(pms.judge_vanish, [true, true, true, false, true, false], "PMS judgeVanish: a BAD does not consume the note");
        assert_eq!(pms.miss_condition, MissCondition::One, "PMS miss");

        let keyboard = JudgeProperty::KEYBOARD;
        assert_eq!(flat(keyboard.note), [-30_000, 30_000, -90_000, 90_000, -200_000, 200_000, -320_000, 240_000, -200_000, 650_000], "KEYBOARD note");
        assert_eq!(flat(keyboard.ln_end), [-160_000, 25_000, -200_000, 75_000, -260_000, 140_000, -320_000, 240_000], "KEYBOARD longnote");
        assert_eq!((keyboard.longnote_margin, keyboard.longscratch_margin), (0, 0), "KEYBOARD margins");
        assert_eq!(keyboard.combo, [true, true, true, false, false, true], "KEYBOARD combo");
        assert_eq!(keyboard.judge_vanish, [true, true, true, true, true, false], "KEYBOARD judgeVanish");
        assert_eq!(keyboard.miss_condition, MissCondition::Always, "KEYBOARD miss");
    }

    #[test]
    fn rows_without_a_scratch_lane_stand_in_their_key_tables() {
        for (name, prop) in [("PMS", JudgeProperty::PMS), ("KEYBOARD", JudgeProperty::KEYBOARD)] {
            assert_eq!(prop.scratch, prop.note, "{name} scratch is empty in JudgeProperty.java, so the note table stands in");
            assert_eq!(prop.ln_scratch_end, prop.ln_end, "{name} longscratch is empty in JudgeProperty.java, so the longnote table stands in");
        }
    }

    #[test]
    fn judge_property_rows_reach_the_engine_through_the_data_file() {
        for (mode, expected) in [
            (Mode::BEAT_5K, JudgeProperty::FIVEKEYS),
            (Mode::BEAT_10K, JudgeProperty::FIVEKEYS),
            (Mode::BEAT_7K, JudgeProperty::SEVENKEYS),
            (Mode::BEAT_14K, JudgeProperty::SEVENKEYS),
            (Mode::POPN_9K, JudgeProperty::PMS),
        ] {
            let loaded = JudgeProperty::for_mode(&mode);
            assert_eq!(flat(loaded.note), flat(expected.note), "{} note", mode.name);
            assert_eq!(flat(loaded.scratch), flat(expected.scratch), "{} scratch", mode.name);
            assert_eq!(flat(loaded.ln_end), flat(expected.ln_end), "{} ln_end", mode.name);
            assert_eq!(flat(loaded.ln_scratch_end), flat(expected.ln_scratch_end), "{} ln_scratch_end", mode.name);
            assert_eq!(loaded.longnote_margin, expected.longnote_margin, "{} longnote margin", mode.name);
            assert_eq!(loaded.longscratch_margin, expected.longscratch_margin, "{} longscratch margin", mode.name);
            assert_eq!(loaded.combo, expected.combo, "{} combo", mode.name);
            assert_eq!(loaded.judge_vanish, expected.judge_vanish, "{} judgeVanish", mode.name);
            assert_eq!(loaded.miss_condition, expected.miss_condition, "{} miss", mode.name);
        }
    }

    #[test]
    fn window_rule_tables_pin() {
        assert_eq!(
            JudgeWindowRule::Normal.judgerank_table(),
            [[25, 25, 25, 25, 100], [50, 50, 50, 50, 100], [75, 75, 75, 75, 100], [100, 100, 100, 100, 100], [125, 125, 125, 125, 100]]
        );
        assert_eq!(JudgeWindowRule::Normal.fixjudge(), [false, false, false, false, true]);
        assert_eq!(
            JudgeWindowRule::Pms.judgerank_table(),
            [[100, 33, 33, 100, 100], [100, 50, 50, 100, 100], [100, 70, 70, 100, 100], [100, 100, 100, 100, 100], [100, 133, 133, 100, 100]]
        );
        assert_eq!(JudgeWindowRule::Pms.fixjudge(), [true, false, false, true, true]);
    }

    #[test]
    fn window_rule_for_mode_is_pms_only_for_popn() {
        assert_eq!(JudgeWindowRule::for_mode(&Mode::POPN_9K), JudgeWindowRule::Pms);
        for mode in [Mode::BEAT_5K, Mode::BEAT_7K, Mode::BEAT_10K, Mode::BEAT_14K] {
            assert_eq!(JudgeWindowRule::for_mode(&mode), JudgeWindowRule::Normal, "{}", mode.name);
        }
    }

    #[test]
    fn pms_judgerank_fixes_pg_bd_ms() {
        assert_eq!(JudgeWindowRule::Pms.judgerank_per_index(75), [100, 75, 75, 100, 100]);
        assert_eq!(JudgeWindowRule::Normal.judgerank_per_index(75), [75, 75, 75, 75, 100]);
    }

    #[test]
    fn the_scalar_judgerank_column_reproduces_each_rules_table_row() {
        for rule in JudgeWindowRule::ALL {
            for (rank, row) in rule.judgerank_table().into_iter().enumerate() {
                assert_eq!(rule.judgerank_per_index(rule.judgerank_for_rank(rank as i32)), row, "{rule:?} rank {rank}");
            }
        }
    }

    #[test]
    fn pms_reads_its_own_rank_column() {
        assert_eq!(JudgeWindowRule::Pms.judgerank_for_rank(0), 33, "VERYHARD is 33 on PMS, not 25");
        assert_eq!(JudgeWindowRule::Pms.judgerank_for_rank(2), 70, "NORMAL is 70 on PMS, not 75");
        assert_eq!(JudgeWindowRule::Pms.judgerank_for_rank(4), 133);
        assert_eq!(JudgeWindowRule::Pms.normal_judgerank(), 70);
        assert_eq!(JudgeWindowRule::Pms.judgerank_for_rank(5), 70, "an out-of-range rank falls back to that rule's NORMAL");
        assert_eq!(JudgeWindowRule::Pms.judgerank_for(3, Some(200.0)), 140, "#DEFEXRANK scales the PMS NORMAL judgerank");
        assert_eq!(JudgeWindowRule::Pms.judgerank_for(0, Some(0.0)), 70);
        assert_eq!(JudgeWindowRule::Normal.judgerank_for_rank(0), 25);
        assert_eq!(JudgeWindowRule::Normal.normal_judgerank(), 75);
    }

    #[test]
    fn the_free_judgerank_helpers_are_the_normal_rule() {
        for rank in -2..7 {
            assert_eq!(rank_to_judgerank(rank), JudgeWindowRule::Normal.judgerank_for_rank(rank), "rank {rank}");
            for defexrank in [None, Some(-1.0), Some(0.0), Some(50.0), Some(200.0)] {
                assert_eq!(judgerank_for(rank, defexrank), JudgeWindowRule::Normal.judgerank_for(rank, defexrank), "rank {rank} defexrank {defexrank:?}");
            }
        }
    }

    #[test]
    fn normal_rule_matches_legacy_scaled() {
        for (name, prop) in REFERENCE_ROWS {
            for (table, org) in [("note", prop.note), ("scratch", prop.scratch), ("ln_end", prop.ln_end), ("ln_scratch_end", prop.ln_scratch_end)] {
                for judgerank in [0, 25, 50, 70, 75, 100, 125, 133, 400] {
                    let rule = JudgeWindowRule::Normal.create(&org, JudgeWindowRule::Normal.judgerank_per_index(judgerank));
                    assert_eq!(rule, legacy_scaled(&org, judgerank), "{name} {table} at {judgerank}");
                    assert_eq!(rule, org.scaled(judgerank), "{name} {table} at {judgerank} through the compatibility wrapper");
                }
            }
        }
    }

    #[test]
    fn pms_create_clamps_great_to_the_pgreat_floor() {
        let org = JudgeProperty::PMS.note;
        let w = JudgeWindowRule::Pms.create(&org, JudgeWindowRule::Pms.judgerank_per_index(33));
        assert_eq!(w.pg, (-20_000, 20_000), "PG is fixjudge");
        assert_eq!(w.gr, (-20_000, 20_000), "GREAT would scale to 16500 but cannot be narrower than PG");
        assert_eq!(w.gd, (-38_610, 38_610), "GOOD scales to 117000 * 33 / 100 and stays between PG and BAD");
        assert_eq!(w.bd, (-183_000, 183_000), "BAD is fixjudge");
        assert_eq!(w.ms, Some((-175_000, 500_000)), "the 空POOR band is fixjudge");
    }

    #[test]
    fn pms_create_clamps_good_to_the_bad_ceiling() {
        let org = JudgeProperty::PMS.note;
        let wide = JudgeWindowRule::Pms.create(&org, JudgeWindowRule::Pms.judgerank_per_index(133));
        assert_eq!(wide.gr, (-66_500, 66_500), "at 133 GREAT stays between PG and BAD");
        assert_eq!(wide.gd, (-155_610, 155_610), "at 133 GOOD is still inside BAD");

        let wider = JudgeWindowRule::Pms.create(&org, JudgeWindowRule::Pms.judgerank_per_index(200));
        assert_eq!(wider.gr, (-100_000, 100_000));
        assert_eq!(wider.gd, (-183_000, 183_000), "GOOD would scale to 234000 but cannot exceed BAD");
        assert_eq!(wider.bd, (-183_000, 183_000));
    }

    #[test]
    fn ln_end_rule_does_not_panic_on_four_pairs() {
        let org = JudgeProperty::PMS.ln_end;
        let w = JudgeWindowRule::Pms.create(&org, JudgeWindowRule::Pms.judgerank_per_index(33));
        assert_eq!(w.pg, (-120_000, 120_000));
        assert_eq!(w.gr, (-120_000, 120_000), "GREAT scales to 49500 and is lifted to the PG floor");
        assert_eq!(w.gd, (-120_000, 120_000), "GOOD scales to 71610 and is lifted to the same floor");
        assert_eq!(w.bd, (-283_000, 283_000));
        assert_eq!(w.ms, None, "a four-pair table gains no 空POOR band");
    }

    #[test]
    fn band_count_and_judge_code_follow_the_table_length() {
        assert_eq!(W.band_count(), 5);
        assert_eq!(JudgeWindows::SEVENKEY_LN_END.band_count(), 4);

        assert_eq!(W.judge_code(0), 0);
        assert_eq!(W.judge_code(-20_000), 0, "PG late edge");
        assert_eq!(W.judge_code(-20_001), 1);
        assert_eq!(W.judge_code(-150_000), 2);
        assert_eq!(W.judge_code(220_000), 3, "BAD early edge");
        assert_eq!(W.judge_code(220_001), 4, "past BAD on the early side is the 空POOR band");
        assert_eq!(W.judge_code(500_001), 5, "no band at all yields the band count");

        let ln = JudgeWindows::SEVENKEY_LN_END;
        assert_eq!(ln.judge_code(-120_000), 0);
        assert_eq!(ln.judge_code(221_000), 4, "a four-pair table yields 4 for no band, not the 空POOR index");
        assert_eq!(ln.judge(221_000), None);
    }

    #[test]
    fn judge_code_and_judge_classify_the_same_deltas() {
        for w in [W, JudgeWindows::SEVENKEY_LN_END, JudgeProperty::PMS.note, JudgeProperty::KEYBOARD.ln_end] {
            for dmtime in [-700_000, -300_000, -150_000, -20_001, 0, 20_000, 60_001, 220_001, 500_000, 700_000] {
                let expected = match w.judge_code(dmtime) {
                    0 => Some(Judge::PerfectGreat),
                    1 => Some(Judge::Great),
                    2 => Some(Judge::Good),
                    3 => Some(Judge::Bad),
                    4 if w.band_count() > 4 => Some(Judge::Miss),
                    _ => None,
                };
                assert_eq!(w.judge(dmtime), expected, "{w:?} at {dmtime}");
            }
        }
    }

    #[test]
    fn in_good_band_is_the_good_pair_of_the_selected_table() {
        assert!(W.in_good_band(150_000));
        assert!(W.in_good_band(-150_000));
        assert!(!W.in_good_band(150_001));
        assert!(!W.in_good_band(-150_001));
        assert!(!W.in_good_band(220_000), "a BAD-only press is outside the GOOD band");
    }

    #[test]
    fn get_time_returns_zero_for_a_band_the_table_does_not_carry() {
        assert_eq!(W.get_time(NoteType::Note, EMPTY_POOR_JUDGE_INDEX, false), -150_000);
        assert_eq!(W.get_time(NoteType::Note, EMPTY_POOR_JUDGE_INDEX, true), 500_000);
        assert_eq!(JudgeWindows::SEVENKEY_LN_END.get_time(NoteType::LongNoteEnd, EMPTY_POOR_JUDGE_INDEX, true), 0);
        assert_eq!(W.get_time(NoteType::Note, BAD_JUDGE_INDEX, false), -280_000);
    }

    #[test]
    fn every_judged_table_takes_the_key_judge_width() {
        let set = JudgeProperty::SEVENKEYS.window_set(JudgeWindowRule::Normal, 100, [50, 100, 100], [100, 100, 100]);
        assert_eq!(set.note.pg, (-10_000, 10_000), "the key rate halves the note PG window");
        assert_eq!(set.ln_end.pg, (-60_000, 60_000), "the key rate also drives the long-note end table");
        assert_eq!(set.scratch.pg, (-15_000, 15_000), "JudgeManager.java:198 builds the scratch table from the key rate too");
        assert_eq!(set.ln_scratch_end.pg, (-65_000, 65_000));
    }

    #[test]
    fn the_scratch_judge_width_moves_only_the_candidate_gate() {
        let stock = JudgeProperty::SEVENKEYS.window_set(JudgeWindowRule::Normal, 100, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        let narrowed = JudgeProperty::SEVENKEYS.window_set(JudgeWindowRule::Normal, 100, UNMODIFIED_JUDGE_WIDTH_RATES, [50, 50, 50]);
        assert_eq!(narrowed.note, stock.note, "the scratch rate never reaches a judged table");
        assert_eq!(narrowed.scratch, stock.scratch);
        assert_eq!(narrowed.ln_end, stock.ln_end);
        assert_eq!(narrowed.ln_scratch_end, stock.ln_scratch_end);
        assert_eq!(stock.candidate_gate, candidate_gate_across(&stock.note, &JudgeProperty::SEVENKEYS.scratch));
        assert_eq!(narrowed.candidate_gate, stock.candidate_gate, "narrowing PG/GR/GD leaves BAD, which is what the gate folds");
    }

    #[test]
    fn a_scratch_press_of_a_widened_run_is_judged_by_the_key_window() {
        let set = JudgeProperty::SEVENKEYS.window_set(JudgeWindowRule::Normal, 100, UNMODIFIED_JUDGE_WIDTH_RATES, [50, 100, 100]);
        assert_eq!(set.scratch.pg, JudgeProperty::SEVENKEYS.scratch.pg, "SEVENKEYS scratch PG stays (-30000, 30000)");
        assert_eq!(set.scratch.judge(25_000), Some(Judge::PerfectGreat), "25 ms into a scratch is still a PERFECT GREAT");
    }

    #[test]
    fn the_keyboard_row_is_the_compiled_in_fallback_for_the_24_key_mode() {
        assert_eq!(
            flat(JudgeProperty::defaults_for_mode(&Mode::KEYBOARD_24K).note),
            flat(JudgeProperty::KEYBOARD.note),
            "a data file with no KEYBOARD_24K row must not silently judge 24K on the 7K table"
        );
        assert_eq!(flat(JudgeProperty::defaults_for_mode(&Mode::KEYBOARD_24K).ln_end), flat(JudgeProperty::KEYBOARD.ln_end));
    }

    #[test]
    fn window_set_for_mode_applies_the_modes_own_rule() {
        let popn = JudgeWindowSet::for_mode(&Mode::POPN_9K, 33, [100, 100, 100], [100, 100, 100]);
        assert_eq!(popn.note.pg, (-20_000, 20_000), "PMS fixes PG whatever the judgerank");
        assert_eq!(popn.note.gr, (-20_000, 20_000), "PMS lifts GREAT to the PG floor at judgerank 33");

        let seven = JudgeWindowSet::for_mode(&Mode::BEAT_7K, 33, [100, 100, 100], [100, 100, 100]);
        assert_eq!(seven.note.pg, (-6_600, 6_600), "the NORMAL rule scales PG with everything else");
        assert_eq!(seven.note.gr, (-19_800, 19_800));
    }

    #[test]
    fn window_rule_round_trips_through_ron() {
        for rule in JudgeWindowRule::ALL {
            let text = ron::ser::to_string(&rule).unwrap();
            assert_eq!(ron::from_str::<JudgeWindowRule>(&text).unwrap(), rule, "{rule:?}");
        }
    }
}
