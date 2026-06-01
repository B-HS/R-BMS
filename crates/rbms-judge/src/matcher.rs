use rbms_model::{Model, NoteKind};

use crate::Judge;
use crate::gauge::{ClearType, Gauge, GaugeKind, clear_lamp};
use crate::windows::{JudgeWindows, rank_to_judgerank};

const LN_MARGIN: i64 = 200_000;

struct JNote {
    head_us: i64,
    end_us: Option<i64>,
    judged: bool,
    holding: bool,
    head_judge: Option<Judge>,
}

struct Lane {
    notes: Vec<JNote>,
    cursor: usize,
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

/// Stateful note-matching engine with LN support: per-lane time-sorted notes, a moving
/// cursor, combo, EX score, per-judge counts and a groove gauge. `press` matches the
/// nearest unjudged note (LN heads start a hold); `release` finalises a held LN against
/// the LN-end window; `update` sweeps unhittable notes into MISS.
pub struct JudgeEngine {
    lanes: Vec<Lane>,
    windows: JudgeWindows,
    /// LN/CN release window. Derived from the chart's mode + `#RANK` (and scaled by JUDGE WIDTH)
    /// alongside `windows`, so long-note releases respect rank/width just like note heads — not a
    /// fixed constant. Defaults to `SEVENKEY_LN_END` for the mode-less constructors.
    ln_end: JudgeWindows,
    pub combo: u32,
    pub max_combo: u32,
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub gauge: Gauge,
    pub last_judge: Option<Judge>,
    pub last_fast: bool,
    pub fast: u32,
    pub slow: u32,
    /// Per-judge early/late split (index = judge: 0=PG..5=MS), so `early[i] + late[i] == counts[i]`.
    /// EARLY = pressed before the note (`dm > 0`); LATE = on/after (`dm <= 0`) or swept (note passed).
    /// Feeds the IR `JudgeBreakdown` (beatoraja `IRScoreData` epg/lpg…ems/lms 12-field split).
    pub early: [u32; 6],
    pub late: [u32; 6],
    /// Empty-poor (空POOR) tally: presses that landed only in the wide MS window, beyond the
    /// BAD window (far early). Kept apart from `counts` because such a press neither consumes a
    /// note nor breaks combo (beatoraja `SEVENKEYS`), so it must not inflate the per-note tally.
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
        let lanes: Vec<Lane> = per_lane
            .into_iter()
            .map(|mut notes| {
                notes.sort_by_key(|n| n.0);
                Lane {
                    notes: notes.into_iter().map(|(h, e)| JNote { head_us: h, end_us: e, judged: false, holding: false, head_judge: None }).collect(),
                    cursor: 0,
                }
            })
            .collect();
        let total_notes = lanes.iter().map(|l| l.notes.len() as u32).sum();
        let gauge = Gauge::new(GaugeKind::Normal, 200.0, total_notes as usize);
        JudgeEngine {
            lanes,
            windows,
            ln_end: JudgeWindows::SEVENKEY_LN_END,
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

    pub fn from_model(model: &Model, windows: JudgeWindows) -> Self {
        let n = model.mode.key;
        let mut per_lane: Vec<Vec<(i64, Option<i64>)>> = vec![Vec::new(); n];
        for lane in 0..n {
            let mut pending_start: Option<i64> = None;
            for tl in &model.timelines {
                let Some(note) = &tl.notes[lane] else { continue };
                match note.kind {
                    NoteKind::Mine { .. } => {}
                    NoteKind::Normal => per_lane[lane].push((note.time_us, None)),
                    NoteKind::LongStart { .. } => pending_start = Some(note.time_us),
                    NoteKind::LongEnd { .. } => {
                        if let Some(s) = pending_start.take() {
                            per_lane[lane].push((s, Some(note.time_us)));
                        }
                    }
                }
            }
        }
        let mut engine = Self::from_pairs(per_lane, windows);
        // LN release window follows the same mode + #RANK policy as the note window (the caller
        // passes the already-scaled note window in); previously this was a hardcoded 100% constant,
        // so long-note releases ignored both #RANK and JUDGE WIDTH.
        engine.ln_end = JudgeWindows::ln_end_for_mode(&model.mode).scaled(rank_to_judgerank(model.meta.rank));
        engine.set_gauge(GaugeKind::Normal, model.meta.total);
        engine
    }

    pub fn set_gauge(&mut self, kind: GaugeKind, total: f64) {
        self.gauge = Gauge::new(kind, total, self.total_notes as usize);
    }

    /// Replace the note timing windows (e.g. after applying a user judge-width multiplier).
    /// Call before play begins.
    pub fn set_windows(&mut self, windows: JudgeWindows) {
        self.windows = windows;
    }

    /// Replace the LN/CN release window (kept in step with [`set_windows`](Self::set_windows) when
    /// the user changes JUDGE WIDTH, so note and LN leniency scale together).
    pub fn set_ln_end(&mut self, ln_end: JudgeWindows) {
        self.ln_end = ln_end;
    }

    pub fn total_notes(&self) -> u32 {
        self.total_notes
    }

    pub fn total_judged(&self) -> u32 {
        self.counts.iter().sum()
    }

    pub fn clear_lamp(&self) -> ClearType {
        clear_lamp(&self.gauge, &self.counts, self.max_combo, self.total_notes)
    }

    pub fn press(&mut self, lane: usize, press_us: i64) -> Option<JudgeResult> {
        let w = self.windows;
        let l = self.lanes.get(lane)?;

        // Candidate gate derived from the active (scaled) windows, not fixed constants, so it
        // stays consistent with judge()/the miss sweep at any judge width: widest EARLY is the
        // POOR upper bound, widest LATE is the BAD lower bound (== update()'s miss_bound).
        let gate_early = w.ms.1;
        let gate_late = w.bd.0;

        let mut best: Option<usize> = None;
        let mut best_abs = i64::MAX;
        let mut i = l.cursor;
        while i < l.notes.len() {
            let n = &l.notes[i];
            let dm = n.head_us - press_us;
            if dm > gate_early {
                break;
            }
            if !n.judged && !n.holding && dm >= gate_late {
                let a = dm.abs();
                if a < best_abs {
                    best_abs = a;
                    best = Some(i);
                }
            }
            i += 1;
        }

        let idx = best?;
        let dm = self.lanes[lane].notes[idx].head_us - press_us;
        let judge = w.judge(dm)?;
        // Empty poor (空POOR): the press landed only in the wide MS window, beyond the BAD window
        // (far early — for 7K NOTE that is +220..+500ms). beatoraja `JudgeProperty.SEVENKEYS` sets
        // `judgeVanish[5]=false` and `combo[5]=true`, so this neither consumes the note nor breaks
        // combo — it costs only an MS gauge penalty and a POOR flash, leaving the note hittable
        // when it actually arrives. Consuming it here was eating notes and breaking combo on
        // legitimate (slightly-early / jack) presses.
        if judge == Judge::Poor {
            self.empty_poor += 1;
            self.gauge.update(Judge::Miss);
            self.last_judge = Some(Judge::Poor);
            return Some(JudgeResult { judge: Judge::Poor, lane, note_index: idx, fast: dm > 0, delta_us: dm });
        }
        let note = &mut self.lanes[lane].notes[idx];
        if note.end_us.is_some() {
            note.holding = true;
            note.head_judge = Some(judge);
            return Some(JudgeResult { judge, lane, note_index: idx, fast: dm > 0, delta_us: dm });
        }
        note.judged = true;
        self.apply(judge);
        self.record_timing(judge, dm);
        Some(JudgeResult { judge, lane, note_index: idx, fast: dm > 0, delta_us: dm })
    }

    pub fn release(&mut self, lane: usize, release_us: i64) -> Option<JudgeResult> {
        let l = self.lanes.get(lane)?;
        let idx = l.notes.iter().position(|n| n.holding)?;
        let note = &self.lanes[lane].notes[idx];
        let end = note.end_us.unwrap();
        let head_judge = note.head_judge.unwrap_or(Judge::Poor);
        let dm = end - release_us;
        let end_judge = self.ln_end.judge(dm).unwrap_or(Judge::Poor);
        let final_judge = worse(head_judge, end_judge);
        let note = &mut self.lanes[lane].notes[idx];
        note.judged = true;
        note.holding = false;
        self.apply(final_judge);
        self.record_timing(final_judge, dm);
        Some(JudgeResult { judge: final_judge, lane, note_index: idx, fast: dm > 0, delta_us: dm })
    }

    /// Sweep notes that can no longer be hit (now past their latest BAD-late bound) into
    /// MISS, and finalise held LNs not released within the margin.
    pub fn update(&mut self, now_us: i64) {
        let miss_bound = self.windows.bd.0;
        let ln_end = self.ln_end;
        let mut events: Vec<Judge> = Vec::new();
        for l in &mut self.lanes {
            while l.cursor < l.notes.len() {
                let n = &mut l.notes[l.cursor];
                if n.judged {
                    l.cursor += 1;
                    continue;
                }
                if n.holding {
                    let end = n.end_us.unwrap();
                    if now_us > end + LN_MARGIN {
                        let end_judge = ln_end.judge(end - now_us).unwrap_or(Judge::Poor);
                        let final_judge = worse(n.head_judge.unwrap_or(Judge::Poor), end_judge);
                        n.judged = true;
                        n.holding = false;
                        events.push(final_judge);
                        l.cursor += 1;
                    } else {
                        break;
                    }
                } else if n.head_us - now_us < miss_bound {
                    n.judged = true;
                    events.push(Judge::Miss);
                    l.cursor += 1;
                } else {
                    break;
                }
            }
        }
        // Every sweep event is LATE by construction: a swept normal note and an over-held LN both
        // resolve only after their time/end has passed, so they count toward `late[judge]`.
        for j in events {
            self.apply(j);
            self.late[j as usize] += 1;
        }
    }

    fn apply(&mut self, judge: Judge) {
        match judge {
            Judge::PerfectGreat => {
                self.counts[0] += 1;
                self.ex_score += 2;
                self.combo += 1;
            }
            Judge::Great => {
                self.counts[1] += 1;
                self.ex_score += 1;
                self.combo += 1;
            }
            Judge::Good => {
                self.counts[2] += 1;
                self.combo += 1;
            }
            Judge::Bad => {
                self.counts[3] += 1;
                self.combo = 0;
            }
            Judge::Poor => {
                self.counts[4] += 1;
                self.combo = 0;
            }
            Judge::Miss => {
                self.counts[5] += 1;
                self.combo = 0;
            }
        }
        self.gauge.update(judge);
        self.last_judge = Some(judge);
        self.max_combo = self.max_combo.max(self.combo);
    }

    fn record_timing(&mut self, judge: Judge, dm: i64) {
        self.record_direction(judge, dm);
        if judge != Judge::Miss {
            self.last_fast = dm > 0;
            if dm > 0 {
                self.fast += 1;
            } else if dm < 0 {
                self.slow += 1;
            }
            self.sum_delta_us += dm;
            self.timing_count += 1;
        }
    }

    /// Tally a judgment as EARLY (`dm > 0`, pressed before the note) or LATE (`dm <= 0`), keeping
    /// `early[i] + late[i] == counts[i]`.
    fn record_direction(&mut self, judge: Judge, dm: i64) {
        let i = judge as usize;
        if dm > 0 {
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
