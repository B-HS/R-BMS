use rbms_model::{LnKind, Mode, Model, NoteKind};

use crate::Judge;
use crate::gauge::{ClearType, Gauge, GaugeKind, clear_lamp};
use crate::windows::{JudgeProperty, JudgeWindows, judgerank_for};

struct JNote {
    head_us: i64,
    end_us: Option<i64>,
    ln: Option<LnKind>,
    judged: bool,
    holding: bool,
    head_judge: Option<Judge>,
    /// Signed head timing delta (µs) recorded when the head was hit, beatoraja
    /// `LaneState.lnstartDuration`. `None` until the head is judged.
    head_delta_us: Option<i64>,
}

/// CN/HCN ("charge"/"hell-charge") long notes are judged twice — the head at press and the release
/// end at key-up — each a counted judgment (beatoraja `JudgeManager` calls `updateMicro` at both),
/// whereas a plain LN is a single judgment (the worse of head/end). This predicate gates the
/// charge-note behaviour so the verified LN/Normal paths are byte-identical to before. See
/// `docs/reference/cn-hcn-judgment.md`.
fn is_charge(ln: Option<LnKind>) -> bool {
    matches!(ln, Some(LnKind::Cn) | Some(LnKind::Hcn))
}

struct Lane {
    notes: Vec<JNote>,
    cursor: usize,
}

/// A mine note: passing it with the lane key held costs `damage` gauge points
/// (beatoraja `JudgeManager.java:244-247`).
struct Mine {
    time_us: i64,
    damage: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct JudgeResult {
    pub judge: Judge,
    pub lane: usize,
    pub note_index: usize,
    pub fast: bool,
    pub delta_us: i64,
}

fn worse(a: Judge, b: Judge) -> Judge {
    if a as usize >= b as usize { a } else { b }
}

/// Widest candidate window across a key and a scratch table, seeded at 0 like beatoraja's
/// `mjudgestart`/`mjudgeend` (`JudgeManager.java:189-197`).
fn gate_of(note: &JudgeWindows, scratch: &JudgeWindows) -> (i64, i64) {
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

/// Stateful note-matching engine with LN support: per-lane time-sorted notes, a moving
/// cursor, combo, EX score, per-judge counts and a groove gauge. `press` matches the
/// nearest unjudged note (LN heads start a hold); `release` finalises a held LN against
/// the LN-end window; `update` sweeps unhittable notes into 見逃し POOR and applies mine damage.
///
/// Judge slots follow beatoraja exactly: index 4 ([`Judge::Poor`]) is 見逃し POOR — a note that
/// went by unhit — and index 5 ([`Judge::Miss`]) is 空POOR, a press that reached only the MS band
/// and consumed nothing.
pub struct JudgeEngine {
    lanes: Vec<Lane>,
    mines: Vec<Vec<Mine>>,
    mine_cursor: Vec<usize>,
    held: Vec<bool>,
    scratch_lane: Vec<bool>,
    prop: JudgeProperty,
    windows: JudgeWindows,
    scratch_windows: JudgeWindows,
    /// LN/CN release window for key lanes. Derived from the chart's mode + `#RANK` (and scaled by
    /// JUDGE WIDTH) alongside `windows`, so long-note releases respect rank/width just like note
    /// heads. Defaults to `SEVENKEY_LN_END` for the mode-less constructors.
    ln_end: JudgeWindows,
    ln_scratch_end: JudgeWindows,
    gate: (i64, i64),
    longnote_margin_rate: i32,
    pub combo: u32,
    pub max_combo: u32,
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub gauge: Gauge,
    pub last_judge: Option<Judge>,
    pub last_fast: bool,
    pub fast: u32,
    pub slow: u32,
    /// Per-judge early/late split (index = judge: 0=PG..4=PR, 5=MS), so `early[i] + late[i] ==
    /// counts[i]`. EARLY = pressed at or before the note (`dm >= 0`, beatoraja
    /// `JudgeManager.java:652` passes `mfast >= 0`); LATE = after.
    pub early: [u32; 6],
    pub late: [u32; 6],
    /// Empty-poor (空POOR) tally, kept as a mirror of `counts[5]` for callers that read it by name.
    pub empty_poor: u32,
    /// Running sum and count of signed hit deltas (µs) for actual hits (press/release, not sweeps),
    /// so `avg_judge_us()` yields beatoraja `IRScoreData.avgjudge` — the mean timing error.
    sum_delta_us: i64,
    timing_count: u32,
    total_notes: u32,
}

impl JudgeEngine {
    pub fn new(per_lane_times: Vec<Vec<i64>>, windows: JudgeWindows) -> Self {
        let pairs = per_lane_times.into_iter().map(|ts| ts.into_iter().map(|t| (t, None)).collect()).collect();
        Self::from_pairs(pairs, windows)
    }

    pub fn from_pairs(per_lane: Vec<Vec<(i64, Option<i64>)>>, windows: JudgeWindows) -> Self {
        // Pair-built notes (tests, `new`) treat any long note as a plain LN; `from_model` carries the
        // real `LnKind` through `from_triples`.
        let triples: Vec<Vec<(i64, Option<i64>, Option<LnKind>)>> =
            per_lane.into_iter().map(|lane| lane.into_iter().map(|(h, e)| (h, e, e.map(|_| LnKind::Ln))).collect()).collect();
        Self::from_triples(triples, windows)
    }

    fn from_triples(per_lane: Vec<Vec<(i64, Option<i64>, Option<LnKind>)>>, windows: JudgeWindows) -> Self {
        let lanes: Vec<Lane> = per_lane
            .into_iter()
            .map(|mut notes| {
                notes.sort_by_key(|n| n.0);
                Lane {
                    notes: notes
                        .into_iter()
                        .map(|(h, e, ln)| JNote { head_us: h, end_us: e, ln, judged: false, holding: false, head_judge: None, head_delta_us: None })
                        .collect(),
                    cursor: 0,
                }
            })
            .collect();
        // A CN/HCN long note is judged twice (head + release end) so it counts as two toward the
        // total; a plain LN or normal note counts once. Keeps the EX/gauge denominators in step with
        // the per-note judgments and with `rbms_chart::count_playable_notes`.
        let total_notes = lanes
            .iter()
            .flat_map(|l| l.notes.iter())
            .map(|n| if is_charge(n.ln) && n.end_us.is_some() { 2 } else { 1 })
            .sum();
        let gauge = Gauge::new(GaugeKind::Normal, 200.0, total_notes as usize);
        let n = lanes.len();
        let prop = JudgeProperty::SEVENKEYS;
        JudgeEngine {
            lanes,
            mines: (0..n).map(|_| Vec::new()).collect(),
            mine_cursor: vec![0; n],
            held: vec![false; n],
            scratch_lane: vec![false; n],
            prop,
            windows,
            scratch_windows: prop.scratch,
            ln_end: JudgeWindows::SEVENKEY_LN_END,
            ln_scratch_end: prop.ln_scratch_end,
            gate: gate_of(&windows, &prop.scratch),
            longnote_margin_rate: DEFAULT_LONGNOTE_MARGIN_RATE,
            combo: 0,
            max_combo: 0,
            counts: [0; 6],
            ex_score: 0,
            gauge,
            last_judge: None,
            last_fast: false,
            fast: 0,
            slow: 0,
            early: [0; 6],
            late: [0; 6],
            empty_poor: 0,
            sum_delta_us: 0,
            timing_count: 0,
            total_notes,
        }
    }

    /// Build an engine from `model`, deriving all four timing tables (note/scratch/LN end/LN
    /// scratch end) from the chart's [`Mode`] at its `#RANK`/`#DEFEXRANK` judgerank. Prefer this
    /// over [`from_model`](Self::from_model), whose `windows` argument replaces only the key-lane
    /// note table and can therefore disagree with the other three.
    pub fn from_model_for_mode(model: &Model) -> Self {
        let windows = JudgeWindows::note_for_mode(&model.mode).scaled(judgerank_for(model.meta.rank, model.meta.defexrank));
        Self::from_model(model, windows)
    }

    /// Build an engine from `model`, overriding the key-lane note table with `windows`.
    ///
    /// The scratch, LN-end and LN-scratch-end tables are always derived from the chart's mode and
    /// judgerank, so `windows` must be the matching mode table (optionally re-scaled) or the key
    /// lanes will judge on a different curve from everything else. [`from_model_for_mode`]
    /// (Self::from_model_for_mode) derives it for you.
    pub fn from_model(model: &Model, windows: JudgeWindows) -> Self {
        let n = model.mode.key;
        let mut per_lane: Vec<Vec<(i64, Option<i64>, Option<LnKind>)>> = vec![Vec::new(); n];
        let mut mines: Vec<Vec<Mine>> = (0..n).map(|_| Vec::new()).collect();
        for lane in 0..n {
            let mut pending_start: Option<(i64, LnKind)> = None;
            for tl in &model.timelines {
                let Some(note) = &tl.notes[lane] else { continue };
                match note.kind {
                    NoteKind::Mine { damage } => mines[lane].push(Mine { time_us: note.time_us, damage }),
                    NoteKind::Normal => per_lane[lane].push((note.time_us, None, None)),
                    NoteKind::LongStart { ln } => pending_start = Some((note.time_us, ln)),
                    NoteKind::LongEnd { .. } => {
                        if let Some((s, ln)) = pending_start.take() {
                            per_lane[lane].push((s, Some(note.time_us), Some(ln)));
                        }
                    }
                }
            }
            mines[lane].sort_by_key(|m| m.time_us);
        }
        let mut engine = Self::from_triples(per_lane, windows);
        engine.mines = mines;
        engine.mine_cursor = vec![0; n];
        engine.apply_mode(&model.mode, judgerank_for(model.meta.rank, model.meta.defexrank));
        engine.windows = windows;
        engine.refresh_gate();
        engine.set_gauge(GaugeKind::Normal, model.meta.total);
        engine
    }

    /// Adopt `mode`'s [`JudgeProperty`] row and derive all four timing tables at `judgerank`.
    pub fn apply_mode(&mut self, mode: &Mode, judgerank: i32) {
        let prop = JudgeProperty::for_mode(mode);
        self.prop = prop;
        self.windows = prop.note.scaled(judgerank);
        self.scratch_windows = prop.scratch.scaled(judgerank);
        self.ln_end = prop.ln_end.scaled(judgerank);
        self.ln_scratch_end = prop.ln_scratch_end.scaled(judgerank);
        self.scratch_lane = (0..self.lanes.len()).map(|l| mode.is_scratch(l)).collect();
        self.refresh_gate();
    }

    fn refresh_gate(&mut self) {
        self.gate = gate_of(&self.windows, &self.scratch_windows);
    }

    fn is_scratch(&self, lane: usize) -> bool {
        self.scratch_lane.get(lane).copied().unwrap_or(false)
    }

    fn note_window(&self, lane: usize) -> JudgeWindows {
        if self.is_scratch(lane) { self.scratch_windows } else { self.windows }
    }

    fn ln_window(&self, lane: usize) -> JudgeWindows {
        if self.is_scratch(lane) { self.ln_scratch_end } else { self.ln_end }
    }

    /// Release grace for a held long note (`JudgeProperty.longnoteMargin` scaled by the user's
    /// long-note margin rate; the scratch table's margin is not user-scalable in beatoraja,
    /// `JudgeManager.java:186-188`).
    fn ln_margin(&self, lane: usize) -> i64 {
        if self.is_scratch(lane) {
            self.prop.longscratch_margin
        } else {
            self.prop.longnote_margin * self.longnote_margin_rate.max(0) as i64 / 100
        }
    }

    pub fn set_gauge(&mut self, kind: GaugeKind, total: f64) {
        self.gauge = Gauge::new(kind, total, self.total_notes as usize);
    }

    /// Replace the key-lane note timing window (e.g. after applying a user judge-width multiplier).
    /// Call before play begins.
    pub fn set_windows(&mut self, windows: JudgeWindows) {
        self.windows = windows;
        self.refresh_gate();
    }

    /// Replace the scratch-lane note timing window.
    pub fn set_scratch_windows(&mut self, windows: JudgeWindows) {
        self.scratch_windows = windows;
        self.refresh_gate();
    }

    /// Replace the key-lane LN/CN release window (kept in step with [`set_windows`](Self::set_windows)
    /// when the user changes JUDGE WIDTH, so note and LN leniency scale together).
    pub fn set_ln_end(&mut self, ln_end: JudgeWindows) {
        self.ln_end = ln_end;
    }

    /// Replace the scratch-lane long-note release window.
    pub fn set_ln_scratch_end(&mut self, ln_end: JudgeWindows) {
        self.ln_scratch_end = ln_end;
    }

    /// User LONGNOTE MARGIN rate in percent (100 = the mode's stock margin), beatoraja
    /// `PlayerConfig.longnoteMarginRate`.
    pub fn set_longnote_margin_rate(&mut self, rate_percent: i32) {
        self.longnote_margin_rate = rate_percent;
    }

    pub fn total_notes(&self) -> u32 {
        self.total_notes
    }

    /// Notes resolved so far. 空POOR (`counts[5]`) consumes no note, so it is excluded — this stays
    /// the "how far through the chart are we" counter.
    pub fn total_judged(&self) -> u32 {
        self.counts[..5].iter().sum()
    }

    pub fn clear_lamp(&self) -> ClearType {
        clear_lamp(&self.gauge, &self.counts, self.max_combo, self.total_notes)
    }

    pub fn press(&mut self, lane: usize, press_us: i64) -> Option<JudgeResult> {
        if let Some(h) = self.held.get_mut(lane) {
            *h = true;
        }
        let w = self.note_window(lane);
        let l = self.lanes.get(lane)?;

        let (gate_late, gate_early) = self.gate;

        let mut best: Option<usize> = None;
        let mut best_abs = i64::MAX;
        let mut rehit: Option<(usize, i64)> = None;
        let mut rehit_abs = i64::MAX;
        let floor_us = press_us + gate_late - REMARK_SLACK_US;
        let mut i = l.cursor;
        while i > 0 && l.notes[i - 1].head_us >= floor_us {
            i -= 1;
        }
        while i < l.notes.len() {
            let n = &l.notes[i];
            let dm = n.head_us - press_us;
            if dm > gate_early {
                break;
            }
            if dm >= gate_late {
                let a = dm.abs();
                if !n.judged && !n.holding {
                    if a < best_abs {
                        best_abs = a;
                        best = Some(i);
                    }
                } else if n.judged && a < rehit_abs && w.in_ms_band(dm) {
                    rehit_abs = a;
                    rehit = Some((i, dm));
                }
            }
            i += 1;
        }

        let idx = match best {
            Some(i) => i,
            None => {
                let (idx, dm) = rehit?;
                return Some(self.empty_poor_hit(lane, idx, dm));
            }
        };
        let dm = self.lanes[lane].notes[idx].head_us - press_us;
        let judge = w.judge(dm)?;
        if judge == Judge::Miss {
            return Some(self.empty_poor_hit(lane, idx, dm));
        }
        let is_cn = is_charge(self.lanes[lane].notes[idx].ln);
        if self.lanes[lane].notes[idx].end_us.is_some() {
            {
                let note = &mut self.lanes[lane].notes[idx];
                note.holding = true;
                note.head_judge = Some(judge);
                note.head_delta_us = Some(dm);
            }
            if is_cn {
                // CN/HCN: the head is a counted judgment committed at press; the release end is judged
                // separately at key-up (beatoraja's two-`updateMicro` model). Plain LN stays a single
                // judgment resolved at release.
                self.apply(judge);
                self.record_timing(judge, dm);
            }
            return Some(JudgeResult { judge, lane, note_index: idx, fast: dm >= 0, delta_us: dm });
        }
        let note = &mut self.lanes[lane].notes[idx];
        note.judged = true;
        self.apply(judge);
        self.record_timing(judge, dm);
        Some(JudgeResult { judge, lane, note_index: idx, fast: dm >= 0, delta_us: dm })
    }

    fn empty_poor_hit(&mut self, lane: usize, idx: usize, dm: i64) -> JudgeResult {
        self.apply(Judge::Miss);
        self.record_direction(Judge::Miss, dm);
        self.empty_poor = self.counts[5];
        JudgeResult { judge: Judge::Miss, lane, note_index: idx, fast: dm >= 0, delta_us: dm }
    }

    pub fn release(&mut self, lane: usize, release_us: i64) -> Option<JudgeResult> {
        if let Some(h) = self.held.get_mut(lane) {
            *h = false;
        }
        let w = self.ln_window(lane);
        let l = self.lanes.get(lane)?;
        let idx = l.notes.iter().position(|n| n.holding)?;
        let note = &self.lanes[lane].notes[idx];
        let end = note.end_us?;
        let head_judge = note.head_judge.unwrap_or(Judge::Poor);
        let dm = end - release_us;
        let end_judge = w.judge(dm).unwrap_or(Judge::Poor);
        // CN/HCN: the release end is its own counted judgment (the head was already counted at press),
        // so it is not capped by the head. Plain LN resolves to the worse of head/end (single count).
        let final_judge = if is_charge(note.ln) { end_judge } else { worse(head_judge, end_judge) };
        let note = &mut self.lanes[lane].notes[idx];
        note.judged = true;
        note.holding = false;
        self.apply(final_judge);
        self.record_timing(final_judge, dm);
        Some(JudgeResult { judge: final_judge, lane, note_index: idx, fast: dm >= 0, delta_us: dm })
    }

    /// Sweep notes that can no longer be hit (now past their latest BAD-late bound) into
    /// 見逃し POOR, finalise held long notes whose end has gone by, and charge mine damage to
    /// lanes whose key is down.
    pub fn update(&mut self, now_us: i64) {
        for lane in 0..self.mines.len() {
            while self.mine_cursor[lane] < self.mines[lane].len() && self.mines[lane][self.mine_cursor[lane]].time_us <= now_us {
                let damage = self.mines[lane][self.mine_cursor[lane]].damage;
                if self.held.get(lane).copied().unwrap_or(false) {
                    self.gauge.add_value(-damage as f32);
                }
                self.mine_cursor[lane] += 1;
            }
        }
        let mut events: Vec<(Judge, i64)> = Vec::new();
        for lane in 0..self.lanes.len() {
            let miss_bound = self.note_window(lane).bd.0;
            let margin = self.ln_margin(lane);
            let l = &mut self.lanes[lane];
            while l.cursor < l.notes.len() {
                let n = &mut l.notes[l.cursor];
                if n.judged {
                    l.cursor += 1;
                    continue;
                }
                if n.holding {
                    let Some(end) = n.end_us else {
                        l.cursor += 1;
                        continue;
                    };
                    if is_charge(n.ln) {
                        if end - now_us < miss_bound {
                            n.judged = true;
                            n.holding = false;
                            events.push((Judge::Poor, end - now_us));
                            l.cursor += 1;
                        } else {
                            break;
                        }
                    } else if now_us > end + margin {
                        let head = n.head_judge.unwrap_or(Judge::Poor);
                        let head_dm = n.head_delta_us.unwrap_or(end - now_us);
                        n.judged = true;
                        n.holding = false;
                        events.push((head, head_dm));
                        l.cursor += 1;
                    } else {
                        break;
                    }
                } else if n.head_us - now_us < miss_bound {
                    let charge_pair = is_charge(n.ln) && n.end_us.is_some();
                    let dm = n.head_us - now_us;
                    n.judged = true;
                    events.push((Judge::Poor, dm));
                    // A never-hit CN/HCN misses both of its judged objects (head + end).
                    if charge_pair {
                        events.push((Judge::Poor, dm));
                    }
                    l.cursor += 1;
                } else {
                    break;
                }
            }
        }
        for (j, dm) in events {
            self.apply(j);
            self.record_direction(j, dm);
        }
    }

    fn apply(&mut self, judge: Judge) {
        let i = judge as usize;
        self.counts[i] += 1;
        match judge {
            Judge::PerfectGreat => self.ex_score += 2,
            Judge::Great => self.ex_score += 1,
            _ => {}
        }
        if self.prop.combo[i] && i < 5 {
            self.combo += 1;
        }
        if !self.prop.combo[i] {
            self.combo = 0;
        }
        self.gauge.update(judge);
        self.last_judge = Some(judge);
        self.max_combo = self.max_combo.max(self.combo);
        self.empty_poor = self.counts[5];
    }

    fn record_timing(&mut self, judge: Judge, dm: i64) {
        self.record_direction(judge, dm);
        if (judge as usize) < 4 {
            self.last_fast = dm >= 0;
            if dm >= 0 {
                self.fast += 1;
            } else {
                self.slow += 1;
            }
            self.sum_delta_us += dm;
            self.timing_count += 1;
        }
    }

    /// Tally a judgment as EARLY (`dm >= 0`, pressed at or before the note) or LATE, keeping
    /// `early[i] + late[i] == counts[i]` (beatoraja `score.addJudgeCount(judge, mfast >= 0, 1)`).
    fn record_direction(&mut self, judge: Judge, dm: i64) {
        let i = judge as usize;
        if dm >= 0 {
            self.early[i] += 1;
        } else {
            self.late[i] += 1;
        }
    }

    /// Mean signed hit timing (µs) over actual hits — beatoraja `IRScoreData.avgjudge`. Positive =
    /// early on average. Zero when nothing was timed.
    pub fn avg_judge_us(&self) -> i64 {
        if self.timing_count == 0 { 0 } else { self.sum_delta_us / self.timing_count as i64 }
    }
}

/// Extra slack beatoraja subtracts when re-marking a lane's scan start each frame
/// (`JudgeManager.java:220`: `prevmtime + mjudgestart - 100000`).
const REMARK_SLACK_US: i64 = 100_000;

/// Stock long-note margin rate (percent) — beatoraja `PlayerConfig.longnoteMarginRate` default.
const DEFAULT_LONGNOTE_MARGIN_RATE: i32 = 100;
