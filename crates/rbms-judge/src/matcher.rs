use rbms_model::{LnKind, Mode, Model, NoteKind};
use serde::{Deserialize, Serialize};

use crate::Judge;
use crate::algorithm::{JudgeAlgorithm, NoteRef, NoteType, UNJUDGED_STATE};
use crate::gauge::{ClearType, GaugeKind, GrooveGauge, clear_lamp};
use crate::gauge_tables::GaugeSetId;
use crate::ln::{HCN_GAUGE_TICK_RATE, HCN_GAUGE_TICK_US, LaneHold, LnMode};
use crate::windows::{
    EMPTY_POOR_JUDGE_INDEX, JudgeProperty, JudgeWindowRule, JudgeWindowSet, JudgeWindows, MissCondition, UNMODIFIED_JUDGE_WIDTH_RATES, candidate_gate_across,
};

/// Per-lane note seeds as `(head_us, end_us, long-note kind)`, the shape the constructors build
/// lanes from.
type LaneSeeds = Vec<Vec<(i64, Option<i64>, Option<LnKind>)>>;

/// Any non-zero `Note.getState()`. The reference only ever tests state against zero
/// (`JudgeManager.java:396, 398, 401`), so one non-zero value stands for every judged note.
const JUDGED_STATE: u8 = UNJUDGED_STATE + 1;

/// The judgment each reference judge code names (`JudgeManager.java:400-404`): 0..3 are the timing
/// bands, 4 is 見逃し POOR and 5 is 空POOR. A code past the end of this table names no judgment and
/// cancels the press (`JudgeManager.java:412`).
const JUDGE_BY_CODE: [Judge; 6] = [Judge::PerfectGreat, Judge::Great, Judge::Good, Judge::Bad, Judge::Poor, Judge::Miss];

/// Reference judge code for a press that reached only the 空POOR band (`JudgeManager.java:401, 404`).
const EMPTY_POOR_CODE: usize = Judge::Miss as usize;

/// Reference judge code for a candidate too far out to judge at all (`JudgeManager.java:401, 412`).
const IGNORED_CODE: usize = JUDGE_BY_CODE.len();

/// Judge code at or above which a long-note release that came early is deferred instead of
/// confirmed (`JudgeManager.java:509, 540`).
const DEFERRED_RELEASE_CODE: usize = Judge::Bad as usize;

/// Highest judge code that still counts as "the end was taken cleanly", the reference's
/// `getState() > 0 && getState() <= 3` test on a hell-charge end (`JudgeManager.java:290`).
const HCN_HELD_END_CODE: usize = Judge::Good as usize;

/// Divisor the reference's `Note.getPlayTime()` applies to the microsecond value
/// `setMicroPlayTime` stored, so a hit inside one millisecond reads back as "never played"
/// (`JudgeManager.java:398, 648, 651`).
const MICROS_PER_MILLI: i64 = 1_000;

fn judge_of_code(code: usize) -> Option<Judge> {
    JUDGE_BY_CODE.get(code).copied()
}

struct JNote {
    head_us: i64,
    end_us: Option<i64>,
    /// The flavour the chart stated for this long note, kept untouched so that LN MODE resolves
    /// from the chart every time it is set rather than from an already-resolved value.
    chart_ln: Option<LnKind>,
    ln: Option<LnKind>,
    judged: bool,
    holding: bool,
    /// `Note.playtime` of the head object: the signed delta of the last judgment it took.
    head_play_time_us: i64,
    /// `Note.playtime` of the paired end object, which only a charge note ever judges separately.
    end_play_time_us: i64,
    /// Judgment the paired end object took, the reference's `getPair().getState() - 1`. Read by the
    /// hell-charge tick to decide whether a released note still gains gauge.
    end_judge: Option<Judge>,
}

/// CN/HCN ("charge"/"hell-charge") long notes are judged twice — the head at press and the release
/// end at key-up — each a counted judgment (the reference implementation's `JudgeManager` calls `updateMicro` at both),
/// whereas a plain LN is a single judgment (the worse of head/end). This predicate gates the
/// charge-note behaviour so the verified LN/Normal paths are byte-identical to before. See
/// `docs/reference/cn-hcn-judgment.md`.
fn is_charge(ln: Option<LnKind>) -> bool {
    matches!(ln, Some(LnKind::Cn) | Some(LnKind::Hcn))
}

/// Whether this note pays out the continuous gauge ticks of a hell-charge note
/// (`JudgeManager.java:235-242`).
fn is_hell_charge(ln: Option<LnKind>) -> bool {
    matches!(ln, Some(LnKind::Hcn))
}

/// How the candidate-selection predicate sees a note.
fn note_ref(note: &JNote) -> NoteRef {
    NoteRef { time_us: note.head_us, state: if note.judged || note.holding { JUDGED_STATE } else { UNJUDGED_STATE }, is_long: note.end_us.is_some() }
}

struct Lane {
    notes: Vec<JNote>,
    cursor: usize,
    /// Scan position of the hell-charge pass detector, kept apart from `cursor` because a passing
    /// note is not yet a resolved one.
    hcn_cursor: usize,
}

/// A mine note: passing it with the lane key held costs `damage` gauge points
/// (reference implementation `JudgeManager.java:244-247`).
struct Mine {
    time_us: i64,
    damage: f64,
}

/// One judgment the frame sweep resolved. Collected while the lane is borrowed and applied once
/// the borrow ends.
struct SweepEvent {
    lane: usize,
    note: usize,
    judge: Judge,
    delta_us: i64,
    on_end: bool,
}

/// Physical direction of a scratch input. The reference remembers which of a scratch lane's two
/// keys grabbed a long note (`JudgeManager.java:435, 449` set `sckey[sc]`) so that the other one
/// ends it — a back-spin scratch. Key lanes have no second direction and always use
/// [`ScratchDir::Forward`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScratchDir {
    #[default]
    Forward,
    Backward,
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

/// Judged objects in `lanes`: a charge note's head and end each count, every other note counts once.
fn count_notes(lanes: &[Lane]) -> u32 {
    lanes.iter().flat_map(|l| l.notes.iter()).map(|n| if is_charge(n.ln) && n.end_us.is_some() { 2 } else { 1 }).sum()
}

/// Stateful note-matching engine with LN support: per-lane time-sorted notes, a moving
/// cursor, combo, EX score, per-judge counts and the nine groove gauges. `press` folds the lane's
/// candidates through the selected [`JudgeAlgorithm`] (LN heads start a hold); `release` finalises
/// or defers a held long note against the LN-end window; `update` confirms deferred releases, pays
/// out hell-charge ticks, sweeps unhittable notes into 見逃し POOR and applies mine damage.
///
/// Judge slots follow the reference implementation exactly: index 4 ([`Judge::Poor`]) is 見逃し POOR — a note that
/// went by unhit — and index 5 ([`Judge::Miss`]) is 空POOR, a press that reached only the MS band
/// and consumed nothing.
pub struct JudgeEngine {
    lanes: Vec<Lane>,
    holds: Vec<LaneHold>,
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
    algorithm: JudgeAlgorithm,
    ln_mode: LnMode,
    gauge_set: GaugeSetId,
    gauge_total: f64,
    /// The reference's `prevmtime` (`JudgeManager.java:100`), the previous frame clock the
    /// hell-charge ticks measure their step against.
    prev_update_us: i64,
    pub combo: u32,
    pub max_combo: u32,
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub gauge: GrooveGauge,
    pub last_judge: Option<Judge>,
    pub last_fast: bool,
    pub fast: u32,
    pub slow: u32,
    /// Per-judge early/late split (index = judge: 0=PG..4=PR, 5=MS), so `early[i] + late[i] ==
    /// counts[i]`. EARLY = pressed at or before the note (`dm >= 0`, reference implementation
    /// `JudgeManager.java:652` passes `mfast >= 0`); LATE = after.
    pub early: [u32; 6],
    pub late: [u32; 6],
    /// Empty-poor (空POOR) tally, kept as a mirror of `counts[5]` for callers that read it by name.
    pub empty_poor: u32,
    /// Running sum and count of signed hit deltas (µs) for actual hits (press/release, not sweeps),
    /// so `avg_judge_us()` yields the reference implementation's `IRScoreData.avgjudge` — the mean timing error.
    sum_delta_us: i64,
    timing_count: u32,
    total_notes: u32,
    /// The reference implementation's `IRScoreData.passnotes`: judgments that consumed their note.
    pass_notes: u32,
}

impl JudgeEngine {
    pub fn new(per_lane_times: Vec<Vec<i64>>, windows: JudgeWindows) -> Self {
        let pairs = per_lane_times.into_iter().map(|ts| ts.into_iter().map(|t| (t, None)).collect()).collect();
        Self::from_pairs(pairs, windows)
    }

    pub fn from_pairs(per_lane: Vec<Vec<(i64, Option<i64>)>>, windows: JudgeWindows) -> Self {
        let triples: LaneSeeds = per_lane.into_iter().map(|lane| lane.into_iter().map(|(h, e)| (h, e, e.map(|_| LnKind::Ln))).collect()).collect();
        Self::from_triples(triples, windows)
    }

    fn from_triples(per_lane: LaneSeeds, windows: JudgeWindows) -> Self {
        let lanes: Vec<Lane> = per_lane
            .into_iter()
            .map(|mut notes| {
                notes.sort_by_key(|n| n.0);
                Lane {
                    notes: notes
                        .into_iter()
                        .map(|(h, e, ln)| JNote {
                            head_us: h,
                            end_us: e,
                            chart_ln: ln,
                            ln,
                            judged: false,
                            holding: false,
                            head_play_time_us: 0,
                            end_play_time_us: 0,
                            end_judge: None,
                        })
                        .collect(),
                    cursor: 0,
                    hcn_cursor: 0,
                }
            })
            .collect();
        let total_notes = count_notes(&lanes);
        let gauge_set = GaugeSetId::default();
        let gauge_total = DEFAULT_GAUGE_TOTAL;
        let gauge = GrooveGauge::new(gauge_set, GaugeKind::Normal, gauge_total, total_notes as usize);
        let n = lanes.len();
        let prop = JudgeProperty::SEVENKEYS;
        JudgeEngine {
            lanes,
            holds: (0..n).map(|_| LaneHold::default()).collect(),
            mines: (0..n).map(|_| Vec::new()).collect(),
            mine_cursor: vec![0; n],
            held: vec![false; n],
            scratch_lane: vec![false; n],
            prop,
            windows,
            scratch_windows: prop.scratch,
            ln_end: JudgeWindows::SEVENKEY_LN_END,
            ln_scratch_end: prop.ln_scratch_end,
            gate: candidate_gate_across(&windows, &prop.scratch),
            longnote_margin_rate: DEFAULT_LONGNOTE_MARGIN_RATE,
            algorithm: JudgeAlgorithm::default(),
            ln_mode: LnMode::default(),
            gauge_set,
            gauge_total,
            prev_update_us: 0,
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
            pass_notes: 0,
        }
    }

    /// Build an engine from `model`, deriving all four timing tables (note/scratch/LN end/LN
    /// scratch end) from the chart's [`Mode`] at its `#RANK`/`#DEFEXRANK` judgerank. Prefer this
    /// over [`from_model`](Self::from_model), whose `windows` argument replaces only the key-lane
    /// note table and can therefore disagree with the other three.
    pub fn from_model_for_mode(model: &Model) -> Self {
        let windows = Self::mode_window_set(model).note;
        Self::from_model(model, windows)
    }

    /// The four stock tables `model` is judged against: its mode's [`JudgeProperty`] row, built
    /// under its mode's [`JudgeWindowRule`] at the judgerank that rule resolves `#RANK`/`#DEFEXRANK`
    /// to, with no user JUDGE WIDTH applied.
    fn mode_window_set(model: &Model) -> JudgeWindowSet {
        let rule = JudgeWindowRule::for_mode(&model.mode);
        let judgerank = rule.judgerank_for(model.meta.rank, model.meta.defexrank);
        JudgeProperty::for_mode(&model.mode).window_set(rule, judgerank, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES)
    }

    /// Build an engine from `model`, overriding the key-lane note table with `windows`.
    ///
    /// The scratch, LN-end and LN-scratch-end tables are always derived from the chart's mode and
    /// judgerank, so `windows` must be the matching mode table (optionally re-scaled) or the key
    /// lanes will judge on a different curve from everything else. [`from_model_for_mode`]
    /// (Self::from_model_for_mode) derives it for you.
    pub fn from_model(model: &Model, windows: JudgeWindows) -> Self {
        let n = model.mode.key;
        let mut per_lane: LaneSeeds = vec![Vec::new(); n];
        let mut mines: Vec<Vec<Mine>> = (0..n).map(|_| Vec::new()).collect();
        for lane in 0..n {
            let mut pending_start: Option<(i64, LnKind)> = None;
            for tl in &model.timelines {
                let Some(note) = &tl.notes[lane] else {
                    continue;
                };
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
        engine.apply_mode(&model.mode, JudgeWindowRule::for_mode(&model.mode).judgerank_for(model.meta.rank, model.meta.defexrank));
        engine.windows = windows;
        engine.refresh_gate();
        engine.set_gauge(GaugeKind::Normal, model.meta.total);
        engine
    }

    /// Adopt `mode`'s [`JudgeProperty`] row, derive all four timing tables at `judgerank` under the
    /// mode's own [`JudgeWindowRule`], and take the gauge table the mode plays on.
    pub fn apply_mode(&mut self, mode: &Mode, judgerank: i32) {
        let prop = JudgeProperty::for_mode(mode);
        self.prop = prop;
        let set = prop.window_set(JudgeWindowRule::for_mode(mode), judgerank, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        self.apply_window_set(&set);
        self.scratch_lane = (0..self.lanes.len()).map(|l| mode.is_scratch(l)).collect();
        self.gauge_set = GaugeSetId::for_mode(mode);
        self.rebuild_gauge();
    }

    /// Judge against `set`: all four tables plus the candidate gate, which is the only place the
    /// scratch JUDGE WIDTH rates reach (`JudgeManager.java:185-198`). Call before play begins.
    pub fn apply_window_set(&mut self, set: &JudgeWindowSet) {
        self.windows = set.note;
        self.scratch_windows = set.scratch;
        self.ln_end = set.ln_end;
        self.ln_scratch_end = set.ln_scratch_end;
        self.gate = set.candidate_gate;
    }

    fn refresh_gate(&mut self) {
        self.gate = candidate_gate_across(&self.windows, &self.scratch_windows);
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
    /// long-note margin rate; the scratch table's margin is not user-scalable in the reference implementation,
    /// `JudgeManager.java:186-188`).
    fn ln_margin(&self, lane: usize) -> i64 {
        if self.is_scratch(lane) { self.prop.longscratch_margin } else { self.prop.longnote_margin * self.longnote_margin_rate.max(0) as i64 / 100 }
    }

    /// Rebuild the nine gauges for the current table, chart total and note count, keeping whichever
    /// slot is selected.
    fn rebuild_gauge(&mut self) {
        let selected = self.gauge.selected_index();
        self.gauge = GrooveGauge::at_index(self.gauge_set, selected, self.gauge_total, self.total_notes as usize);
    }

    pub fn set_gauge(&mut self, kind: GaugeKind, total: f64) {
        self.gauge_total = total;
        self.gauge = GrooveGauge::new(self.gauge_set, kind, total, self.total_notes as usize);
    }

    /// Play on another gauge table — the LR2 set, which no mode selects, is chosen this way
    /// (`GaugeProperty.java:117-125`).
    pub fn set_gauge_set(&mut self, set: GaugeSetId) {
        self.gauge_set = set;
        self.rebuild_gauge();
    }

    /// The gauge table the nine gauges are built from.
    pub fn gauge_set(&self) -> GaugeSetId {
        self.gauge_set
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

    /// Candidate-selection policy for [`press`](Self::press). Defaults to
    /// [`JudgeAlgorithm::Combo`], the reference implementation's own default
    /// (`JudgeAlgorithm.java:42` lists it first); a replay recorded before the algorithm was written
    /// down names [`JudgeAlgorithm::Duration`] and is played back on that.
    pub fn set_algorithm(&mut self, algorithm: JudgeAlgorithm) {
        self.algorithm = algorithm;
    }

    /// The candidate-selection policy currently in force.
    pub fn algorithm(&self) -> JudgeAlgorithm {
        self.algorithm
    }

    /// Resolve every chart note that stated no long-note flavour to `mode`
    /// (`JudgeManager.java:262, 273, 356, 495, 615` consult `lntype` only for `TYPE_UNDEFINED`).
    /// Charge notes are judged twice, so this also re-counts the chart and rebuilds the gauges;
    /// call it before play begins.
    /// Resolving from each note's recorded chart flavour rather than from its current one keeps this
    /// idempotent: setting one mode and then another lands on the second, and setting the same mode
    /// twice changes nothing.
    pub fn set_ln_mode(&mut self, mode: LnMode) {
        self.ln_mode = mode;
        let resolved = mode.resolve();
        for lane in self.lanes.iter_mut() {
            for note in lane.notes.iter_mut() {
                if note.chart_ln == Some(LnKind::Undefined) {
                    note.ln = Some(resolved);
                }
            }
        }
        self.total_notes = count_notes(&self.lanes);
        self.rebuild_gauge();
    }

    /// The long-note flavour chart notes with no stated type play as.
    pub fn ln_mode(&self) -> LnMode {
        self.ln_mode
    }

    /// User LONGNOTE MARGIN rate in percent (100 = the mode's stock margin), reference implementation
    /// `PlayerConfig.longnoteMarginRate`.
    pub fn set_longnote_margin_rate(&mut self, rate_percent: i32) {
        self.longnote_margin_rate = rate_percent;
    }

    pub fn total_notes(&self) -> u32 {
        self.total_notes
    }

    /// Notes resolved so far, the reference implementation's `IRScoreData.passnotes`
    /// (`JudgeManager.java:640-646` counts one per judgment that consumed its note). A judgment the
    /// mode's `judgeVanish` leaves the note alive — 空POOR everywhere, and a pop'n BAD — resolves
    /// nothing and is not counted, so this stays the "how far through the chart are we" counter even
    /// when the same note is judged twice.
    pub fn total_judged(&self) -> u32 {
        self.pass_notes
    }

    pub fn clear_lamp(&self) -> ClearType {
        clear_lamp(self.gauge.selected(), &self.counts, self.max_combo, self.total_notes)
    }

    pub fn press(&mut self, lane: usize, press_us: i64) -> Option<JudgeResult> {
        self.press_dir(lane, ScratchDir::Forward, press_us)
    }

    /// Press a lane from one physical direction. On a scratch lane holding a charge note, spinning
    /// the other way ends it — the back-spin scratch of `JudgeManager.java:358-372` — while the
    /// same way is a re-grab that cancels a deferred release (`:373-375`).
    pub fn press_dir(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        if let Some(h) = self.held.get_mut(lane) {
            *h = true;
        }
        if lane >= self.lanes.len() {
            return None;
        }
        if self.holds[lane].note.is_some() {
            return self.press_while_holding(lane, dir, press_us);
        }
        self.press_new_note(lane, dir, press_us)
    }

    fn press_while_holding(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        let idx = self.holds[lane].note?;
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        let owner = self.holds[lane].owner;
        let ends_the_spin = charge && self.is_scratch(lane) && owner.is_some() && owner != Some(dir);
        if !ends_the_spin {
            self.holds[lane].release_us = None;
            return None;
        }
        let end_us = end_us?;
        let delta_us = end_us - press_us;
        let judge = judge_of_code(self.ln_window(lane).judge_code(delta_us))?;
        Some(self.confirm_hold(lane, idx, judge, delta_us, true))
    }

    /// The reference's two-stage candidate fold (`JudgeManager.java:385-415`). Stage one asks the
    /// algorithm whether to move on — a candidate already chosen but already judged is always
    /// replaced, which is what keeps [`JudgeAlgorithm::Lowest`] from re-judging a consumed note.
    /// Stage two turns the survivor into a judge code, where a 空POOR candidate only wins on a
    /// strictly smaller `|Δt|` and a candidate past every window cancels the press outright.
    ///
    /// `code` deliberately lives outside the loop, as the reference's `judge` does: it is the code
    /// of the last candidate that cleared both filters, not necessarily of `chosen`.
    fn select_candidate(&self, lane: usize, press_us: i64) -> (Option<usize>, usize) {
        let w = self.note_window(lane);
        let note_type = if self.is_scratch(lane) { NoteType::Scratch } else { NoteType::Note };
        let algorithm = self.algorithm;
        let miss_once = self.prop.miss_condition == MissCondition::One;
        let (gate_late, gate_early) = self.gate;
        let l = &self.lanes[lane];

        let mut chosen: Option<usize> = None;
        let mut code = 0usize;
        let floor_us = press_us + gate_late - REMARK_SLACK_US;
        let mut i = l.cursor;
        while i > 0 && l.notes[i - 1].head_us >= floor_us {
            i -= 1;
        }
        while i < l.notes.len() {
            let note = &l.notes[i];
            let delta_us = note.head_us - press_us;
            if delta_us >= gate_early {
                break;
            }
            if delta_us < gate_late {
                i += 1;
                continue;
            }
            let cand = note_ref(note);
            let unjudged = cand.state == UNJUDGED_STATE;
            let adopt = match chosen {
                None => true,
                Some(c) => {
                    let best = note_ref(&l.notes[c]);
                    best.state != UNJUDGED_STATE || algorithm.prefer(&best, &cand, press_us, &w, note_type)
                }
            };
            let blocked = miss_once && (!unjudged || (was_played(note.head_play_time_us) && !w.in_good_band(delta_us)));
            if adopt && !blocked {
                code = if unjudged {
                    let band = w.judge_code(delta_us);
                    if band >= EMPTY_POOR_JUDGE_INDEX { band + 1 } else { band }
                } else if w.in_ms_band(delta_us) {
                    EMPTY_POOR_CODE
                } else {
                    IGNORED_CODE
                };
                if code < IGNORED_CODE {
                    let nearer = match chosen {
                        Some(c) => (l.notes[c].head_us - press_us).abs() > delta_us.abs(),
                        None => true,
                    };
                    if code < EMPTY_POOR_JUDGE_INDEX || nearer {
                        chosen = Some(i);
                    }
                } else {
                    chosen = None;
                }
            }
            i += 1;
        }
        (chosen, code)
    }

    fn press_new_note(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        let (chosen, code) = self.select_candidate(lane, press_us);
        let idx = chosen?;
        let judge = judge_of_code(code)?;
        let delta_us = self.lanes[lane].notes[idx].head_us - press_us;
        let vanish = self.prop.judge_vanish[judge as usize];
        let (long, charge) = {
            let note = &self.lanes[lane].notes[idx];
            (note.end_us.is_some(), is_charge(note.ln))
        };
        if long {
            if vanish {
                self.start_hold(lane, idx, dir, judge, delta_us);
            }
            if charge || !vanish {
                self.update_micro(lane, idx, judge, delta_us, false, vanish);
            }
        } else {
            if vanish {
                self.lanes[lane].notes[idx].judged = true;
            }
            self.update_micro(lane, idx, judge, delta_us, false, vanish);
        }
        Some(JudgeResult { judge, lane, note_index: idx, fast: delta_us >= 0, delta_us })
    }

    /// Take a long note under this lane's control (`JudgeManager.java:428-437, 444-451`), which
    /// only happens when the head judgment consumed it.
    fn start_hold(&mut self, lane: usize, idx: usize, dir: ScratchDir, judge: Judge, delta_us: i64) {
        self.lanes[lane].notes[idx].holding = true;
        let scratch = self.is_scratch(lane);
        let hold = &mut self.holds[lane];
        hold.note = Some(idx);
        hold.start_judge = Some(judge);
        hold.start_delta_us = delta_us;
        hold.release_us = None;
        hold.end_judge = None;
        hold.owner = if scratch { Some(dir) } else { None };
    }

    pub fn release(&mut self, lane: usize, release_us: i64) -> Option<JudgeResult> {
        self.release_dir(lane, ScratchDir::Forward, release_us)
    }

    /// Release a lane from one physical direction. A charge note on a scratch lane only accepts a
    /// mid-spin release from the direction that grabbed it and only from outside the end window
    /// (`JudgeManager.java:501-508`); a plain long note drops the second condition (`:526-532`).
    /// A release that came early and would only be a BAD is deferred rather than confirmed
    /// (`:509-512`), so re-grabbing inside the margin rescues it.
    pub fn release_dir(&mut self, lane: usize, dir: ScratchDir, release_us: i64) -> Option<JudgeResult> {
        if let Some(h) = self.held.get_mut(lane) {
            *h = false;
        }
        if lane >= self.lanes.len() {
            return None;
        }
        let idx = self.holds[lane].note?;
        let w = self.ln_window(lane);
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        let end_us = end_us?;
        let delta_us = end_us - release_us;
        let band = w.judge_code(delta_us);
        let scratch = self.is_scratch(lane);
        if charge {
            if scratch {
                if band != w.band_count() || self.holds[lane].owner != Some(dir) {
                    return None;
                }
                self.holds[lane].owner = None;
            }
            let judge = judge_of_code(band)?;
            if judge as usize >= DEFERRED_RELEASE_CODE && delta_us > 0 {
                self.holds[lane].release_us = Some(release_us);
                self.holds[lane].end_judge = Some(judge);
                return None;
            }
            return Some(self.confirm_hold(lane, idx, judge, delta_us, true));
        }
        if scratch {
            if self.holds[lane].owner != Some(dir) {
                return None;
            }
            self.holds[lane].owner = None;
        }
        let judge = worse(judge_of_code(band)?, self.holds[lane].start_judge.unwrap_or(Judge::Poor));
        let start_delta_us = self.holds[lane].start_delta_us;
        let delta_us = if start_delta_us.abs() > delta_us.abs() { start_delta_us } else { delta_us };
        if judge as usize >= DEFERRED_RELEASE_CODE && delta_us > 0 {
            self.holds[lane].release_us = Some(release_us);
            self.holds[lane].end_judge = Some(Judge::Bad);
            return None;
        }
        Some(self.confirm_hold(lane, idx, judge, delta_us, false))
    }

    /// Resolve the held note here and now, dropping the hold and its deferred-release state.
    fn confirm_hold(&mut self, lane: usize, idx: usize, judge: Judge, delta_us: i64, on_end: bool) -> JudgeResult {
        self.finish_hold(lane, idx, judge, on_end);
        self.update_micro(lane, idx, judge, delta_us, on_end, CONSUMES_THE_NOTE);
        JudgeResult { judge, lane, note_index: idx, fast: delta_us >= 0, delta_us }
    }

    fn finish_hold(&mut self, lane: usize, idx: usize, judge: Judge, on_end: bool) {
        let note = &mut self.lanes[lane].notes[idx];
        note.judged = true;
        note.holding = false;
        if on_end {
            note.end_judge = Some(judge);
        }
        self.holds[lane].clear();
    }

    /// Sweep notes that can no longer be hit (now past their latest BAD-late bound) into
    /// 見逃し POOR, confirm deferred and over-held long notes, pay out hell-charge gauge ticks and
    /// charge mine damage to lanes whose key is down.
    pub fn update(&mut self, now_us: i64) {
        self.sweep_mines(now_us);
        self.tick_hell_charges(now_us);
        let mut events: Vec<SweepEvent> = Vec::new();
        for lane in 0..self.lanes.len() {
            self.confirm_deferred(lane, now_us, &mut events);
            self.sweep_lane(lane, now_us, &mut events);
        }
        for e in events {
            self.update_micro(e.lane, e.note, e.judge, e.delta_us, e.on_end, CONSUMES_THE_NOTE);
        }
        self.prev_update_us = now_us;
    }

    fn sweep_mines(&mut self, now_us: i64) {
        for lane in 0..self.mines.len() {
            while self.mine_cursor[lane] < self.mines[lane].len() && self.mines[lane][self.mine_cursor[lane]].time_us <= now_us {
                let damage = self.mines[lane][self.mine_cursor[lane]].damage;
                if self.held.get(lane).copied().unwrap_or(false) {
                    self.gauge.add_value(-damage as f32);
                }
                self.mine_cursor[lane] += 1;
            }
        }
    }

    /// Hell-charge gauge ticks (`JudgeManager.java:305-336`). A note the play head is inside pays
    /// out half a GREAT every 200 ms while it is held (or while its end already took a GOOD or
    /// better) and half a BAD every 200 ms while it is not. At most one tick per frame, exactly as
    /// the reference's single `if` allows. Counts, combo and EX score are untouched.
    fn tick_hell_charges(&mut self, now_us: i64) {
        let step_us = now_us - self.prev_update_us;
        for lane in 0..self.lanes.len() {
            self.advance_passing(lane, now_us);
            let Some(idx) = self.holds[lane].passing else {
                continue;
            };
            let (started, end_judge) = {
                let note = &self.lanes[lane].notes[idx];
                (note.judged || note.holding, note.end_judge)
            };
            if !started {
                continue;
            }
            let increasing = self.held.get(lane).copied().unwrap_or(false) || matches!(end_judge, Some(j) if (j as usize) <= HCN_HELD_END_CODE);
            self.holds[lane].increasing = increasing;
            if increasing {
                self.holds[lane].passing_us += step_us;
                if self.holds[lane].passing_us > HCN_GAUGE_TICK_US {
                    self.gauge.update_with_rate(Judge::Great, HCN_GAUGE_TICK_RATE);
                    self.holds[lane].passing_us -= HCN_GAUGE_TICK_US;
                }
            } else {
                self.holds[lane].passing_us -= step_us;
                if self.holds[lane].passing_us < -HCN_GAUGE_TICK_US {
                    self.gauge.update_with_rate(Judge::Bad, HCN_GAUGE_TICK_RATE);
                    self.holds[lane].passing_us += HCN_GAUGE_TICK_US;
                }
            }
        }
    }

    /// Track which hell-charge note the play head is inside (`JudgeManager.java:235-242`): the head
    /// arms it and the end disarms it, clearing the tick balance.
    fn advance_passing(&mut self, lane: usize, now_us: i64) {
        loop {
            let cursor = self.lanes[lane].hcn_cursor;
            let Some(note) = self.lanes[lane].notes.get(cursor) else {
                return;
            };
            let head_us = note.head_us;
            let end_us = note.end_us;
            let hell_charge = is_hell_charge(note.ln);
            if head_us > now_us {
                return;
            }
            let span_end_us = end_us.filter(|_| hell_charge);
            if let Some(span_end_us) = span_end_us
                && span_end_us > now_us
            {
                self.holds[lane].passing = Some(cursor);
                return;
            }
            if span_end_us.is_some() && self.holds[lane].passing == Some(cursor) {
                self.holds[lane].passing = None;
                self.holds[lane].passing_us = 0;
            }
            self.lanes[lane].hcn_cursor += 1;
        }
    }

    /// Confirm a long note whose deferred release has outlasted the margin, or a plain one whose
    /// end time has gone by while it was still held (`JudgeManager.java:559-590`).
    fn confirm_deferred(&mut self, lane: usize, now_us: i64, events: &mut Vec<SweepEvent>) {
        let Some(idx) = self.holds[lane].note else {
            return;
        };
        let margin = self.ln_margin(lane);
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        let Some(end_us) = end_us else {
            return;
        };
        if let Some(release_us) = self.holds[lane].release_us
            && release_us + margin <= now_us
        {
            let judge = self.holds[lane].end_judge.unwrap_or(Judge::Poor);
            self.finish_hold(lane, idx, judge, charge);
            events.push(SweepEvent { lane, note: idx, judge, delta_us: end_us - release_us, on_end: charge });
            return;
        }
        if !charge && end_us < now_us {
            let judge = self.holds[lane].start_judge.unwrap_or(Judge::Poor);
            let delta_us = self.holds[lane].start_delta_us;
            self.finish_hold(lane, idx, judge, false);
            events.push(SweepEvent { lane, note: idx, judge, delta_us, on_end: false });
        }
    }

    /// 見逃し POOR sweep (`JudgeManager.java:592-628`): every unhit note now past its BAD-late
    /// bound, a charge note's head and end both, and a held charge note whose end went by unheld.
    fn sweep_lane(&mut self, lane: usize, now_us: i64, events: &mut Vec<SweepEvent>) {
        let miss_bound = self.note_window(lane).bd.0;
        loop {
            let cursor = self.lanes[lane].cursor;
            let Some(note) = self.lanes[lane].notes.get(cursor) else {
                return;
            };
            let (judged, holding, head_us, end_us, charge) = (note.judged, note.holding, note.head_us, note.end_us, is_charge(note.ln));
            if judged {
                self.lanes[lane].cursor += 1;
                continue;
            }
            if holding {
                let Some(end_us) = end_us else {
                    self.lanes[lane].cursor += 1;
                    continue;
                };
                if !charge || end_us - now_us >= miss_bound {
                    return;
                }
                self.finish_hold(lane, cursor, Judge::Poor, true);
                events.push(SweepEvent { lane, note: cursor, judge: Judge::Poor, delta_us: end_us - now_us, on_end: true });
                self.lanes[lane].cursor += 1;
                continue;
            }
            if head_us - now_us >= miss_bound {
                return;
            }
            let delta_us = head_us - now_us;
            self.lanes[lane].notes[cursor].judged = true;
            events.push(SweepEvent { lane, note: cursor, judge: Judge::Poor, delta_us, on_end: false });
            if charge && end_us.is_some() {
                events.push(SweepEvent { lane, note: cursor, judge: Judge::Poor, delta_us, on_end: true });
            }
            self.lanes[lane].cursor += 1;
        }
    }

    /// Count one judgment (`JudgeManager.java:639-655`). `vanish` is the reference's own
    /// `judgeVanish` argument — `judgeVanish[judge]` at a note head, and an unconditional `true`
    /// everywhere a long-note end, a deferred release or a 見逃し POOR sweep resolves a note — and
    /// only a vanishing judgment advances `passnotes`. Under [`MissCondition::One`] a 見逃し POOR on
    /// a note that already survived a judgment is consumed but not counted, which is what stops a
    /// pop'n BAD from tallying twice.
    fn update_micro(&mut self, lane: usize, idx: usize, judge: Judge, delta_us: i64, on_end: bool, vanish: bool) {
        if vanish {
            self.pass_notes += 1;
        }
        let note = &mut self.lanes[lane].notes[idx];
        let played = if on_end { note.end_play_time_us } else { note.head_play_time_us };
        if self.prop.miss_condition == MissCondition::One && judge == Judge::Poor && was_played(played) {
            return;
        }
        if on_end {
            note.end_play_time_us = delta_us;
        } else {
            note.head_play_time_us = delta_us;
        }
        self.apply(judge);
        self.record_timing(judge, delta_us);
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
    /// `early[i] + late[i] == counts[i]` (reference implementation `score.addJudgeCount(judge, mfast >= 0, 1)`).
    fn record_direction(&mut self, judge: Judge, dm: i64) {
        let i = judge as usize;
        if dm >= 0 {
            self.early[i] += 1;
        } else {
            self.late[i] += 1;
        }
    }

    /// Mean signed hit timing (µs) over actual hits — reference implementation `IRScoreData.avgjudge`. Positive =
    /// early on average. Zero when nothing was timed.
    pub fn avg_judge_us(&self) -> i64 {
        if self.timing_count == 0 { 0 } else { self.sum_delta_us / self.timing_count as i64 }
    }
}

/// Whether a stored play time reads back as non-zero through the reference's millisecond accessor
/// (`JudgeManager.java:398, 648`).
fn was_played(play_time_us: i64) -> bool {
    play_time_us / MICROS_PER_MILLI != 0
}

/// Extra slack the reference implementation subtracts when re-marking a lane's scan start each frame
/// (`JudgeManager.java:220`: `prevmtime + mjudgestart - 100000`).
const REMARK_SLACK_US: i64 = 100_000;

/// The `judgeVanish` argument of every `updateMicro` call that resolves a note outright: a
/// long-note end, a deferred release, a 見逃し POOR sweep (`JudgeManager.java:517, 567, 573, 583,
/// 596-618` all pass a literal `true`).
const CONSUMES_THE_NOTE: bool = true;

/// Stock long-note margin rate (percent) — reference implementation `PlayerConfig.longnoteMarginRate` default.
const DEFAULT_LONGNOTE_MARGIN_RATE: i32 = 100;

/// Chart total the mode-less constructors seed their gauges with until a caller supplies the real
/// one through [`JudgeEngine::set_gauge`].
const DEFAULT_GAUGE_TOTAL: f64 = 200.0;

#[cfg(test)]
mod algorithm_selection_tests {
    use super::*;
    use crate::algorithm::JudgeAlgorithm;

    const EARLY_NOTE_US: i64 = 100_000;
    const LATE_NOTE_US: i64 = 200_000;

    fn engine(algorithm: JudgeAlgorithm) -> JudgeEngine {
        let mut e = JudgeEngine::new(vec![vec![EARLY_NOTE_US, LATE_NOTE_US]], JudgeWindows::SEVENKEY_NOTE);
        e.set_algorithm(algorithm);
        e
    }

    #[test]
    fn the_default_algorithm_is_combo() {
        assert_eq!(JudgeEngine::new(vec![vec![0]], JudgeWindows::SEVENKEY_NOTE).algorithm(), JudgeAlgorithm::Combo);
    }

    #[test]
    fn apply_mode_builds_popn_under_the_pms_rule() {
        let mode = rbms_model::Mode::POPN_9K;
        let rank = JudgeWindowRule::Pms.judgerank_for_rank(2);
        let mut e = JudgeEngine::new(vec![Vec::new(); mode.key], JudgeWindows::SEVENKEY_NOTE);
        e.apply_mode(&mode, rank);
        let expected = JudgeWindowSet::for_mode(&mode, rank, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        assert_eq!(e.windows, expected.note, "PMS fixjudge holds PG/BAD/空POOR at their tabulated width");
        assert_eq!(e.ln_end, expected.ln_end);
        assert_ne!(e.windows, JudgeProperty::PMS.note.scaled(rank), "the NORMAL rule would have scaled PG as well");
    }

    #[test]
    fn from_model_for_mode_takes_the_judgerank_of_the_modes_own_rule() {
        let mode = rbms_model::Mode::POPN_9K;
        let model = Model {
            mode,
            meta: rbms_model::ModelMeta { total: 200.0, rank: 2, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines: Vec::new(),
            md5: String::new(),
            sha256: String::new(),
        };
        let engine = JudgeEngine::from_model_for_mode(&model);
        let expected = JudgeWindowSet::for_mode(&mode, JudgeWindowRule::Pms.judgerank_for_rank(2), UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        assert_eq!(engine.windows, expected.note, "the PMS rule resolves #RANK 2 to 70, not the NORMAL 75");
    }

    #[test]
    fn duration_takes_the_nearer_note_even_when_the_earlier_one_is_still_reachable() {
        let press_us = 160_000;
        assert_eq!(engine(JudgeAlgorithm::Duration).press(0, press_us).unwrap().note_index, 1);
    }

    #[test]
    fn combo_and_lowest_keep_the_earlier_note_that_duration_would_abandon() {
        let press_us = 160_000;
        for algorithm in [JudgeAlgorithm::Combo, JudgeAlgorithm::Lowest] {
            assert_eq!(engine(algorithm).press(0, press_us).unwrap().note_index, 0, "{algorithm:?}");
        }
    }

    #[test]
    fn score_abandons_a_note_that_can_no_longer_be_a_great_while_combo_keeps_it() {
        let press_us = 170_000;
        assert_eq!(engine(JudgeAlgorithm::Score).press(0, press_us).unwrap().note_index, 1);
        assert_eq!(engine(JudgeAlgorithm::Combo).press(0, press_us).unwrap().note_index, 0);
    }

    #[test]
    fn score_keeps_a_note_that_is_still_a_great() {
        let press_us = 160_000;
        assert_eq!(engine(JudgeAlgorithm::Score).press(0, press_us).unwrap().note_index, 0);
    }

    #[test]
    fn every_algorithm_still_takes_the_only_candidate() {
        for algorithm in JudgeAlgorithm::ALL {
            let mut e = JudgeEngine::new(vec![vec![EARLY_NOTE_US]], JudgeWindows::SEVENKEY_NOTE);
            e.set_algorithm(algorithm);
            let r = e.press(0, EARLY_NOTE_US).unwrap();
            assert_eq!(r.judge, Judge::PerfectGreat, "{algorithm:?}");
            assert_eq!(r.note_index, 0, "{algorithm:?}");
        }
    }

    #[test]
    fn scratch_lanes_are_judged_against_the_scratch_table() {
        let model_mode = rbms_model::Mode::BEAT_7K;
        let mut e = JudgeEngine::new(vec![Vec::new(); model_mode.key], JudgeWindows::SEVENKEY_NOTE);
        e.apply_mode(&model_mode, 100);
        assert!(e.is_scratch(model_mode.key - 1), "lane 7 is the 7K scratch");
        assert_eq!(e.note_window(model_mode.key - 1), JudgeProperty::SEVENKEYS.scratch);
    }

    #[test]
    fn a_judged_first_candidate_is_always_replaced_even_under_lowest() {
        let mut e = engine(JudgeAlgorithm::Lowest);
        assert_eq!(e.press(0, EARLY_NOTE_US).unwrap().judge, Judge::PerfectGreat, "the first note is consumed");
        let r = e.press(0, LATE_NOTE_US).unwrap();
        assert_eq!(r.note_index, 1, "JudgeManager.java:396 replaces an already-judged t1 whatever the algorithm says");
        assert_eq!(r.judge, Judge::PerfectGreat);
        assert_eq!(e.counts[5], 0, "the consumed note never becomes the chosen candidate again");
    }

    #[test]
    fn a_press_inside_the_gate_but_past_every_note_band_yields_nothing() {
        let mode = rbms_model::Mode::BEAT_7K;
        let mut e = JudgeEngine::new(vec![vec![1_000_000]; mode.key], JudgeWindows::SEVENKEY_NOTE);
        e.apply_mode(&mode, 100);
        let (gate_late, _) = e.gate;
        assert_eq!(gate_late, JudgeProperty::SEVENKEYS.scratch.bd.0, "the scratch table widens the shared gate past the key table's own bands");
        let press_us = 1_000_000 + JudgeProperty::SEVENKEYS.note.bd.0.abs() + 5_000;
        assert!(1_000_000 - press_us > gate_late, "the press is still inside the shared candidate gate");
        assert!(e.press(0, press_us).is_none(), "judge code 6: inside the gate, outside every window of this lane's table");
        assert_eq!(e.counts, [0; 6], "a cancelled press tallies nothing at all");
    }

    #[test]
    fn a_play_time_inside_one_millisecond_reads_back_as_never_played() {
        assert!(!was_played(0), "a note nothing has judged yet");
        assert!(!was_played(MICROS_PER_MILLI - 1), "Note.getPlayTime() truncates to milliseconds (JudgeManager.java:398, 648)");
        assert!(!was_played(-(MICROS_PER_MILLI - 1)));
        assert!(was_played(MICROS_PER_MILLI));
        assert!(was_played(-MICROS_PER_MILLI));
    }

    #[test]
    fn an_empty_poor_candidate_only_wins_on_a_strictly_smaller_delta() {
        let build = || {
            let mut e = JudgeEngine::new(vec![vec![1_000_000, 1_100_000]], JudgeWindows::SEVENKEY_NOTE);
            e.press(0, 1_000_000);
            e.press(0, 1_100_000);
            e
        };
        assert_eq!(build().press(0, 1_060_000).unwrap().note_index, 1, "JudgeManager.java:408-410 takes the nearer of two consumed notes");
        assert_eq!(build().press(0, 1_040_000).unwrap().note_index, 0, "and keeps the first one when it is no further away");
    }

    #[test]
    fn the_gate_excludes_its_own_early_bound() {
        let mut at_bound = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert!(at_bound.press(0, 500_000).is_none(), "JudgeManager.java:387 breaks on dmtime >= mjudgeend");
        let mut inside = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(inside.press(0, 500_001).unwrap().judge, Judge::Miss, "one microsecond inside the gate is still an empty poor");
    }
}
