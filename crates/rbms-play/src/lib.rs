#![forbid(unsafe_code)]

use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeAutoShift, GaugeShiftOutcome, clamp_bottom_shiftable};
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;
pub use rbms_judge::matcher::ScratchDir;
use rbms_judge::windows::{JudgeWindowRule, JudgeWindowSet};
use rbms_judge::{GaugeKind, JudgeEngine, JudgeResult};
use rbms_model::{Model, NoteKind};

mod session;

pub use session::{
    ANALYSIS_RATE_MAX, ANALYSIS_RATE_MIN, ANALYSIS_RATE_STEP, ANALYSIS_SEEK_STEP_US, JudgeSetup, KEYSOUND_GAIN, KEYSOUND_PAN, KEYSOUND_PITCH, NullSink,
    PlaySession, PlaySummary, SessionClock, SessionOptions, SoundRequest, SoundSink, SoundTime, TimingMark,
};

/// Which output bus a keysound belongs to. `rbms-play` does not depend on `rbms-audio`, so it
/// carries its own discriminant and the caller maps it one-to-one onto `rbms_audio::Bus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaySource {
    /// BGM channel (automatic accompaniment); maps to the audio `Bg` bus.
    Bgm,
    /// Note keysound (an autoplay note press, or an interactive hit); maps to the audio `Key` bus.
    Key,
}

/// A keysound that should fire at `at_us` (autoplay BGM, or a hit note's sound).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayEvent {
    pub wav: i32,
    pub at_us: i64,
    pub source: PlaySource,
}

enum AutoAction {
    Press { wav: i32, ln: bool },
    Release,
}

/// Autoplay key-beam flash for a tapped (non-LN) note, mirroring the reference implementation's
/// `auto_minduration` (80 ms): a tap lights its lane beam for this long, then releases.
const AUTO_BEAM_US: i64 = 80_000;

/// JUDGE WIDTH percentage that leaves a mode's own timing windows alone.
pub(crate) const UNMODIFIED_RATE_PERCENT: i32 = 100;

/// How many judge tiers a JUDGE WIDTH rate covers: PGREAT, GREAT and GOOD. BAD and the 空POOR band
/// never widen (`JudgeProperty.java:262`).
pub const JUDGE_WIDTH_TIER_COUNT: usize = 3;

/// JUDGE WIDTH rates leaving all three widenable tiers at their stock width.
pub const UNMODIFIED_JUDGE_RATES: [i32; JUDGE_WIDTH_TIER_COUNT] = [UNMODIFIED_RATE_PERCENT; JUDGE_WIDTH_TIER_COUNT];

fn collect_bg(model: &Model) -> Vec<(i64, i32)> {
    let mut v: Vec<(i64, i32)> = model.timelines.iter().flat_map(|tl| tl.bgnotes.iter().map(|n| (n.time_us, n.wav))).collect();
    v.sort_by_key(|e| e.0);
    v
}

/// `(head_time, lane, wav)` for every judgeable note head (Normal + LN start). Used for
/// interactive keysound lookup.
fn collect_note_heads(model: &Model) -> Vec<(i64, usize, i32)> {
    let mut v = Vec::new();
    for tl in &model.timelines {
        for (lane, slot) in tl.notes.iter().enumerate() {
            if let Some(note) = slot {
                match note.kind {
                    NoteKind::Normal | NoteKind::LongStart { .. } => v.push((note.time_us, lane, note.wav)),
                    _ => {}
                }
            }
        }
    }
    v.sort_by_key(|e| e.0);
    v
}

/// Autoplay judge actions, time-sorted: a press for every note head (LN heads too) and a
/// release for every LN end.
fn collect_actions(model: &Model) -> Vec<(i64, usize, AutoAction)> {
    let n = model.mode.key;
    let mut v = Vec::new();
    for lane in 0..n {
        let mut pending_head: Option<(i64, i32)> = None;
        for tl in &model.timelines {
            let Some(note) = &tl.notes[lane] else {
                continue;
            };
            match note.kind {
                NoteKind::Normal => v.push((note.time_us, lane, AutoAction::Press { wav: note.wav, ln: false })),
                NoteKind::LongStart { .. } => pending_head = Some((note.time_us, note.wav)),
                NoteKind::LongEnd { .. } => {
                    if let Some((head_us, wav)) = pending_head.take() {
                        v.push((head_us, lane, AutoAction::Press { wav, ln: true }));
                        v.push((note.time_us, lane, AutoAction::Release));
                    }
                }
                NoteKind::Mine { .. } => {}
            }
        }
    }
    v.sort_by_key(|e| e.0);
    v
}

/// Give `judge` all four timing tables of `model`'s mode, scaled by the chart's judgerank under the
/// mode's [`JudgeWindowRule`] and then by the user's JUDGE WIDTH rates.
///
/// The rule decides both the judgerank a `#RANK`/`#DEFEXRANK` resolves to and which judge indices
/// hold their tabulated width, so it has to be picked from the mode rather than assumed.
fn apply_window_set(judge: &mut JudgeEngine, model: &Model, key: [i32; JUDGE_WIDTH_TIER_COUNT], scratch: [i32; JUDGE_WIDTH_TIER_COUNT]) {
    let rule = JudgeWindowRule::for_mode(&model.mode);
    let judgerank = rule.judgerank_for(model.meta.rank, model.meta.defexrank);
    judge.apply_window_set(&JudgeWindowSet::for_mode(&model.mode, judgerank, key, scratch));
}

/// Run a full autoplay pass and return the resulting judge engine. Verification helper
/// (autoplay = all PGREAT, LNs counted once).
pub fn simulate_autoplay(model: &Model) -> JudgeEngine {
    let mut p = Player::new(model.clone(), true);
    let end = p.last_time_us() + 1_000_000;
    p.update(end, |_| {});
    p.into_judge()
}

/// Real-time play driver. Advances over the song clock, emitting keysound events and (in
/// autoplay) feeding perfect judgments including LN releases.
pub struct Player {
    model: Model,
    judge: JudgeEngine,
    bg: Vec<(i64, i32)>,
    bg_cursor: usize,
    heads: Vec<(i64, usize, i32)>,
    actions: Vec<(i64, usize, AutoAction)>,
    action_cursor: usize,
    judge_cursor: usize,
    autoplay: bool,
    auto_lanes: Vec<bool>,
    beam_on: Vec<i64>,
    beam_off: Vec<i64>,
    ln_active: Vec<bool>,
    bomb: Vec<(i64, u8)>,
    gauge_auto_shift: GaugeAutoShift,
    configured_gauge: GaugeKind,
    bottom_shiftable_gauge: GaugeKind,
    failed: bool,
}

impl Player {
    pub fn new(model: Model, autoplay: bool) -> Self {
        let mut judge = JudgeEngine::from_model_for_mode(&model);
        apply_window_set(&mut judge, &model, UNMODIFIED_JUDGE_RATES, UNMODIFIED_JUDGE_RATES);
        let bg = collect_bg(&model);
        let heads = collect_note_heads(&model);
        let actions = collect_actions(&model);
        let lanes = model.mode.key;
        let auto_lanes = vec![false; lanes];
        Player {
            model,
            judge,
            bg,
            bg_cursor: 0,
            heads,
            actions,
            action_cursor: 0,
            judge_cursor: 0,
            autoplay,
            auto_lanes,
            beam_on: vec![i64::MIN; lanes],
            beam_off: vec![i64::MIN; lanes],
            ln_active: vec![false; lanes],
            bomb: vec![(i64::MIN, 0); lanes],
            gauge_auto_shift: GaugeAutoShift::default(),
            configured_gauge: GaugeKind::Normal,
            bottom_shiftable_gauge: GaugeKind::AssistEasy,
            failed: false,
        }
    }

    /// Apply a single user JUDGE WIDTH rate (percent; 100 = chart default) to PGREAT/GREAT/GOOD on
    /// both key and scratch lanes. Kept for callers that expose one slider; see
    /// [`set_judge_window_rates`](Self::set_judge_window_rates) for the reference implementation's six-value form.
    pub fn set_judge_rate(&mut self, rate_percent: i32) {
        self.set_judge_window_rates([rate_percent; JUDGE_WIDTH_TIER_COUNT], [rate_percent; JUDGE_WIDTH_TIER_COUNT]);
    }

    /// Reference implementation JUDGE WIDTH: per-tier `[PGREAT, GREAT, GOOD]` percentages for key lanes and for
    /// scratch lanes (`JudgeManager.java:169-174`). The chart's own judgerank (`#RANK`/`#DEFEXRANK`)
    /// is applied first, then these rates — BAD and the 空POOR band never widen, and each tier is
    /// clamped to BAD and to the tier before it.
    ///
    /// Both passes run under the mode's [`JudgeWindowRule`], so a pop'n chart takes the PMS
    /// judgerank column and its per-index fixjudge rather than the beat-mode one.
    pub fn set_judge_window_rates(&mut self, key: [i32; JUDGE_WIDTH_TIER_COUNT], scratch: [i32; JUDGE_WIDTH_TIER_COUNT]) {
        apply_window_set(&mut self.judge, &self.model, key, scratch);
    }

    /// The judgerank rule this chart's mode is judged under (`JudgeProperty.java:21, 32, 43, 54`).
    pub fn judge_window_rule(&self) -> JudgeWindowRule {
        JudgeWindowRule::for_mode(&self.model.mode)
    }

    /// Reference implementation LONGNOTE MARGIN rate (percent of the mode's stock margin).
    pub fn set_longnote_margin_rate(&mut self, rate_percent: i32) {
        self.judge.set_longnote_margin_rate(rate_percent);
    }

    /// Which note a press takes when several are in range (`JudgeAlgorithm.java:17-37`).
    pub fn set_algorithm(&mut self, algorithm: JudgeAlgorithm) {
        self.judge.set_algorithm(algorithm);
    }

    /// The candidate-selection policy in force.
    pub fn algorithm(&self) -> JudgeAlgorithm {
        self.judge.algorithm()
    }

    /// Resolve long notes the chart left unstated to `mode`. Charge notes are judged at both ends,
    /// so this re-counts the chart; call it before the gauge is set.
    pub fn set_ln_mode(&mut self, mode: LnMode) {
        self.judge.set_ln_mode(mode);
    }

    /// The long-note flavour unstated chart notes play as.
    pub fn ln_mode(&self) -> LnMode {
        self.judge.ln_mode()
    }

    /// Play on a gauge table other than the one the mode selects — the LR2 set, which no mode
    /// reaches (`GaugeProperty.java:117-125`).
    pub fn set_gauge_set(&mut self, set: GaugeSetId) {
        self.judge.set_gauge_set(set);
    }

    /// The gauge table the nine gauges are built from. Derived from the chart's mode unless
    /// [`set_gauge_set`](Self::set_gauge_set) overrode it.
    pub fn gauge_set(&self) -> GaugeSetId {
        self.judge.gauge_set()
    }

    /// How the selected gauge may move during play (`PlayerConfig.java:163-167`).
    pub fn set_gauge_auto_shift(&mut self, mode: GaugeAutoShift) {
        self.gauge_auto_shift = mode;
    }

    /// The gauge auto-shift mode in force.
    pub fn gauge_auto_shift(&self) -> GaugeAutoShift {
        self.gauge_auto_shift
    }

    /// The floor the per-frame auto-shift may drop the selection to, clamped to the range the
    /// reference allows (`PlayerConfig.java:908`).
    pub fn set_bottom_shiftable_gauge(&mut self, kind: GaugeKind) {
        self.bottom_shiftable_gauge = clamp_bottom_shiftable(kind);
    }

    /// The auto-shift floor.
    pub fn bottom_shiftable_gauge(&self) -> GaugeKind {
        self.bottom_shiftable_gauge
    }

    /// Whether the selected gauge emptied under a shift mode that does not rescue it
    /// (`BMSPlayer.java:653-661`). The run keeps judging; ending it is the caller's decision.
    pub fn failed(&self) -> bool {
        self.failed
    }

    /// Per-lane key-beam press timestamps (µs); `i64::MIN` means the beam is not held. Paired
    /// with [`beam_off`](Self::beam_off) so the renderer can show a hold then a release fade.
    pub fn beam_on(&self) -> &[i64] {
        &self.beam_on
    }

    /// Per-lane key-beam release timestamps (µs); `i64::MIN` means no release fade is pending.
    pub fn beam_off(&self) -> &[i64] {
        &self.beam_off
    }

    /// Per-lane key-bomb state `(hit_us, judge_index)` — set on a note hit (PG/GR/GD/BD, not empty
    /// POOR or miss); `i64::MIN` means no bomb. The renderer fades it over the skin's bomb window.
    pub fn bomb(&self) -> &[(i64, u8)] {
        &self.bomb
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Mark lanes (e.g. an auto-scratch lane) as auto-played: their notes are hit from
    /// the chart even in interactive mode, and key input on them is ignored.
    pub fn set_auto_lanes(&mut self, auto: Vec<bool>) {
        if auto.len() == self.auto_lanes.len() {
            self.auto_lanes = auto;
        }
    }

    /// Select the gauge the player chose. It is also the ceiling the SELECT TO UNDER auto-shift
    /// re-picks under, so the choice is remembered rather than only handed to the engine.
    pub fn set_gauge(&mut self, kind: GaugeKind) {
        self.configured_gauge = kind;
        self.judge.set_gauge(kind, self.model.meta.total);
    }

    /// The gauge the player chose, which auto-shift may have moved the selection away from.
    pub fn configured_gauge(&self) -> GaugeKind {
        self.configured_gauge
    }

    /// Read-only access to the judge engine. The field itself is private, so the engine can only be
    /// driven through this type's own methods.
    pub fn judge(&self) -> &JudgeEngine {
        &self.judge
    }

    pub fn into_judge(self) -> JudgeEngine {
        self.judge
    }

    /// Schedule axis (the audio engine's `scheduled_us`, i.e. lookahead included). Advances the BGM
    /// and autoplay keysound cursors and emits the sounds to reserve. Performs no judging, no beam
    /// bookkeeping and no miss sweep, so running it ahead of the judge axis cannot shift judgments.
    pub fn update_schedule<F: FnMut(PlayEvent)>(&mut self, sched_us: i64, mut play: F) {
        while self.bg_cursor < self.bg.len() && self.bg[self.bg_cursor].0 <= sched_us {
            let (at, wav) = self.bg[self.bg_cursor];
            play(PlayEvent { wav, at_us: at, source: PlaySource::Bgm });
            self.bg_cursor += 1;
        }
        while self.action_cursor < self.actions.len() && self.actions[self.action_cursor].0 <= sched_us {
            let (at, lane, ref kind) = self.actions[self.action_cursor];
            if (self.autoplay || self.auto_lanes.get(lane).copied().unwrap_or(false))
                && let AutoAction::Press { wav, .. } = kind
            {
                play(PlayEvent { wav: *wav, at_us: at, source: PlaySource::Key });
            }
            self.action_cursor += 1;
        }
    }

    /// Judge axis (the audio engine's `audible_us`, i.e. what the player is hearing right now).
    /// Runs the autoplay press/release judgments over its own `judge_cursor`, the key-beam timer and
    /// the miss sweep. Emits no sound: [`update_schedule`](Self::update_schedule) already reserved it.
    pub fn update_judge(&mut self, audible_us: i64) {
        while self.judge_cursor < self.actions.len() && self.actions[self.judge_cursor].0 <= audible_us {
            let (at, lane, ref kind) = self.actions[self.judge_cursor];
            if self.autoplay || self.auto_lanes.get(lane).copied().unwrap_or(false) {
                match kind {
                    AutoAction::Press { ln, .. } => {
                        if let Some(jr) = self.judge.press(lane, at)
                            && (jr.judge as usize) <= 3
                        {
                            self.bomb[lane] = (at, jr.judge as u8);
                        }
                        self.beam_on[lane] = at;
                        self.beam_off[lane] = i64::MIN;
                        if *ln {
                            self.ln_active[lane] = true;
                        }
                    }
                    AutoAction::Release => {
                        if let Some(jr) = self.judge.release(lane, at)
                            && (jr.judge as usize) <= 3
                        {
                            self.bomb[lane] = (at, jr.judge as u8);
                        }
                        self.beam_off[lane] = at;
                        self.beam_on[lane] = i64::MIN;
                        self.ln_active[lane] = false;
                    }
                }
            }
            self.judge_cursor += 1;
        }
        for lane in 0..self.beam_on.len() {
            let auto = self.autoplay || self.auto_lanes[lane];
            if auto && self.beam_on[lane] != i64::MIN && !self.ln_active[lane] && audible_us - self.beam_on[lane] >= AUTO_BEAM_US {
                self.beam_off[lane] = self.beam_on[lane] + AUTO_BEAM_US;
                self.beam_on[lane] = i64::MIN;
            }
        }
        self.judge.update(audible_us);
        self.run_gauge_auto_shift();
    }

    /// One frame of gauge auto-shift, run after the frame's judgments exactly like the reference's
    /// play loop (`BMSPlayer.java:638-672`). BEST CLEAR and SELECT TO UNDER re-pick every frame; the
    /// other three only act once the selected gauge is empty, and only NONE ends the play there.
    fn run_gauge_auto_shift(&mut self) {
        if self.failed {
            return;
        }
        let outcome = self.judge.gauge.auto_shift(self.gauge_auto_shift, self.configured_gauge, self.bottom_shiftable_gauge);
        self.failed = outcome == GaugeShiftOutcome::Failed;
    }

    /// Both axes at one clock: [`update_schedule`](Self::update_schedule) then
    /// [`update_judge`](Self::update_judge). Kept for the virtual-clock paths (offline analysis,
    /// autoplay simulation, tests); the real-time path drives the two axes separately so that the
    /// scheduler's lookahead never reaches judging.
    pub fn update<F: FnMut(PlayEvent)>(&mut self, now_us: i64, play: F) {
        self.update_schedule(now_us, play);
        self.update_judge(now_us);
    }

    pub fn press<F: FnMut(PlayEvent)>(&mut self, lane: usize, now_us: i64, play: F) -> Option<JudgeResult> {
        self.press_dir(lane, ScratchDir::Forward, now_us, play)
    }

    /// Press a lane from one physical direction. On a scratch lane the two directions are two keys,
    /// and spinning the other way ends a charge note the lane is holding
    /// (`JudgeManager.java:358-372`); a key lane only ever presses forwards.
    pub fn press_dir<F: FnMut(PlayEvent)>(&mut self, lane: usize, dir: ScratchDir, now_us: i64, mut play: F) -> Option<JudgeResult> {
        if self.auto_lanes.get(lane).copied().unwrap_or(false) {
            return None;
        }
        if lane < self.beam_on.len() {
            self.beam_on[lane] = now_us;
            self.beam_off[lane] = i64::MIN;
        }
        if let Some(wav) = self.nearest_head_wav(lane, now_us) {
            play(PlayEvent { wav, at_us: now_us, source: PlaySource::Key });
        }
        let res = self.judge.press_dir(lane, dir, now_us);
        if let Some(jr) = &res
            && (jr.judge as usize) <= 3
            && jr.lane < self.bomb.len()
        {
            self.bomb[jr.lane] = (now_us, jr.judge as u8);
        }
        res
    }

    pub fn release(&mut self, lane: usize, now_us: i64) -> Option<JudgeResult> {
        self.release_dir(lane, ScratchDir::Forward, now_us)
    }

    /// Release a lane from one physical direction. An auto-played lane drops the input exactly as
    /// [`press_dir`](Self::press_dir) does: the chart is holding its long notes, and letting a stray
    /// key-up through would break one the player never grabbed.
    pub fn release_dir(&mut self, lane: usize, dir: ScratchDir, now_us: i64) -> Option<JudgeResult> {
        if self.auto_lanes.get(lane).copied().unwrap_or(false) {
            return None;
        }
        if lane < self.beam_on.len() {
            if self.beam_on[lane] != i64::MIN {
                self.beam_off[lane] = now_us;
            }
            self.beam_on[lane] = i64::MIN;
            self.ln_active[lane] = false;
        }
        let res = self.judge.release_dir(lane, dir, now_us);
        if let Some(jr) = &res
            && (jr.judge as usize) <= 3
            && jr.lane < self.bomb.len()
        {
            self.bomb[jr.lane] = (now_us, jr.judge as u8);
        }
        res
    }

    fn nearest_head_wav(&self, lane: usize, now_us: i64) -> Option<i32> {
        self.heads.iter().filter(|(_, l, _)| *l == lane).min_by_key(|(t, _, _)| (t - now_us).abs()).map(|(_, _, wav)| *wav)
    }

    pub fn last_time_us(&self) -> i64 {
        self.model.timelines.last().map(|t| t.time_us).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_chart::to_model;
    use rbms_model::Mode;
    use rbms_parser::parse;

    fn model(bms: &[u8]) -> Model {
        to_model(&parse(bms), Mode::BEAT_7K)
    }

    #[test]
    fn autoplay_achieves_perfect_score() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00211:01010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "all PGREAT");
        assert_eq!(engine.ex_score, 2 * n as u32);
        assert_eq!(engine.max_combo, n as u32);
        assert_eq!(engine.counts[5], 0, "no miss");
    }

    #[test]
    fn autoplay_handles_ln_as_single_judge() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        assert_eq!(n, 1, "one LN = one playable note");
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], 1, "LN judged once as PGREAT");
        assert_eq!(engine.counts.iter().sum::<u32>(), 1);
        assert_eq!(engine.max_combo, 1);
    }

    #[test]
    fn player_autoplay_emits_keysounds_and_judges() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00101:01\r\n#00111:0101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let mut p = Player::new(m, true);
        let mut events = Vec::new();
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |e| events.push(e));
        assert!(events.len() >= n, "emitted bg + note keysounds");
        assert_eq!(p.judge().counts[0], n as u32);
    }

    #[test]
    fn auto_lane_judged_without_input() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        assert_eq!(p.judge().counts[0], 1, "scratch lane auto-judged as PGREAT");
        assert!(p.press(7, last, |_| {}).is_none(), "input on auto lane is ignored");
    }

    #[test]
    fn interactive_press_sets_beam_release_clears_it() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        assert_eq!(p.beam_on()[0], i64::MIN, "beam off initially");
        p.press(0, 1_000_000, |_| {});
        assert_eq!(p.beam_on()[0], 1_000_000, "beam lit on press");
        assert_eq!(p.beam_off()[0], i64::MIN);
        p.release(0, 1_050_000);
        assert_eq!(p.beam_on()[0], i64::MIN, "beam cleared on release");
        assert_eq!(p.beam_off()[0], 1_050_000, "release fade timestamp set");
    }

    #[test]
    fn note_hit_lights_key_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        assert_eq!(p.bomb()[0].0, i64::MIN, "no bomb before any hit");
        let res = p.press(0, 2_000_000, |_| {});
        assert!(res.is_some(), "clean press judges the note");
        assert_eq!(p.bomb()[0].0, 2_000_000, "bomb lit at the hit time");
        assert!((p.bomb()[0].1 as usize) <= 3, "bomb judge index is a hit (PG/GR/GD/BD)");
    }

    #[test]
    fn empty_press_does_not_light_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.press(0, 100_000, |_| {});
        assert_eq!(p.bomb()[0].0, i64::MIN, "a press far from any note lights no bomb");
    }

    #[test]
    fn autoplay_tap_beam_flashes_then_clears_while_ln_holds() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00112:01\r\n");
        let mut p = Player::new(m, true);
        p.update(2_200_000, |_| {});
        assert_ne!(p.beam_on()[0], i64::MIN, "LN lane beam held through the long note");
        assert_eq!(p.beam_on()[1], i64::MIN, "tapped lane beam auto-cleared after the 80ms flash");
        assert_ne!(p.beam_off()[1], i64::MIN, "tapped lane left a release-fade timestamp");
    }

    #[test]
    fn judge_rate_widens_windows() {
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut narrow = Player::new(m.clone(), false);
        assert_eq!(narrow.press(0, nt - 17_000, |_| {}).unwrap().judge, rbms_judge::Judge::Great, "17ms off is GREAT at default (#RANK 2 = 75%, PG ±15ms)");
        let mut wide = Player::new(m, false);
        wide.set_judge_rate(200);
        assert_eq!(wide.press(0, nt - 17_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "200% width makes 17ms off a PGREAT");
    }

    #[test]
    fn judge_width_does_not_widen_bad() {
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m.clone(), false);
        p.set_judge_rate(200);
        assert!(p.press(0, nt + 250_000, |_| {}).is_none(), "250ms late is past the BAD edge, width cannot reach it");
        let mut q = Player::new(m, false);
        assert_eq!(q.press(0, nt + 200_000, |_| {}).unwrap().judge, rbms_judge::Judge::Bad, "200ms late is inside the fixed BAD window at stock width");
    }

    #[test]
    fn the_key_judge_width_rate_drives_the_scratch_lane_too() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n#00116:01\r\n");
        let key_t = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let scr_t = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();

        let mut stock = Player::new(m.clone(), false);
        assert_eq!(stock.press(0, key_t - 15_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "15ms off is inside the stock key PGREAT");
        assert_eq!(stock.press(7, scr_t - 25_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "and 25ms off is inside the stock scratch PGREAT");

        let mut p = Player::new(m, false);
        p.set_judge_window_rates([50, 100, 100], [100, 100, 100]);
        assert_eq!(p.press(0, key_t - 15_000, |_| {}).unwrap().judge, rbms_judge::Judge::Great, "50% key rate pulls the key PGREAT in to +-10ms");
        assert_eq!(
            p.press(7, scr_t - 25_000, |_| {}).unwrap().judge,
            rbms_judge::Judge::Great,
            "JudgeManager.java:198 builds every judged table from the key rate, so the scratch PGREAT comes in to +-15ms as well"
        );
    }

    #[test]
    fn the_scratch_judge_width_rate_never_moves_a_judgement() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n#00116:01\r\n");
        let key_t = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let scr_t = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();

        let mut p = Player::new(m, false);
        p.set_judge_window_rates(UNMODIFIED_JUDGE_RATES, [50, 50, 50]);
        assert_eq!(p.press(0, key_t - 15_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "the key table is untouched");
        assert_eq!(
            p.press(7, scr_t - 25_000, |_| {}).unwrap().judge,
            rbms_judge::Judge::PerfectGreat,
            "the scratch rate only ever fed the candidate gate (JudgeManager.java:185-197)"
        );
    }

    #[test]
    fn dangling_longstart_does_not_stick_beam() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000000\r\n");
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |_| {});
        assert_eq!(p.beam_on()[0], i64::MIN, "an unterminated LN (no LongEnd) must not leave the beam stuck on");
    }

    #[test]
    fn interactive_press_and_release_judges_ln() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let times: Vec<i64> = m.timelines.iter().flat_map(|t| t.notes[0].as_ref()).map(|nn| nn.time_us).collect();
        let (head, end) = (times[0], times[1]);
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        let r = p.press(0, head, |e| events.push(e));
        assert_eq!(r.unwrap().judge, rbms_judge::Judge::PerfectGreat, "LN head judged");
        let rr = p.release(0, end);
        assert_eq!(rr.unwrap().judge, rbms_judge::Judge::PerfectGreat, "LN release judged");
        assert_eq!(p.judge().counts[0], 1);
    }

    #[test]
    fn ln_release_window_scales_with_rank() {
        let ln = |bms: &[u8]| {
            let m = model(bms);
            let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
            (m, t[0], t[1])
        };

        let (m, head, end) = ln(b"#BPM 120\r\n#RANK 1\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let mut hard = Player::new(m, false);
        assert_eq!(hard.press(0, head, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat);
        assert_eq!(hard.release(0, end - 100_000).unwrap().judge, rbms_judge::Judge::Good, "100ms-early LN release is GOOD at #RANK 1");

        let (m2, head2, end2) = ln(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let mut normal = Player::new(m2, false);
        assert_eq!(normal.press(0, head2, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat);
        assert_eq!(normal.release(0, end2 - 100_000).unwrap().judge, rbms_judge::Judge::PerfectGreat, "same release is PGREAT at #RANK 3");
    }

    /// Run a full autoplay pass and collect every emitted keysound, plus the final engine.
    fn autoplay_collect(m: Model) -> (JudgeEngine, Vec<PlayEvent>) {
        let mut p = Player::new(m, true);
        let mut events = Vec::new();
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |e| events.push(e));
        (p.into_judge(), events)
    }

    #[test]
    fn autoplay_dense_chart_all_pgreat_exscore_2n_no_miss() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:0101010101010101\r\n#00112:0101010101010101\r\n#00113:0101010101010101\r\n#00211:0101010101010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "every note PGREAT");
        assert_eq!(engine.ex_score, 2 * n as u32, "EX == 2n for an all-PGREAT clear");
        assert_eq!(engine.max_combo, n as u32, "full combo");
        assert_eq!(engine.counts[5], 0, "no miss");
        assert_eq!(engine.counts.iter().sum::<u32>(), n as u32, "exactly n judgments, LNs not double-counted");
    }

    #[test]
    fn autoplay_ignores_mines_count_and_judges() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#001D1:01010101\r\n#00112:01\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        assert_eq!(n, 1, "mines are excluded from the playable count");
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], 1, "only the real note is judged (PGREAT)");
        assert_eq!(engine.counts.iter().sum::<u32>(), 1, "mines never produce a judgment");
    }

    #[test]
    fn autoplay_mixed_ln_and_normal_counts_each_once() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00112:01010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "LN + normals all PGREAT");
        assert_eq!(engine.ex_score, 2 * n as u32);
        assert_eq!(engine.max_combo, n as u32);
    }

    #[test]
    fn autoplay_empty_chart_yields_no_judgments() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00101:01\r\n");
        assert_eq!(rbms_chart::count_playable_notes(&m), 0, "BGM-only chart has no playable notes");
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts, [0; 6]);
        assert_eq!(engine.ex_score, 0);
        assert_eq!(engine.max_combo, 0);
        assert_eq!(engine.total_judged(), 0);
    }

    #[test]
    fn autoplay_is_deterministic_across_runs() {
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01010101\r\n#00116:01010101\r\n#00112:01000001\r\n");
        let a = simulate_autoplay(&m);
        let b = simulate_autoplay(&m);
        assert_eq!(a.counts, b.counts);
        assert_eq!(a.ex_score, b.ex_score);
        assert_eq!(a.max_combo, b.max_combo);
        assert_eq!(a.empty_poor, b.empty_poor);
    }

    #[test]
    fn autoplay_scratch_lane_judged_like_keys() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00116:01010101\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let engine = simulate_autoplay(&m);
        assert_eq!(engine.counts[0], n as u32, "scratch-lane autoplay is all PGREAT");
        assert_eq!(engine.max_combo, n as u32);
    }

    #[test]
    fn autoplay_emits_bg_and_note_keysounds_time_sorted() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01\r\n");
        let (_engine, events) = autoplay_collect(m);
        assert!(events.len() >= 2, "a BGM and a note both produce keysounds");
        for w in events.windows(2) {
            assert!(w[0].at_us <= w[1].at_us, "keysounds emitted in non-decreasing time order");
        }
    }

    #[test]
    fn autoplay_ln_emits_exactly_one_keysound() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let (_engine, events) = autoplay_collect(m);
        assert_eq!(events.len(), 1, "an LN fires exactly one (head) keysound");
        assert_eq!(events[0].wav, 1, "the head wav is played");
    }

    #[test]
    fn dangling_longstart_emits_no_keysound() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000000\r\n");
        let (engine, events) = autoplay_collect(m);
        assert!(events.is_empty(), "a dangling LongStart must not sound");
        assert_eq!(engine.total_judged(), 0, "and produces no judgment in autoplay");
    }

    #[test]
    fn auto_lane_ignores_input_on_that_lane_but_judges_from_chart() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        assert!(p.press(7, nt, |_| {}).is_none(), "press on an auto lane returns None");
        assert!(p.release(7, nt).is_none(), "release on an auto lane returns None");
        assert_eq!(p.judge().counts[0], 0, "input did not score it early");
        p.update(nt + 1_000_000, |_| {});
        assert_eq!(p.judge().counts[0], 1, "the chart auto-judged it as PGREAT");
    }

    /// Resolving LN MODE in the model rather than in the engine is what keeps the chart's own note
    /// count in step with the one the engine judges against: a charge note is two judged objects, so
    /// a chart left unresolved reports one note where the engine counts two, and `#TOTAL`, the gauge
    /// denominator and the song list all disagree with the run.
    #[test]
    fn resolving_the_long_note_flavour_in_the_model_keeps_the_chart_and_the_engine_in_step() {
        let src = rbms_parser::parse(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let unresolved = to_model(&src, Mode::BEAT_7K);
        assert!(rbms_chart::contains_undefined_long_note(&unresolved), "the chart states no #LNMODE, so the setting decides");

        let mut engine_only = Player::new(unresolved.clone(), false);
        engine_only.set_ln_mode(LnMode::ChargeNote);
        assert_ne!(
            engine_only.judge().total_notes() as usize,
            rbms_chart::count_playable_notes(&unresolved),
            "resolving in the engine alone leaves the chart counting one note where the run judges two"
        );

        let mut resolved = unresolved;
        rbms_chart::resolve_long_note_flavour(&mut resolved, LnMode::ChargeNote.resolve());
        let mut player = Player::new(resolved.clone(), false);
        player.set_ln_mode(LnMode::ChargeNote);
        assert_eq!(player.judge().total_notes() as usize, rbms_chart::count_playable_notes(&resolved));
        assert_eq!(rbms_chart::count_playable_notes(&resolved), 2, "a charge note is judged at its head and at its end");
    }

    #[test]
    fn a_stray_release_on_an_auto_lane_cannot_break_the_long_note_it_is_holding() {
        let chart = b"#BPM 120\r\n#WAV01 a.wav\r\n#00156:01000001\r\n";
        let run = |stray_release: bool| {
            let m = model(chart);
            let times: Vec<i64> = m.timelines.iter().flat_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).collect();
            let (head, end) = (times[0], times[1]);
            let mut p = Player::new(m, false);
            p.set_auto_lanes((0..8).map(|l| l == 7).collect());
            p.update(head, |_| {});
            if stray_release {
                p.release(7, (head + end) / 2);
            }
            p.update(end + 1_000_000, |_| {});
            (p.judge().counts, p.judge().ex_score)
        };
        assert_eq!(run(true), run(false), "an auto-played lane holds its own long note, so a key-up on it is not the player's");
        assert_eq!(run(false).0[0], 1, "the chart took the long note as a PGREAT");
    }

    #[test]
    fn auto_lane_press_does_not_light_beam() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        p.press(7, 500_000, |_| {});
        assert_eq!(p.beam_on()[7], i64::MIN, "ignored auto-lane press leaves the beam off");
    }

    #[test]
    fn non_auto_lane_in_interactive_is_not_auto_judged() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        assert_eq!(p.judge().counts[0], 1, "only the auto lane scored");
        assert_eq!(p.judge.counts[4], 1, "the interactive lane's unpressed note became a 見逃し POOR");
    }

    #[test]
    fn set_auto_lanes_wrong_length_is_a_noop() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[7].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_auto_lanes(vec![true; 3]);
        p.update(nt - 1_000, |_| {});
        assert_eq!(p.judge().counts[0], 0, "no lane became auto, so nothing was auto-judged");
        assert!(p.press(7, nt, |_| {}).is_some(), "lane 7 input is honored when the guard rejected the vector");
    }

    #[test]
    fn set_auto_lanes_exact_length_is_applied() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes(vec![false; 8]);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        assert_eq!(p.judge().counts[0], 1, "exact-length auto vector took effect");
    }

    #[test]
    fn empty_press_lights_beam_but_no_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.press(0, 100_000, |_| {});
        assert_eq!(p.beam_on()[0], 100_000, "even an empty press lights the beam");
        assert_eq!(p.bomb()[0].0, i64::MIN, "but lights no bomb");
    }

    #[test]
    fn bomb_only_fires_for_judge_index_le_3() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut bad = Player::new(m.clone(), false);
        let r = bad.press(0, nt + 250_000, |_| {}).unwrap();
        assert_eq!(r.judge, rbms_judge::Judge::Bad);
        assert_eq!(bad.bomb()[0].0, nt + 250_000, "a BAD (index 3) lights a bomb");
        assert_eq!(bad.bomb()[0].1, 3, "bomb judge index is BAD");
        let mut poor = Player::new(m, false);
        let rp = poor.press(0, nt - 300_000, |_| {}).unwrap();
        assert_eq!(rp.judge, rbms_judge::Judge::Miss, "300ms-early is an empty poor (judge code 5)");
        assert_eq!(poor.bomb()[0].0, i64::MIN, "an empty POOR (index 5) lights no bomb");
    }

    #[test]
    fn release_without_held_ln_still_clears_beam() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.press(0, 500_000, |_| {});
        assert_eq!(p.beam_on()[0], 500_000);
        let r = p.release(0, 600_000);
        assert!(r.is_none(), "no held LN -> release judges nothing");
        assert_eq!(p.beam_on()[0], i64::MIN, "beam cleared on release");
        assert_eq!(p.beam_off()[0], 600_000, "release fade timestamp set");
    }

    #[test]
    fn release_when_beam_already_off_sets_no_fade() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        p.release(0, 700_000);
        assert_eq!(p.beam_on()[0], i64::MIN);
        assert_eq!(p.beam_off()[0], i64::MIN, "release with no prior press leaves no fade timestamp");
    }

    #[test]
    fn interactive_ln_press_lights_beam_until_release() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
        let (head, end) = (t[0], t[1]);
        let mut p = Player::new(m, false);
        p.press(0, head, |_| {});
        assert_eq!(p.beam_on()[0], head, "LN head lights the beam");
        p.release(0, end);
        assert_eq!(p.beam_on()[0], i64::MIN, "release clears the beam");
        assert_eq!(p.beam_off()[0], end, "release fade timestamp set at the end");
    }

    #[test]
    fn ln_head_press_lights_bomb_for_pgreat() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let head = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).next().unwrap();
        let mut p = Player::new(m, false);
        p.press(0, head, |_| {});
        assert_eq!(p.bomb()[0].0, head, "LN head hit lights a bomb");
        assert_eq!(p.bomb()[0].1, 0, "PGREAT head -> bomb index 0");
    }

    #[test]
    fn press_plays_nearest_head_wav_in_lane() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00111:01000002\r\n");
        let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
        let (first, second) = (t[0], t[1]);
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        p.press(0, second - 5_000, |e| events.push(e));
        assert_eq!(events.len(), 1, "a press emits exactly one keysound");
        assert_eq!(events[0].wav, 2, "the nearest head's wav (note 2) is chosen");
        let mut p2 = {
            let m2 = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00111:01000002\r\n");
            Player::new(m2, false)
        };
        let mut ev2 = Vec::new();
        p2.press(0, first + 5_000, |e| ev2.push(e));
        assert_eq!(ev2[0].wav, 1, "nearer the first head -> wav 1");
    }

    #[test]
    fn nearest_head_wav_is_lane_scoped() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00112:01\r\n");
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        p.press(0, 2_000_000, |e| events.push(e));
        assert!(events.is_empty(), "pressing an empty lane emits no keysound");
    }

    #[test]
    fn nearest_head_wav_uses_ln_head_for_lookup() {
        let m = model(b"#BPM 120\r\n#WAV07 g.wav\r\n#00151:07000007\r\n");
        let head = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).next().unwrap();
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        p.press(0, head, |e| events.push(e));
        assert_eq!(events[0].wav, 7, "LN head wav (07) is selectable via nearest_head_wav");
    }

    #[test]
    fn judge_rate_zero_clamps_to_min_not_panic() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_judge_rate(0);
        assert_eq!(p.press(0, nt, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "on-time press is PGREAT at clamped min width");
    }

    #[test]
    fn judge_rate_widening_reaches_pgreat_edge() {
        let m = model(b"#BPM 120\r\n#RANK 0\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut narrow = Player::new(m.clone(), false);
        assert_eq!(narrow.press(0, nt - 14_000, |_| {}).unwrap().judge, rbms_judge::Judge::Great, "14ms off is GREAT at #RANK 0 (PG ±5ms)");
        let mut wide = Player::new(m, false);
        wide.set_judge_rate(400);
        assert_eq!(wide.press(0, nt - 14_000, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "400% width reaches the PGREAT edge");
    }

    #[test]
    fn judge_width_cannot_reach_past_the_rank_bad_edge() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut narrow = Player::new(m.clone(), false);
        assert!(narrow.press(0, nt + 300_000, |_| {}).is_none(), "300ms late at 100% is beyond every window");
        let mut wide = Player::new(m, false);
        wide.set_judge_rate(200);
        assert!(wide.press(0, nt + 300_000, |_| {}).is_none(), "200% width still cannot reach past the BAD edge");
    }

    #[test]
    fn judge_rate_does_not_widen_fixed_ms_window() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_judge_rate(200);
        assert!(p.press(0, nt - 600_000, |_| {}).is_none(), "the fixed MS early edge (+500ms) does not widen with JUDGE WIDTH");
    }

    #[test]
    fn unpressed_note_swept_to_miss_breaks_combo() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.update(nt + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[4], 1, "unpressed note swept to 見逃し POOR (reference judge code 4)");
        assert_eq!(p.judge.counts[5], 0, "nothing lands in the 空POOR slot");
        assert_eq!(p.judge.max_combo, 0, "a swept POOR keeps combo at zero");
    }

    #[test]
    fn held_ln_never_released_finalises_via_update_sweep() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n");
        let t: Vec<i64> = m.timelines.iter().flat_map(|tl| tl.notes[0].as_ref()).map(|n| n.time_us).collect();
        let (head, end) = (t[0], t[1]);
        let mut p = Player::new(m, false);
        assert_eq!(p.press(0, head, |_| {}).unwrap().judge, rbms_judge::Judge::PerfectGreat, "LN head PGREAT");
        p.update(end - 1, |_| {});
        assert_eq!(p.judge.total_judged(), 0, "before the end the LN is still held");
        p.update(end + 1_000_000, |_| {});
        assert_eq!(p.judge.total_judged(), 1, "the over-held LN is finalized exactly once");
        assert_eq!(p.judge().counts[0], 1, "finalized with the head PGREAT (lnstartJudge)");
    }

    #[test]
    fn update_is_idempotent_after_full_judge() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n");
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        p.update(end, |_| {});
        let snapshot = p.judge.counts;
        let ex = p.judge.ex_score;
        p.update(end + 5_000_000, |_| {});
        assert_eq!(p.judge.counts, snapshot, "no extra judgments on a second sweep");
        assert_eq!(p.judge.ex_score, ex);
    }

    #[test]
    fn update_monotonic_combo_progress_in_autoplay() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:0101010101010101\r\n");
        let n = rbms_chart::count_playable_notes(&m) as u32;
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        let mut prev = 0u32;
        let mut t = 0i64;
        while t <= end {
            p.update(t, |_| {});
            assert!(p.judge.combo >= prev, "autoplay combo never drops");
            prev = p.judge.combo;
            t += 100_000;
        }
        assert_eq!(p.judge.combo, n, "autoplay ends on a full combo");
        assert_eq!(p.judge.max_combo, n);
    }

    #[test]
    fn last_time_us_is_final_timeline_time() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00311:01\r\n");
        let expected = m.timelines.last().map(|t| t.time_us).unwrap();
        let p = Player::new(m, true);
        assert_eq!(p.last_time_us(), expected, "last_time_us mirrors the final timeline's time");
    }

    #[test]
    fn new_player_starts_with_clean_beam_bomb_state() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let p = Player::new(m, false);
        assert_eq!(p.beam_on().len(), 8, "one beam slot per lane (BEAT_7K key=8)");
        assert!(p.beam_on().iter().all(|&v| v == i64::MIN), "all beams initially off");
        assert!(p.beam_off().iter().all(|&v| v == i64::MIN), "no fades pending");
        assert!(p.bomb().iter().all(|&(t, j)| t == i64::MIN && j == 0), "no bombs initially");
    }

    #[test]
    fn press_out_of_range_lane_returns_none_and_no_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        assert!(p.press(99, 1_000_000, |_| {}).is_none(), "press on a non-existent lane judges nothing");
    }

    #[test]
    fn swept_miss_does_not_light_bomb() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let mut p = Player::new(m, false);
        let nt = p.last_time_us();
        p.update(nt + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[4], 1, "the note was swept to 見逃し POOR");
        assert_eq!(p.bomb()[0].0, i64::MIN, "a swept POOR lights no bomb");
    }

    const LOOKAHEAD_SKEW_US: i64 = 10_000;
    const AXIS_STEP_US: i64 = 1_000;

    #[test]
    fn update_schedule_emits_sounds_without_touching_judgment() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00101:01\r\n#00111:01\r\n");
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        let mut events = Vec::new();
        p.update_schedule(end, |e| events.push(e));
        assert_eq!(events.len(), 2, "the BGM channel and the autoplay note were both scheduled");
        assert_eq!(p.judge.total_judged(), 0, "update_schedule judges nothing");
        assert!(p.beam_on().iter().all(|&v| v == i64::MIN), "update_schedule lights no beam");
        assert!(p.beam_off().iter().all(|&v| v == i64::MIN), "update_schedule leaves no beam fade");
        assert!(p.bomb().iter().all(|&(t, _)| t == i64::MIN), "update_schedule lights no bomb");
    }

    #[test]
    fn update_judge_advances_judgment_and_leaves_the_schedule_cursors_alone() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00101:01\r\n#00111:01\r\n");
        let n = rbms_chart::count_playable_notes(&m);
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        p.update_judge(end);
        assert_eq!(p.judge().counts[0], n as u32, "the judge axis alone judges the autoplay note");
        assert_ne!(p.beam_off()[0], i64::MIN, "the judge axis alone runs the auto beam timer");
        let mut events = Vec::new();
        p.update_schedule(end, |e| events.push(e));
        assert_eq!(events.len(), 2, "update_judge emitted no sound, so every keysound was still pending");
    }

    #[test]
    fn judge_cursor_never_passes_the_schedule_cursor() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00101:01\r\n#00151:01000001\r\n#00112:01010101\r\n");
        let mut p = Player::new(m, true);
        let end = p.last_time_us() + 1_000_000;
        let mut t = 0i64;
        while t <= end {
            p.update_schedule(t + LOOKAHEAD_SKEW_US, |_| {});
            p.update_judge(t);
            assert!(p.judge_cursor <= p.action_cursor, "judge_cursor trails action_cursor at t={t}");
            t += AXIS_STEP_US;
        }
        assert_eq!(p.action_cursor, p.actions.len(), "the schedule cursor consumed every action");
        assert_eq!(p.judge_cursor, p.action_cursor, "the judge cursor caught up once the song ended");
    }

    #[test]
    fn judge_axis_is_independent_of_how_far_the_schedule_axis_ran() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n");
        let mut split = Player::new(m.clone(), false);
        let mut single = Player::new(m.clone(), false);
        let mut leaked = Player::new(m, false);
        let end = single.last_time_us() + 1_000_000;
        let mut leak_diverged = false;
        let mut t = 0i64;
        while t <= end {
            split.update_schedule(t + LOOKAHEAD_SKEW_US, |_| {});
            split.update_judge(t);
            single.update(t, |_| {});
            leaked.update(t + LOOKAHEAD_SKEW_US, |_| {});
            assert_eq!(split.judge.counts, single.judge.counts, "the miss sweep follows the judge axis at t={t}");
            assert_eq!(split.judge.combo, single.judge.combo, "combo follows the judge axis at t={t}");
            leak_diverged |= leaked.judge.counts != single.judge.counts;
            t += AXIS_STEP_US;
        }
        assert!(leak_diverged, "shifting the judge axis by the skew does change the sweep, so the equality above is not vacuous");
    }

    #[test]
    fn autoplay_beams_and_bombs_follow_the_judge_axis_not_the_schedule_axis() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00151:01000001\r\n#00112:01010101\r\n");
        let mut split = Player::new(m.clone(), true);
        let mut single = Player::new(m.clone(), true);
        let mut leaked = Player::new(m, true);
        let end = single.last_time_us() + 1_000_000;
        let mut leak_diverged = false;
        let mut t = 0i64;
        while t <= end {
            split.update_schedule(t + LOOKAHEAD_SKEW_US, |_| {});
            split.update_judge(t);
            single.update(t, |_| {});
            leaked.update(t + LOOKAHEAD_SKEW_US, |_| {});
            assert_eq!(split.beam_on(), single.beam_on(), "auto beam-on follows the judge axis at t={t}");
            assert_eq!(split.beam_off(), single.beam_off(), "auto beam-off follows the judge axis at t={t}");
            assert_eq!(split.bomb(), single.bomb(), "auto bombs follow the judge axis at t={t}");
            assert_eq!(split.judge.counts, single.judge.counts, "auto judgments follow the judge axis at t={t}");
            leak_diverged |= leaked.beam_on() != single.beam_on();
            t += AXIS_STEP_US;
        }
        assert!(leak_diverged, "shifting the judge axis by the skew does move the beams, so the equalities above are not vacuous");
    }

    #[test]
    fn schedule_ahead_does_not_light_beams_or_sweep_misses() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();

        let mut auto = Player::new(m.clone(), true);
        auto.update_schedule(nt + 1_000_000, |_| {});
        assert_eq!(auto.beam_on()[0], i64::MIN, "the reserved autoplay press has not lit its beam yet");
        assert_eq!(auto.judge.total_judged(), 0, "and has not been judged yet");
        auto.update_judge(nt);
        assert_eq!(auto.beam_on()[0], nt, "the judge axis lights the beam at the charted note time");
        assert_eq!(auto.judge.counts[0], 1, "and judges it PGREAT");

        let mut manual = Player::new(m, false);
        manual.update_schedule(nt + 1_000_000, |_| {});
        assert_eq!(manual.judge.counts[4], 0, "the schedule axis never sweeps an unpressed note to POOR");
        manual.update_judge(nt + 1_000_000);
        assert_eq!(manual.judge.counts[4], 1, "the judge axis does");
    }

    #[test]
    fn play_events_carry_their_output_bus() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01\r\n");
        let (_engine, events) = autoplay_collect(m);
        let bgm: Vec<i32> = events.iter().filter(|e| e.source == PlaySource::Bgm).map(|e| e.wav).collect();
        let key: Vec<i32> = events.iter().filter(|e| e.source == PlaySource::Key).map(|e| e.wav).collect();
        assert_eq!(bgm, vec![2], "the BGM channel keysound is tagged Bgm");
        assert_eq!(key, vec![1], "the autoplay note keysound is tagged Key");
    }

    #[test]
    fn press_keysound_is_tagged_key() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let head = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        let mut events = Vec::new();
        p.press(0, head, |e| events.push(e));
        assert_eq!(events.len(), 1, "an interactive hit emits one keysound");
        assert_eq!(events[0].source, PlaySource::Key, "an interactive hit sound goes to the key bus");
    }

    #[test]
    fn update_is_exactly_schedule_then_judge_at_one_clock() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00101:01\r\n#00151:01000001\r\n#00112:01010101\r\n");
        let mut composed = Player::new(m.clone(), true);
        let mut manual = Player::new(m, true);
        let end = composed.last_time_us() + 1_000_000;
        let mut composed_events = Vec::new();
        let mut manual_events = Vec::new();
        let mut t = 0i64;
        while t <= end {
            composed.update(t, |e| composed_events.push(e));
            manual.update_schedule(t, |e| manual_events.push(e));
            manual.update_judge(t);
            t += AXIS_STEP_US;
        }
        assert_eq!(composed_events, manual_events, "the backward-compatible update emits the same events in the same order");
        assert_eq!(composed.judge.counts, manual.judge.counts, "and reaches the same judgment state");
        assert_eq!(composed.judge.ex_score, manual.judge.ex_score);
        assert_eq!(composed.beam_on(), manual.beam_on());
        assert_eq!(composed.beam_off(), manual.beam_off());
        assert_eq!(composed.bomb(), manual.bomb());
    }
}

#[cfg(test)]
mod judge_wiring_tests {
    use super::*;
    use rbms_chart::{count_playable_notes, to_model, to_model_with_ln_mode};
    use rbms_judge::JudgeProperty;
    use rbms_judge::gauge::GaugeIndex;
    use rbms_model::{LnKind, Mode};
    use rbms_parser::parse;

    /// One note on lane 0 at 2.0 s, with nothing after it, so a single update past the BAD window
    /// sweeps it to a 見逃し POOR.
    const ONE_NOTE: &[u8] = b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n";

    /// The same note on a chart that states no `#LNMODE`, as a long note.
    const ONE_UNSTATED_LONG_NOTE: &[u8] = b"#BPM 120\r\n#WAV01 a.wav\r\n#00151:01000001\r\n";

    /// Far enough past the note that the miss sweep has run.
    const SWEEP_US: i64 = 4_000_000;

    fn player(bms: &[u8], mode: Mode) -> Player {
        Player::new(to_model(&parse(bms), mode), false)
    }

    #[test]
    fn the_keyboard_mode_reaches_the_keyboard_judge_row() {
        let prop = JudgeProperty::for_mode(&Mode::KEYBOARD_24K);
        assert_eq!(prop.note, JudgeProperty::KEYBOARD.note);
        assert_eq!(prop.scratch, JudgeProperty::KEYBOARD.scratch);
        assert_eq!(prop.ln_end, JudgeProperty::KEYBOARD.ln_end);
        assert_eq!(prop.ln_scratch_end, JudgeProperty::KEYBOARD.ln_scratch_end);
    }

    #[test]
    fn a_keyboard_chart_plays_on_the_keyboard_gauge_table() {
        let p = player(ONE_NOTE, Mode::KEYBOARD_24K);
        assert_eq!(p.gauge_set(), GaugeSetId::Keyboard);
        assert_eq!(p.judge_window_rule(), JudgeWindowRule::Normal);
    }

    #[test]
    fn every_mode_takes_the_gauge_table_the_reference_gives_it() {
        for (mode, set) in [
            (Mode::BEAT_5K, GaugeSetId::FiveKeys),
            (Mode::BEAT_10K, GaugeSetId::FiveKeys),
            (Mode::BEAT_7K, GaugeSetId::SevenKeys),
            (Mode::BEAT_14K, GaugeSetId::SevenKeys),
            (Mode::POPN_9K, GaugeSetId::Pms),
            (Mode::KEYBOARD_24K, GaugeSetId::Keyboard),
        ] {
            assert_eq!(player(ONE_NOTE, mode).gauge_set(), set, "{}", mode.name);
        }
    }

    #[test]
    fn the_lr2_gauge_table_is_reached_only_by_asking_for_it() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        assert_eq!(p.gauge_set(), GaugeSetId::SevenKeys);
        p.set_gauge_set(GaugeSetId::Lr2);
        assert_eq!(p.gauge_set(), GaugeSetId::Lr2);
    }

    #[test]
    fn a_popn_chart_is_judged_under_the_pms_judgerank_rule() {
        assert_eq!(player(ONE_NOTE, Mode::POPN_9K).judge_window_rule(), JudgeWindowRule::Pms);
        for mode in [Mode::BEAT_5K, Mode::BEAT_7K, Mode::BEAT_10K, Mode::BEAT_14K, Mode::KEYBOARD_24K] {
            assert_eq!(player(ONE_NOTE, mode).judge_window_rule(), JudgeWindowRule::Normal, "{}", mode.name);
        }
    }

    #[test]
    fn the_pms_rule_holds_pgreat_open_where_the_beat_rule_narrows_it() {
        let hardest = b"#BPM 120\r\n#RANK 0\r\n#WAV01 a.wav\r\n#00111:01\r\n";
        let late_us = 2_000_000 + 10_000;

        let mut popn = Player::new(to_model(&parse(hardest), Mode::POPN_9K), false);
        popn.press(0, late_us, |_| {});
        assert_eq!(popn.judge().counts[0], 1, "PMS fixes PGREAT at its tabulated width whatever the judgerank");

        let mut beat = Player::new(to_model(&parse(hardest), Mode::BEAT_7K), false);
        beat.press(0, late_us, |_| {});
        assert_eq!(beat.judge().counts[0], 0, "the beat rule scales PGREAT down at #RANK 0");
        assert_eq!(beat.judge().counts[1], 1);
    }

    #[test]
    fn judge_width_still_widens_the_windows_under_the_mode_rule() {
        let outside_us = 2_000_000 + 25_000;
        let mut stock = player(ONE_NOTE, Mode::BEAT_7K);
        stock.press(0, outside_us, |_| {});
        assert_eq!(stock.judge().counts[0], 0, "25 ms late is outside the stock 20 ms PGREAT window");

        let mut widened = player(ONE_NOTE, Mode::BEAT_7K);
        widened.set_judge_window_rates([200, 100, 100], UNMODIFIED_JUDGE_RATES);
        widened.press(0, outside_us, |_| {});
        assert_eq!(widened.judge().counts[0], 1, "a 200 % key PGREAT rate reaches 40 ms");
    }

    #[test]
    fn the_single_rate_slider_sets_every_tier_of_both_lane_kinds() {
        let outside_us = 2_000_000 + 25_000;
        let mut one_slider = player(ONE_NOTE, Mode::BEAT_7K);
        one_slider.set_judge_rate(200);
        one_slider.press(0, outside_us, |_| {});

        let mut six_values = player(ONE_NOTE, Mode::BEAT_7K);
        six_values.set_judge_window_rates([200; JUDGE_WIDTH_TIER_COUNT], [200; JUDGE_WIDTH_TIER_COUNT]);
        six_values.press(0, outside_us, |_| {});

        assert_eq!(one_slider.judge().counts, six_values.judge().counts);
        assert_eq!(one_slider.judge().counts[0], 1);
    }

    #[test]
    fn a_keyboard_chart_drives_all_twenty_six_lanes() {
        let p = player(ONE_NOTE, Mode::KEYBOARD_24K);
        assert_eq!(p.beam_on().len(), Mode::KEYBOARD_24K.key);
        assert_eq!(p.bomb().len(), Mode::KEYBOARD_24K.key);
    }

    #[test]
    fn a_fresh_player_starts_on_the_stock_widths() {
        let p = player(ONE_NOTE, Mode::BEAT_7K);
        assert_eq!(p.algorithm(), rbms_judge::algorithm::JudgeAlgorithm::default());
        assert_eq!(p.gauge_auto_shift(), GaugeAutoShift::None);
        assert_eq!(p.bottom_shiftable_gauge(), GaugeKind::AssistEasy);
        assert_eq!(p.configured_gauge(), GaugeKind::Normal);
        assert!(!p.failed());
    }

    #[test]
    fn the_configured_gauge_is_remembered_for_auto_shift() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Hard);
        assert_eq!(p.configured_gauge(), GaugeKind::Hard);
        assert_eq!(p.judge().gauge.selected_index(), GaugeIndex::Hard);
    }

    #[test]
    fn the_auto_shift_floor_is_clamped_to_the_reference_range() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        for (asked, clamped) in [
            (GaugeKind::AssistEasy, GaugeKind::AssistEasy),
            (GaugeKind::Easy, GaugeKind::Easy),
            (GaugeKind::Normal, GaugeKind::Normal),
            (GaugeKind::Hard, GaugeKind::Normal),
            (GaugeKind::ExHard, GaugeKind::Normal),
            (GaugeKind::Hazard, GaugeKind::Normal),
        ] {
            p.set_bottom_shiftable_gauge(asked);
            assert_eq!(p.bottom_shiftable_gauge(), clamped, "{asked:?}");
        }
    }

    #[test]
    fn gauge_auto_shift_none_fails_the_run_when_the_gauge_empties() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Hazard);
        p.update(SWEEP_US, |_| {});
        assert_eq!(p.judge().counts[4], 1, "the unhit note is swept to a POOR");
        assert_eq!(p.judge().gauge.value(), 0.0, "one POOR empties the hazard gauge");
        assert!(p.failed(), "NONE ends the play the moment the gauge empties");
    }

    #[test]
    fn gauge_auto_shift_continue_keeps_playing_on_an_empty_gauge() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Hazard);
        p.set_gauge_auto_shift(GaugeAutoShift::Continue);
        p.update(SWEEP_US, |_| {});
        assert_eq!(p.judge().gauge.value(), 0.0);
        assert!(!p.failed());
    }

    #[test]
    fn gauge_auto_shift_survival_to_groove_drops_to_normal_instead_of_failing() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Hazard);
        p.set_gauge_auto_shift(GaugeAutoShift::SurvivalToGroove);
        p.update(SWEEP_US, |_| {});
        assert!(!p.failed());
        assert_eq!(p.judge().gauge.selected_index(), GaugeIndex::Normal);
    }

    #[test]
    fn gauge_auto_shift_does_not_move_the_selection_while_the_gauge_holds() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Normal);
        p.set_gauge_auto_shift(GaugeAutoShift::SurvivalToGroove);
        p.update(SWEEP_US, |_| {});
        assert!(p.judge().gauge.value() > 0.0);
        assert_eq!(p.judge().gauge.selected_index(), GaugeIndex::Normal);
        assert!(!p.failed());
    }

    #[test]
    fn gauge_auto_shift_best_clear_re_picks_every_frame() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::AssistEasy);
        p.set_gauge_auto_shift(GaugeAutoShift::BestClear);
        assert_eq!(p.judge().gauge.selected_index(), GaugeIndex::AssistEasy, "no shift before the first frame");
        p.update(0, |_| {});
        assert_eq!(p.judge().gauge.selected_index(), GaugeIndex::Hazard, "BEST CLEAR takes the strongest gauge still clearing");
        assert!(!p.failed());
    }

    #[test]
    fn gauge_auto_shift_select_to_under_never_rises_above_the_chosen_gauge() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Easy);
        p.set_gauge_auto_shift(GaugeAutoShift::SelectToUnder);
        p.update(0, |_| {});
        let selected = p.judge().gauge.selected_index();
        assert!(selected.index() <= GaugeIndex::Easy.index(), "the chosen gauge is the ceiling, got {selected:?}");
        assert_eq!(selected, GaugeIndex::AssistEasy, "with no gauge above the floor still clearing, the scan leaves the floor selected");
    }

    #[test]
    fn gauge_auto_shift_select_to_under_holds_the_floor_it_is_given() {
        let mut p = player(ONE_NOTE, Mode::BEAT_7K);
        p.set_gauge(GaugeKind::Normal);
        p.set_gauge_auto_shift(GaugeAutoShift::SelectToUnder);
        p.set_bottom_shiftable_gauge(GaugeKind::Normal);
        p.update(0, |_| {});
        assert_eq!(p.judge().gauge.selected_index(), GaugeIndex::Normal, "the floor stops the drop");
    }

    #[test]
    fn an_unstated_long_note_takes_the_players_ln_mode() {
        let src = parse(ONE_UNSTATED_LONG_NOTE);
        let model = to_model(&src, Mode::BEAT_7K);
        assert_eq!(count_playable_notes(&model), 1);

        let mut plain = Player::new(model.clone(), true);
        assert_eq!(plain.ln_mode(), LnMode::LongNote);
        assert_eq!(plain.judge().total_notes(), 1, "a plain long note is judged once");

        let mut charge = Player::new(model, true);
        charge.set_ln_mode(LnMode::ChargeNote);
        assert_eq!(charge.ln_mode(), LnMode::ChargeNote);
        assert_eq!(charge.judge().total_notes(), 2, "a charge note is judged at both ends");

        let end = plain.last_time_us() + 1_000_000;
        plain.update(end, |_| {});
        charge.update(end, |_| {});
        assert_eq!(plain.judge().counts[0], 1);
        assert_eq!(charge.judge().counts[0], 2);
    }

    #[test]
    fn a_chart_that_states_its_long_note_type_ignores_the_players_ln_mode() {
        let model = to_model(&parse(b"#BPM 120\r\n#LNMODE 1\r\n#WAV01 a.wav\r\n#00151:01000001\r\n"), Mode::BEAT_7K);
        let mut p = Player::new(model, true);
        p.set_ln_mode(LnMode::HellChargeNote);
        assert_eq!(p.judge().total_notes(), 1, "#LNMODE 1 keeps the note a plain long note");
    }

    #[test]
    fn resolving_the_ln_mode_in_the_chart_matches_resolving_it_in_the_engine() {
        let src = parse(ONE_UNSTATED_LONG_NOTE);
        let in_chart = Player::new(to_model_with_ln_mode(&src, Mode::BEAT_7K, LnKind::Cn), true);
        let mut in_engine = Player::new(to_model(&src, Mode::BEAT_7K), true);
        in_engine.set_ln_mode(LnMode::ChargeNote);
        assert_eq!(in_chart.judge().total_notes(), in_engine.judge().total_notes());
    }
}
