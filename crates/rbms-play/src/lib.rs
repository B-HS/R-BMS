use rbms_judge::{GaugeKind, JudgeEngine, JudgeResult, JudgeWindows, rank_to_judgerank};
use rbms_model::{Model, NoteKind};

/// A keysound that should fire at `at_us` (autoplay BGM, or a hit note's sound).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayEvent {
    pub wav: i32,
    pub at_us: i64,
}

enum AutoAction {
    Press { wav: i32, ln: bool },
    Release,
}

/// Autoplay key-beam flash for a tapped (non-LN) note, mirroring beatoraja's
/// `auto_minduration` (80 ms): a tap lights its lane beam for this long, then releases.
const AUTO_BEAM_US: i64 = 80_000;

fn windows_for(model: &Model) -> JudgeWindows {
    JudgeWindows::SEVENKEY_NOTE.scaled(rank_to_judgerank(model.meta.rank))
}

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
            let Some(note) = &tl.notes[lane] else { continue };
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
    pub judge: JudgeEngine,
    bg: Vec<(i64, i32)>,
    bg_cursor: usize,
    heads: Vec<(i64, usize, i32)>,
    actions: Vec<(i64, usize, AutoAction)>,
    action_cursor: usize,
    autoplay: bool,
    auto_lanes: Vec<bool>,
    beam_on: Vec<i64>,
    beam_off: Vec<i64>,
    ln_active: Vec<bool>,
    bomb: Vec<(i64, u8)>,
}

impl Player {
    pub fn new(model: Model, autoplay: bool) -> Self {
        let judge = JudgeEngine::from_model(&model, windows_for(&model));
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
            autoplay,
            auto_lanes,
            beam_on: vec![i64::MIN; lanes],
            beam_off: vec![i64::MIN; lanes],
            ln_active: vec![false; lanes],
            bomb: vec![(i64::MIN, 0); lanes],
        }
    }

    /// Apply a user judge-width multiplier (percent; 100 = chart default). The effective
    /// window is the chart's rank judgerank scaled by this rate — wider = more lenient.
    pub fn set_judge_rate(&mut self, rate_percent: i32) {
        let base = rank_to_judgerank(self.model.meta.rank);
        let eff = (base * rate_percent.max(1) / 100).max(1);
        self.judge.set_windows(JudgeWindows::SEVENKEY_NOTE.scaled(eff));
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

    pub fn set_gauge(&mut self, kind: GaugeKind) {
        self.judge.set_gauge(kind, self.model.meta.total);
    }

    pub fn into_judge(self) -> JudgeEngine {
        self.judge
    }

    pub fn update<F: FnMut(PlayEvent)>(&mut self, now_us: i64, mut play: F) {
        while self.bg_cursor < self.bg.len() && self.bg[self.bg_cursor].0 <= now_us {
            let (at, wav) = self.bg[self.bg_cursor];
            play(PlayEvent { wav, at_us: at });
            self.bg_cursor += 1;
        }
        while self.action_cursor < self.actions.len() && self.actions[self.action_cursor].0 <= now_us {
            let (at, lane, ref kind) = self.actions[self.action_cursor];
            if self.autoplay || self.auto_lanes.get(lane).copied().unwrap_or(false) {
                match kind {
                    AutoAction::Press { wav, ln } => {
                        play(PlayEvent { wav: *wav, at_us: at });
                        if let Some(jr) = self.judge.press(lane, at) {
                            if (jr.judge as usize) <= 3 {
                                self.bomb[lane] = (at, jr.judge as u8);
                            }
                        }
                        self.beam_on[lane] = at;
                        self.beam_off[lane] = i64::MIN;
                        if *ln {
                            self.ln_active[lane] = true;
                        }
                    }
                    AutoAction::Release => {
                        if let Some(jr) = self.judge.release(lane, at) {
                            if (jr.judge as usize) <= 3 {
                                self.bomb[lane] = (at, jr.judge as u8);
                            }
                        }
                        self.beam_off[lane] = at;
                        self.beam_on[lane] = i64::MIN;
                        self.ln_active[lane] = false;
                    }
                }
            }
            self.action_cursor += 1;
        }
        for lane in 0..self.beam_on.len() {
            let auto = self.autoplay || self.auto_lanes[lane];
            if auto && self.beam_on[lane] != i64::MIN && !self.ln_active[lane] && now_us - self.beam_on[lane] >= AUTO_BEAM_US {
                self.beam_off[lane] = self.beam_on[lane] + AUTO_BEAM_US;
                self.beam_on[lane] = i64::MIN;
            }
        }
        self.judge.update(now_us);
    }

    pub fn press<F: FnMut(PlayEvent)>(&mut self, lane: usize, now_us: i64, mut play: F) -> Option<JudgeResult> {
        if self.auto_lanes.get(lane).copied().unwrap_or(false) {
            return None;
        }
        if lane < self.beam_on.len() {
            self.beam_on[lane] = now_us;
            self.beam_off[lane] = i64::MIN;
        }
        if let Some(wav) = self.nearest_head_wav(lane, now_us) {
            play(PlayEvent { wav, at_us: now_us });
        }
        let res = self.judge.press(lane, now_us);
        if let Some(jr) = &res {
            if (jr.judge as usize) <= 3 && jr.lane < self.bomb.len() {
                self.bomb[jr.lane] = (now_us, jr.judge as u8);
            }
        }
        res
    }

    pub fn release(&mut self, lane: usize, now_us: i64) -> Option<JudgeResult> {
        if lane < self.beam_on.len() && !self.auto_lanes[lane] {
            if self.beam_on[lane] != i64::MIN {
                self.beam_off[lane] = now_us;
            }
            self.beam_on[lane] = i64::MIN;
            self.ln_active[lane] = false;
        }
        let res = self.judge.release(lane, now_us);
        if let Some(jr) = &res {
            if (jr.judge as usize) <= 3 && jr.lane < self.bomb.len() {
                self.bomb[jr.lane] = (now_us, jr.judge as u8);
            }
        }
        res
    }

    fn nearest_head_wav(&self, lane: usize, now_us: i64) -> Option<i32> {
        self.heads
            .iter()
            .filter(|(_, l, _)| *l == lane)
            .min_by_key(|(t, _, _)| (t - now_us).abs())
            .map(|(_, _, wav)| *wav)
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
        assert_eq!(p.judge.counts[0], n as u32);
    }

    #[test]
    fn auto_lane_judged_without_input() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00116:01\r\n");
        let mut p = Player::new(m, false);
        p.set_auto_lanes((0..8).map(|l| l == 7).collect());
        let last = p.last_time_us();
        p.update(last + 1_000_000, |_| {});
        assert_eq!(p.judge.counts[0], 1, "scratch lane auto-judged as PGREAT");
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
    fn widened_late_bad_is_reachable_not_dead() {
        // #RANK 2 (75%) at 200% -> eff 150% -> BAD late edge = -280ms*1.5 = -420ms.
        // A press ~350ms late must be a BAD (not silently dropped by a fixed candidate gate).
        let m = model(b"#BPM 120\r\n#RANK 2\r\n#WAV01 a.wav\r\n#00111:01\r\n");
        let nt = m.timelines.iter().find_map(|t| t.notes[0].as_ref()).map(|n| n.time_us).unwrap();
        let mut p = Player::new(m, false);
        p.set_judge_rate(200);
        assert_eq!(p.press(0, nt + 350_000, |_| {}).unwrap().judge, rbms_judge::Judge::Bad, "350ms-late press is reachable as BAD at widened width");
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
        assert_eq!(p.judge.counts[0], 1);
    }
}
